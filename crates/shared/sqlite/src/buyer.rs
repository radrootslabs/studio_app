use std::collections::BTreeSet;

use radroots_studio_app_models::{
    BuyerCartLineProjection, BuyerCartProjection, BuyerCheckoutDraft, BuyerCheckoutProjection,
    BuyerCheckoutSummaryProjection, BuyerContext, BuyerListingRow, BuyerListingsProjection,
    BuyerOrderDetailProjection, BuyerOrderStatus, BuyerOrdersListRow, BuyerOrdersProjection,
    BuyerProductDetailProjection, FarmId, FarmOrderMethod, FulfillmentWindowId, OrderDetailItemRow,
    OrderId, OrderStatus, ProductAvailabilityState, ProductAvailabilitySummary, ProductId,
    ProductPricePresentation, ProductStatus, ProductStockState, ProductStockSummary,
};
use rusqlite::{Connection, OptionalExtension, params};

use crate::AppSqliteError;

const BUYER_LOW_STOCK_THRESHOLD: u32 = 3;

pub struct AppBuyerRepository<'a> {
    connection: &'a Connection,
}

impl<'a> AppBuyerRepository<'a> {
    pub const fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub fn load_buyer_listings(
        &self,
        search_query: &str,
        fulfillment_methods: &BTreeSet<FarmOrderMethod>,
    ) -> Result<BuyerListingsProjection, AppSqliteError> {
        let now_utc = self.current_utc_timestamp()?;
        let normalized_search = normalize_search_query(search_query);
        let mut records = self.load_listing_records()?;

        records.retain(|record| {
            record.is_buyer_visible(&now_utc)
                && record.matches_search(normalized_search.as_deref())
                && record.matches_fulfillment_methods(fulfillment_methods)
        });
        sort_listing_records(&mut records, &now_utc);

        Ok(BuyerListingsProjection {
            rows: records
                .into_iter()
                .map(|record| record.into_listing_row(&now_utc))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub fn load_buyer_product_detail(
        &self,
        product_id: ProductId,
    ) -> Result<Option<BuyerProductDetailProjection>, AppSqliteError> {
        let now_utc = self.current_utc_timestamp()?;

        self.load_listing_record_by_id(product_id)?
            .filter(|record| record.is_buyer_visible(&now_utc))
            .map(|record| {
                Ok(BuyerProductDetailProjection {
                    detail_text: record.detail_text(),
                    listing: record.into_listing_row(&now_utc)?,
                    selected_quantity: 1,
                })
            })
            .transpose()
    }

    pub fn load_buyer_cart(
        &self,
        context: &BuyerContext,
    ) -> Result<BuyerCartProjection, AppSqliteError> {
        let context_key = context.storage_key();
        let header = self.load_cart_header(&context_key)?;
        let line_records = self.load_cart_line_records(&context_key)?;

        self.build_cart_projection(header, line_records)
    }

    pub fn replace_buyer_cart(
        &self,
        context: &BuyerContext,
        cart: &BuyerCartProjection,
    ) -> Result<(), AppSqliteError> {
        validate_cart_projection(cart)?;
        let context_key = context.storage_key();
        let farm_id = if cart.lines.is_empty() {
            None
        } else {
            cart.farm_id
        };

        self.connection
            .execute(
                "insert into buyer_carts (
                    buyer_context_key,
                    farm_id,
                    updated_at
                 ) values (?1, ?2, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                 on conflict(buyer_context_key) do update set
                    farm_id = excluded.farm_id,
                    updated_at = excluded.updated_at",
                params![context_key.as_str(), farm_id.map(|id| id.to_string())],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "save buyer cart header",
                source,
            })?;
        self.connection
            .execute(
                "delete from buyer_cart_lines where buyer_context_key = ?1",
                params![context_key.as_str()],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "clear buyer cart lines",
                source,
            })?;

        for line in &cart.lines {
            self.connection
                .execute(
                    "insert into buyer_cart_lines (
                        buyer_context_key,
                        product_id,
                        quantity,
                        updated_at
                     ) values (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
                    params![
                        context_key.as_str(),
                        line.product_id.to_string(),
                        i64::from(line.quantity),
                    ],
                )
                .map_err(|source| AppSqliteError::Query {
                    operation: "save buyer cart line",
                    source,
                })?;
        }

        Ok(())
    }

    pub fn clear_buyer_cart(&self, context: &BuyerContext) -> Result<(), AppSqliteError> {
        let context_key = context.storage_key();

        self.connection
            .execute(
                "delete from buyer_cart_lines where buyer_context_key = ?1",
                params![context_key.as_str()],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "delete buyer cart lines",
                source,
            })?;
        self.connection
            .execute(
                "update buyer_carts
                 set
                    farm_id = null,
                    buyer_order_note = '',
                    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 where buyer_context_key = ?1",
                params![context_key.as_str()],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "clear buyer cart header",
                source,
            })?;

        Ok(())
    }

    pub fn load_buyer_checkout(
        &self,
        context: &BuyerContext,
    ) -> Result<BuyerCheckoutProjection, AppSqliteError> {
        let context_key = context.storage_key();
        let header = self.load_cart_header(&context_key)?;
        let cart =
            self.build_cart_projection(header.clone(), self.load_cart_line_records(&context_key)?)?;
        let draft = header
            .map(BuyerCartHeader::into_checkout_draft)
            .unwrap_or_default();
        let fulfillment_summary = shared_fulfillment_summary(&cart.lines);

        Ok(BuyerCheckoutProjection {
            draft: draft.clone(),
            summary: BuyerCheckoutSummaryProjection {
                farm_display_name: cart.farm_display_name.clone(),
                fulfillment_summary: fulfillment_summary.clone(),
                line_count: cart.lines.len() as u32,
                subtotal_minor_units: cart.subtotal_minor_units,
                currency_code: cart.currency_code.clone(),
            },
            can_place_order: !cart.lines.is_empty()
                && fulfillment_summary.is_some()
                && !draft.name.trim().is_empty()
                && !draft.email.trim().is_empty(),
        })
    }

    pub fn save_buyer_checkout_draft(
        &self,
        context: &BuyerContext,
        draft: &BuyerCheckoutDraft,
    ) -> Result<(), AppSqliteError> {
        let context_key = context.storage_key();

        self.connection
            .execute(
                "insert into buyer_carts (
                    buyer_context_key,
                    farm_id,
                    updated_at
                 ) values (?1, null, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                 on conflict(buyer_context_key) do nothing",
                params![context_key.as_str()],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "ensure buyer checkout header",
                source,
            })?;
        self.connection
            .execute(
                "update buyer_carts
                 set
                    buyer_name = ?2,
                    buyer_email = ?3,
                    buyer_phone = ?4,
                    buyer_order_note = ?5,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 where buyer_context_key = ?1",
                params![
                    context_key.as_str(),
                    draft.name.trim(),
                    draft.email.trim(),
                    draft.phone.trim(),
                    draft.order_note.trim(),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "save buyer checkout draft",
                source,
            })?;

        Ok(())
    }

    pub fn place_buyer_order(&self, context: &BuyerContext) -> Result<OrderId, AppSqliteError> {
        let context_key = context.storage_key();
        let header =
            self.load_cart_header(&context_key)?
                .ok_or(AppSqliteError::InvalidProjection {
                    reason: "buyer cart header is missing",
                })?;
        let line_records = self.load_cart_line_records(&context_key)?;
        let cart = self.build_cart_projection(Some(header.clone()), line_records.clone())?;
        let checkout = self.load_buyer_checkout(context)?;

        if !checkout.can_place_order {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer checkout is not ready",
            });
        }

        let farm_id = cart.farm_id.ok_or(AppSqliteError::InvalidProjection {
            reason: "buyer cart farm is missing",
        })?;
        let fulfillment_window_id = shared_fulfillment_window_id(&line_records)?;
        let order_id = OrderId::new();
        let order_number = self.next_order_number(farm_id)?;

        self.connection
            .execute_batch("begin immediate transaction")
            .map_err(|source| AppSqliteError::Query {
                operation: "begin buyer checkout write",
                source,
            })?;

        let result = (|| {
            self.connection
                .execute(
                    "insert into orders (
                        id,
                        farm_id,
                        fulfillment_window_id,
                        order_number,
                        customer_display_name,
                        status,
                        updated_at,
                        buyer_context_key,
                        buyer_email,
                        buyer_phone,
                        buyer_order_note
                     ) values (
                        ?1,
                        ?2,
                        ?3,
                        ?4,
                        ?5,
                        'needs_action',
                        strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                        ?6,
                        ?7,
                        ?8,
                        ?9
                     )",
                    params![
                        order_id.to_string(),
                        farm_id.to_string(),
                        fulfillment_window_id.map(|id| id.to_string()),
                        order_number,
                        checkout.draft.name.trim(),
                        context_key.as_str(),
                        checkout.draft.email.trim(),
                        checkout.draft.phone.trim(),
                        checkout.draft.order_note.trim(),
                    ],
                )
                .map_err(|source| AppSqliteError::Query {
                    operation: "insert buyer order",
                    source,
                })?;

            for (index, line) in line_records.iter().enumerate() {
                self.connection
                    .execute(
                        "insert into order_lines (
                            id,
                            order_id,
                            title,
                            quantity_value,
                            quantity_unit_label,
                            quantity_display,
                            sort_index
                         ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            format!("{}:{}", order_id, line.listing.product_id),
                            order_id.to_string(),
                            line.listing.title,
                            i64::from(line.quantity),
                            line.listing.unit_label.as_str(),
                            format_quantity_display(line.quantity, &line.listing.unit_label),
                            index as i64,
                        ],
                    )
                    .map_err(|source| AppSqliteError::Query {
                        operation: "insert buyer order line",
                        source,
                    })?;
            }

            self.connection
                .execute(
                    "delete from buyer_cart_lines where buyer_context_key = ?1",
                    params![context_key.as_str()],
                )
                .map_err(|source| AppSqliteError::Query {
                    operation: "clear buyer cart lines after checkout",
                    source,
                })?;
            self.connection
                .execute(
                    "update buyer_carts
                     set
                        farm_id = null,
                        buyer_order_note = '',
                        updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                     where buyer_context_key = ?1",
                    params![context_key.as_str()],
                )
                .map_err(|source| AppSqliteError::Query {
                    operation: "reset buyer cart header after checkout",
                    source,
                })?;

            Ok(order_id)
        })();

        match result {
            Ok(order_id) => {
                self.connection.execute_batch("commit").map_err(|source| {
                    AppSqliteError::Query {
                        operation: "commit buyer checkout write",
                        source,
                    }
                })?;
                Ok(order_id)
            }
            Err(error) => {
                let _ = self.connection.execute_batch("rollback");
                Err(error)
            }
        }
    }

    pub fn load_buyer_orders(
        &self,
        context: &BuyerContext,
    ) -> Result<BuyerOrdersProjection, AppSqliteError> {
        let context_key = context.storage_key();
        let mut statement = self
            .connection
            .prepare(
                "select
                    o.id,
                    o.farm_id,
                    o.order_number,
                    o.status,
                    f.display_name,
                    fw.label,
                    fw.starts_at,
                    fw.ends_at
                 from orders o
                 inner join farms f on f.id = o.farm_id
                 left join fulfillment_windows fw on fw.id = o.fulfillment_window_id
                 where o.buyer_context_key = ?1
                 order by o.updated_at desc, o.id desc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare buyer orders list",
                source,
            })?;
        let rows = statement
            .query_map(params![context_key.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query buyer orders list",
                source,
            })?;
        let mut orders = Vec::new();

        for row in rows {
            let (
                order_id,
                farm_id,
                order_number,
                status,
                farm_display_name,
                fulfillment_label,
                fulfillment_starts_at,
                fulfillment_ends_at,
            ) = row.map_err(|source| AppSqliteError::Query {
                operation: "read buyer orders list",
                source,
            })?;

            orders.push(BuyerOrdersListRow {
                order_id: parse_typed_id("orders.id", order_id)?,
                farm_id: parse_typed_id("orders.farm_id", farm_id)?,
                order_number,
                farm_display_name,
                fulfillment_summary: format_fulfillment_summary(
                    fulfillment_label,
                    fulfillment_starts_at,
                    fulfillment_ends_at,
                ),
                status: BuyerOrderStatus::from(parse_order_status("orders.status", status)?),
            });
        }

        Ok(BuyerOrdersProjection { rows: orders })
    }

    pub fn load_buyer_order_detail(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
    ) -> Result<Option<BuyerOrderDetailProjection>, AppSqliteError> {
        let context_key = context.storage_key();
        let record = self
            .connection
            .query_row(
                "select
                    o.id,
                    o.farm_id,
                    o.order_number,
                    o.status,
                    o.buyer_order_note,
                    f.display_name,
                    fw.label,
                    fw.starts_at,
                    fw.ends_at
                 from orders o
                 inner join farms f on f.id = o.farm_id
                 left join fulfillment_windows fw on fw.id = o.fulfillment_window_id
                 where o.buyer_context_key = ?1 and o.id = ?2
                 limit 1",
                params![context_key.as_str(), order_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load buyer order detail",
                source,
            })?;

        record
            .map(
                |(
                    order_id,
                    farm_id,
                    order_number,
                    status,
                    order_note,
                    farm_display_name,
                    fulfillment_label,
                    fulfillment_starts_at,
                    fulfillment_ends_at,
                )| {
                    Ok(BuyerOrderDetailProjection {
                        order_id: parse_typed_id("orders.id", order_id.clone())?,
                        farm_id: parse_typed_id("orders.farm_id", farm_id)?,
                        order_number,
                        farm_display_name,
                        fulfillment_summary: format_fulfillment_summary(
                            fulfillment_label,
                            fulfillment_starts_at,
                            fulfillment_ends_at,
                        ),
                        status: BuyerOrderStatus::from(parse_order_status(
                            "orders.status",
                            status,
                        )?),
                        items: self.load_order_detail_items(order_id)?,
                        order_note: empty_string_to_none(order_note),
                    })
                },
            )
            .transpose()
    }

    fn build_cart_projection(
        &self,
        header: Option<BuyerCartHeader>,
        line_records: Vec<BuyerCartLineRecord>,
    ) -> Result<BuyerCartProjection, AppSqliteError> {
        let mut lines = Vec::with_capacity(line_records.len());
        let mut subtotal_minor_units = 0_u32;
        let mut currency_code = None;

        for line_record in line_records {
            let line_projection = line_record.into_projection()?;
            subtotal_minor_units = subtotal_minor_units
                .checked_add(line_projection.line_total_minor_units)
                .ok_or(AppSqliteError::InvalidProjection {
                    reason: "buyer cart subtotal overflowed",
                })?;

            if currency_code.is_none() {
                currency_code = Some(line_projection.unit_price.currency_code.clone());
            }
            lines.push(line_projection);
        }

        let farm_id = if lines.is_empty() {
            None
        } else {
            lines.first().map(|line| line.farm_id)
        }
        .or(header.as_ref().and_then(|header| header.farm_id));
        let farm_display_name = if let Some(line) = lines.first() {
            Some(line.farm_display_name.clone())
        } else if let Some(farm_id) = farm_id {
            self.load_farm_display_name(farm_id)?
        } else {
            None
        };
        let has_lines = !lines.is_empty();

        Ok(BuyerCartProjection {
            farm_id,
            farm_display_name,
            lines,
            subtotal_minor_units: has_lines.then_some(subtotal_minor_units),
            currency_code: has_lines.then_some(currency_code.unwrap_or_default()),
            replace_confirmation: None,
        })
    }

    fn current_utc_timestamp(&self) -> Result<String, AppSqliteError> {
        self.connection
            .query_row("select strftime('%Y-%m-%dT%H:%M:%SZ', 'now')", [], |row| {
                row.get(0)
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "load buyer current utc timestamp",
                source,
            })
    }

    fn load_listing_records(&self) -> Result<Vec<BuyerListingRecord>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select
                    p.id,
                    p.farm_id,
                    f.display_name,
                    f.readiness,
                    p.title,
                    p.subtitle,
                    p.status,
                    p.unit_label,
                    p.price_minor_units,
                    p.price_currency,
                    p.stock_count,
                    fw.id,
                    fw.label,
                    fw.starts_at,
                    fw.ends_at,
                    fw.pickup_location_id,
                    coalesce((
                        select max(afs.pickup_enabled)
                        from account_farm_setups afs
                        where afs.saved_farm_id = p.farm_id
                    ), 0),
                    coalesce((
                        select max(afs.delivery_enabled)
                        from account_farm_setups afs
                        where afs.saved_farm_id = p.farm_id
                    ), 0),
                    coalesce((
                        select max(afs.shipping_enabled)
                        from account_farm_setups afs
                        where afs.saved_farm_id = p.farm_id
                    ), 0)
                 from products p
                 inner join farms f on f.id = p.farm_id
                 left join fulfillment_windows fw on fw.id = p.availability_window_id",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare buyer listings query",
                source,
            })?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, Option<u32>>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<u32>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, Option<String>>(15)?,
                    row.get::<_, i64>(16)?,
                    row.get::<_, i64>(17)?,
                    row.get::<_, i64>(18)?,
                ))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query buyer listings",
                source,
            })?;
        let mut records = Vec::new();

        for row in rows {
            let (
                product_id,
                farm_id,
                farm_display_name,
                farm_readiness,
                title,
                subtitle,
                status,
                unit_label,
                price_minor_units,
                price_currency,
                stock_count,
                fulfillment_window_id,
                fulfillment_window_label,
                fulfillment_starts_at,
                fulfillment_ends_at,
                pickup_location_id,
                pickup_enabled,
                delivery_enabled,
                shipping_enabled,
            ) = row.map_err(|source| AppSqliteError::Query {
                operation: "read buyer listings",
                source,
            })?;

            records.push(BuyerListingRecord {
                product_id: parse_typed_id("products.id", product_id)?,
                farm_id: parse_typed_id("products.farm_id", farm_id)?,
                farm_display_name,
                farm_is_ready: farm_readiness == "ready",
                title,
                subtitle: empty_string_to_none(subtitle),
                status: parse_product_status("products.status", status)?,
                unit_label,
                price_minor_units,
                price_currency,
                stock_count,
                fulfillment_window_id: parse_optional_typed_id(
                    "products.availability_window_id",
                    fulfillment_window_id,
                )?,
                fulfillment_window_label: empty_string_to_none_option(fulfillment_window_label),
                fulfillment_starts_at,
                fulfillment_ends_at,
                pickup_location_present: pickup_location_id.is_some(),
                pickup_enabled: parse_sqlite_bool(
                    "account_farm_setups.pickup_enabled",
                    pickup_enabled,
                )?,
                delivery_enabled: parse_sqlite_bool(
                    "account_farm_setups.delivery_enabled",
                    delivery_enabled,
                )?,
                shipping_enabled: parse_sqlite_bool(
                    "account_farm_setups.shipping_enabled",
                    shipping_enabled,
                )?,
            });
        }

        Ok(records)
    }

    fn load_listing_record_by_id(
        &self,
        product_id: ProductId,
    ) -> Result<Option<BuyerListingRecord>, AppSqliteError> {
        Ok(self
            .load_listing_records()?
            .into_iter()
            .find(|record| record.product_id == product_id))
    }

    fn load_cart_header(
        &self,
        context_key: &str,
    ) -> Result<Option<BuyerCartHeader>, AppSqliteError> {
        self.connection
            .query_row(
                "select
                    farm_id,
                    buyer_name,
                    buyer_email,
                    buyer_phone,
                    buyer_order_note
                 from buyer_carts
                 where buyer_context_key = ?1
                 limit 1",
                params![context_key],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load buyer cart header",
                source,
            })?
            .map(
                |(farm_id, buyer_name, buyer_email, buyer_phone, buyer_order_note)| {
                    Ok(BuyerCartHeader {
                        farm_id: parse_optional_typed_id("buyer_carts.farm_id", farm_id)?,
                        buyer_name,
                        buyer_email,
                        buyer_phone,
                        buyer_order_note,
                    })
                },
            )
            .transpose()
    }

    fn load_cart_line_records(
        &self,
        context_key: &str,
    ) -> Result<Vec<BuyerCartLineRecord>, AppSqliteError> {
        let now_utc = self.current_utc_timestamp()?;
        let mut statement = self
            .connection
            .prepare(
                "select
                    bcl.quantity,
                    p.id,
                    p.farm_id,
                    f.display_name,
                    f.readiness,
                    p.title,
                    p.subtitle,
                    p.status,
                    p.unit_label,
                    p.price_minor_units,
                    p.price_currency,
                    p.stock_count,
                    fw.id,
                    fw.label,
                    fw.starts_at,
                    fw.ends_at,
                    fw.pickup_location_id,
                    coalesce((
                        select max(afs.pickup_enabled)
                        from account_farm_setups afs
                        where afs.saved_farm_id = p.farm_id
                    ), 0),
                    coalesce((
                        select max(afs.delivery_enabled)
                        from account_farm_setups afs
                        where afs.saved_farm_id = p.farm_id
                    ), 0),
                    coalesce((
                        select max(afs.shipping_enabled)
                        from account_farm_setups afs
                        where afs.saved_farm_id = p.farm_id
                    ), 0)
                 from buyer_cart_lines bcl
                 inner join products p on p.id = bcl.product_id
                 inner join farms f on f.id = p.farm_id
                 left join fulfillment_windows fw on fw.id = p.availability_window_id
                 where bcl.buyer_context_key = ?1
                 order by bcl.updated_at desc, p.id desc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare buyer cart lines",
                source,
            })?;
        let rows = statement
            .query_map(params![context_key], |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Option<u32>>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, Option<u32>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, Option<String>>(15)?,
                    row.get::<_, Option<String>>(16)?,
                    row.get::<_, i64>(17)?,
                    row.get::<_, i64>(18)?,
                    row.get::<_, i64>(19)?,
                ))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query buyer cart lines",
                source,
            })?;
        let mut line_records = Vec::new();

        for row in rows {
            let (
                quantity,
                product_id,
                farm_id,
                farm_display_name,
                farm_readiness,
                title,
                subtitle,
                status,
                unit_label,
                price_minor_units,
                price_currency,
                stock_count,
                fulfillment_window_id,
                fulfillment_window_label,
                fulfillment_starts_at,
                fulfillment_ends_at,
                pickup_location_id,
                pickup_enabled,
                delivery_enabled,
                shipping_enabled,
            ) = row.map_err(|source| AppSqliteError::Query {
                operation: "read buyer cart lines",
                source,
            })?;
            let listing = BuyerListingRecord {
                product_id: parse_typed_id("products.id", product_id)?,
                farm_id: parse_typed_id("products.farm_id", farm_id)?,
                farm_display_name,
                farm_is_ready: farm_readiness == "ready",
                title,
                subtitle: empty_string_to_none(subtitle),
                status: parse_product_status("products.status", status)?,
                unit_label,
                price_minor_units,
                price_currency,
                stock_count,
                fulfillment_window_id: parse_optional_typed_id(
                    "products.availability_window_id",
                    fulfillment_window_id,
                )?,
                fulfillment_window_label: empty_string_to_none_option(fulfillment_window_label),
                fulfillment_starts_at,
                fulfillment_ends_at,
                pickup_location_present: pickup_location_id.is_some(),
                pickup_enabled: parse_sqlite_bool(
                    "account_farm_setups.pickup_enabled",
                    pickup_enabled,
                )?,
                delivery_enabled: parse_sqlite_bool(
                    "account_farm_setups.delivery_enabled",
                    delivery_enabled,
                )?,
                shipping_enabled: parse_sqlite_bool(
                    "account_farm_setups.shipping_enabled",
                    shipping_enabled,
                )?,
            };

            if listing.is_buyer_visible(&now_utc) {
                line_records.push(BuyerCartLineRecord { listing, quantity });
            }
        }

        Ok(line_records)
    }

    fn load_order_detail_items(
        &self,
        order_id: String,
    ) -> Result<Vec<OrderDetailItemRow>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select title, quantity_display
                 from order_lines
                 where order_id = ?1
                 order by sort_index asc, id asc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare buyer order detail items",
                source,
            })?;
        let rows = statement
            .query_map(params![order_id], |row| {
                Ok(OrderDetailItemRow {
                    title: row.get(0)?,
                    quantity_display: row.get(1)?,
                })
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query buyer order detail items",
                source,
            })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| AppSqliteError::Query {
                operation: "read buyer order detail items",
                source,
            })
    }

    fn load_farm_display_name(&self, farm_id: FarmId) -> Result<Option<String>, AppSqliteError> {
        self.connection
            .query_row(
                "select display_name from farms where id = ?1 limit 1",
                params![farm_id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load buyer cart farm display name",
                source,
            })
    }

    fn next_order_number(&self, farm_id: FarmId) -> Result<String, AppSqliteError> {
        let max_suffix = self
            .connection
            .query_row(
                "select coalesce(max(cast(substr(order_number, 3) as integer)), 999)
                 from orders
                 where farm_id = ?1
                   and order_number like 'R-%'
                   and substr(order_number, 3) glob '[0-9]*'",
                params![farm_id.to_string()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "load next buyer order number",
                source,
            })?;

        Ok(format!("R-{}", max_suffix + 1))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BuyerCartHeader {
    farm_id: Option<FarmId>,
    buyer_name: String,
    buyer_email: String,
    buyer_phone: String,
    buyer_order_note: String,
}

impl BuyerCartHeader {
    fn into_checkout_draft(self) -> BuyerCheckoutDraft {
        BuyerCheckoutDraft {
            name: self.buyer_name,
            email: self.buyer_email,
            phone: self.buyer_phone,
            order_note: self.buyer_order_note,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BuyerListingRecord {
    product_id: ProductId,
    farm_id: FarmId,
    farm_display_name: String,
    farm_is_ready: bool,
    title: String,
    subtitle: Option<String>,
    status: ProductStatus,
    unit_label: String,
    price_minor_units: Option<u32>,
    price_currency: String,
    stock_count: Option<u32>,
    fulfillment_window_id: Option<FulfillmentWindowId>,
    fulfillment_window_label: Option<String>,
    fulfillment_starts_at: Option<String>,
    fulfillment_ends_at: Option<String>,
    pickup_location_present: bool,
    pickup_enabled: bool,
    delivery_enabled: bool,
    shipping_enabled: bool,
}

impl BuyerListingRecord {
    fn is_buyer_visible(&self, now_utc: &str) -> bool {
        self.farm_is_ready
            && self.status == ProductStatus::Published
            && self.stock_count.is_some_and(|quantity| quantity > 0)
            && self.price_minor_units.is_some_and(|amount| amount > 0)
            && !self.unit_label.trim().is_empty()
            && self.fulfillment_window_id.is_some()
            && self
                .fulfillment_ends_at
                .as_deref()
                .is_some_and(|ends_at| ends_at >= now_utc)
            && !self.fulfillment_methods().is_empty()
    }

    fn matches_search(&self, search_query: Option<&str>) -> bool {
        let Some(search_query) = search_query else {
            return true;
        };

        self.title.to_lowercase().contains(search_query)
            || self
                .subtitle
                .as_deref()
                .is_some_and(|subtitle| subtitle.to_lowercase().contains(search_query))
            || self.farm_display_name.to_lowercase().contains(search_query)
    }

    fn matches_fulfillment_methods(&self, selected: &BTreeSet<FarmOrderMethod>) -> bool {
        selected.is_empty()
            || self
                .fulfillment_methods()
                .iter()
                .any(|method| selected.contains(method))
    }

    fn detail_text(&self) -> Option<String> {
        self.subtitle.clone()
    }

    fn into_listing_row(self, now_utc: &str) -> Result<BuyerListingRow, AppSqliteError> {
        let price = self
            .price_presentation()
            .ok_or(AppSqliteError::InvalidProjection {
                reason: "buyer listing price is missing",
            })?;
        let availability = self.availability_summary(now_utc)?;
        let stock = self.stock_summary();
        let fulfillment_methods = self.fulfillment_methods();
        let next_fulfillment_window_label = Some(self.fulfillment_summary_label()?);

        Ok(BuyerListingRow {
            product_id: self.product_id,
            farm_id: self.farm_id,
            farm_display_name: self.farm_display_name,
            title: self.title,
            subtitle: self.subtitle,
            price,
            availability,
            stock,
            fulfillment_methods,
            next_fulfillment_window_label,
        })
    }

    fn availability_summary(
        &self,
        now_utc: &str,
    ) -> Result<ProductAvailabilitySummary, AppSqliteError> {
        let starts_at =
            self.fulfillment_starts_at
                .clone()
                .ok_or(AppSqliteError::InvalidProjection {
                    reason: "buyer listing fulfillment start is missing",
                })?;
        let ends_at =
            self.fulfillment_ends_at
                .clone()
                .ok_or(AppSqliteError::InvalidProjection {
                    reason: "buyer listing fulfillment end is missing",
                })?;
        let state = if starts_at.as_str() <= now_utc {
            ProductAvailabilityState::Open
        } else {
            ProductAvailabilityState::Scheduled
        };

        Ok(ProductAvailabilitySummary {
            state,
            label: self
                .fulfillment_window_label
                .clone()
                .unwrap_or_else(|| format_window_label(&starts_at, &ends_at)),
        })
    }

    fn fulfillment_summary_label(&self) -> Result<String, AppSqliteError> {
        match (
            self.fulfillment_window_label.clone(),
            self.fulfillment_starts_at.as_deref(),
            self.fulfillment_ends_at.as_deref(),
        ) {
            (Some(label), _, _) => Ok(label),
            (None, Some(starts_at), Some(ends_at)) => Ok(format_window_label(starts_at, ends_at)),
            _ => Err(AppSqliteError::InvalidProjection {
                reason: "buyer listing fulfillment summary is missing",
            }),
        }
    }

    fn stock_summary(&self) -> ProductStockSummary {
        let quantity = self.stock_count;
        let state = match quantity {
            Some(0) => ProductStockState::SoldOut,
            Some(quantity) if quantity <= BUYER_LOW_STOCK_THRESHOLD => ProductStockState::LowStock,
            Some(_) => ProductStockState::InStock,
            None => ProductStockState::Unset,
        };

        ProductStockSummary {
            quantity,
            unit_label: Some(self.unit_label.clone()),
            state,
        }
    }

    fn price_presentation(&self) -> Option<ProductPricePresentation> {
        self.price_minor_units
            .filter(|amount| *amount > 0)
            .map(|amount_minor_units| ProductPricePresentation {
                amount_minor_units,
                currency_code: normalize_currency_code(&self.price_currency),
                unit_label: self.unit_label.clone(),
            })
    }

    fn fulfillment_methods(&self) -> BTreeSet<FarmOrderMethod> {
        let mut methods = BTreeSet::new();
        if self.pickup_enabled
            || (!self.delivery_enabled && !self.shipping_enabled && self.pickup_location_present)
        {
            methods.insert(FarmOrderMethod::Pickup);
        }
        if self.delivery_enabled {
            methods.insert(FarmOrderMethod::Delivery);
        }
        if self.shipping_enabled {
            methods.insert(FarmOrderMethod::Shipping);
        }

        methods
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BuyerCartLineRecord {
    listing: BuyerListingRecord,
    quantity: u32,
}

impl BuyerCartLineRecord {
    fn into_projection(self) -> Result<BuyerCartLineProjection, AppSqliteError> {
        let unit_price =
            self.listing
                .price_presentation()
                .ok_or(AppSqliteError::InvalidProjection {
                    reason: "buyer cart line price is missing",
                })?;
        let line_total_minor_units = unit_price
            .amount_minor_units
            .checked_mul(self.quantity)
            .ok_or(AppSqliteError::InvalidProjection {
                reason: "buyer cart line total overflowed",
            })?;

        Ok(BuyerCartLineProjection {
            product_id: self.listing.product_id,
            farm_id: self.listing.farm_id,
            farm_display_name: self.listing.farm_display_name.clone(),
            title: self.listing.title.clone(),
            quantity: self.quantity,
            unit_price,
            line_total_minor_units,
            fulfillment_summary: self.listing.fulfillment_summary_label()?,
        })
    }
}

fn validate_cart_projection(cart: &BuyerCartProjection) -> Result<(), AppSqliteError> {
    if cart.lines.is_empty() {
        return Ok(());
    }

    let farm_id = cart.farm_id.ok_or(AppSqliteError::InvalidProjection {
        reason: "buyer cart farm is required when cart has lines",
    })?;

    for line in &cart.lines {
        if line.quantity == 0 {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer cart quantities must stay positive",
            });
        }
        if line.farm_id != farm_id {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer cart must remain single farm",
            });
        }
    }

    Ok(())
}

fn shared_fulfillment_summary(lines: &[BuyerCartLineProjection]) -> Option<String> {
    let first = lines.first()?.fulfillment_summary.clone();

    lines
        .iter()
        .all(|line| line.fulfillment_summary == first)
        .then_some(first)
}

fn shared_fulfillment_window_id(
    lines: &[BuyerCartLineRecord],
) -> Result<Option<FulfillmentWindowId>, AppSqliteError> {
    let Some(first) = lines.first() else {
        return Err(AppSqliteError::InvalidProjection {
            reason: "buyer cart must contain at least one line",
        });
    };
    let first_window_id = first.listing.fulfillment_window_id;

    if lines
        .iter()
        .all(|line| line.listing.fulfillment_window_id == first_window_id)
    {
        Ok(first_window_id)
    } else {
        Err(AppSqliteError::InvalidProjection {
            reason: "buyer cart must share one fulfillment window at checkout",
        })
    }
}

fn normalize_search_query(search_query: &str) -> Option<String> {
    let trimmed = search_query.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_lowercase())
    }
}

fn sort_listing_records(records: &mut [BuyerListingRecord], now_utc: &str) {
    records.sort_by(|left, right| {
        left.fulfillment_starts_at
            .cmp(&right.fulfillment_starts_at)
            .then_with(|| {
                left.availability_state(now_utc)
                    .cmp(&right.availability_state(now_utc))
            })
            .then_with(|| {
                left.farm_display_name
                    .to_lowercase()
                    .cmp(&right.farm_display_name.to_lowercase())
            })
            .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
            .then_with(|| left.product_id.cmp(&right.product_id))
    });
}

impl BuyerListingRecord {
    fn availability_state(&self, now_utc: &str) -> u8 {
        match self.fulfillment_starts_at.as_deref() {
            Some(starts_at) if starts_at <= now_utc => 0,
            Some(_) => 1,
            None => 2,
        }
    }
}

fn format_window_label(starts_at: &str, ends_at: &str) -> String {
    let start_date = starts_at.get(0..10);
    let start_time = starts_at.get(11..16);
    let end_date = ends_at.get(0..10);
    let end_time = ends_at.get(11..16);

    match (start_date, start_time, end_date, end_time) {
        (Some(start_date), Some(start_time), Some(end_date), Some(end_time))
            if start_date == end_date =>
        {
            format!("{start_date} {start_time}-{end_time} UTC")
        }
        (Some(start_date), Some(start_time), Some(end_date), Some(end_time)) => {
            format!("{start_date} {start_time} UTC to {end_date} {end_time} UTC")
        }
        _ => starts_at.to_owned(),
    }
}

fn format_fulfillment_summary(
    label: Option<String>,
    starts_at: Option<String>,
    ends_at: Option<String>,
) -> String {
    if let Some(label) = empty_string_to_none_option(label) {
        return label;
    }

    match (starts_at.as_deref(), ends_at.as_deref()) {
        (Some(starts_at), Some(ends_at)) => format_window_label(starts_at, ends_at),
        _ => "Fulfillment pending".to_owned(),
    }
}

fn format_quantity_display(quantity: u32, unit_label: &str) -> String {
    let trimmed = unit_label.trim();
    if trimmed.is_empty() {
        quantity.to_string()
    } else {
        format!("{quantity} {trimmed}")
    }
}

fn normalize_currency_code(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "USD".to_owned()
    } else {
        trimmed.to_ascii_uppercase()
    }
}

fn parse_typed_id<T>(field: &'static str, value: String) -> Result<T, AppSqliteError>
where
    T: std::str::FromStr,
{
    value
        .parse()
        .map_err(|_| AppSqliteError::DecodeId { field, value })
}

fn parse_optional_typed_id<T>(
    field: &'static str,
    value: Option<String>,
) -> Result<Option<T>, AppSqliteError>
where
    T: std::str::FromStr,
{
    value.map(|value| parse_typed_id(field, value)).transpose()
}

fn parse_sqlite_bool(field: &'static str, value: i64) -> Result<bool, AppSqliteError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(AppSqliteError::DecodeEnum {
            field,
            value: value.to_string(),
        }),
    }
}

fn parse_product_status(
    field: &'static str,
    value: String,
) -> Result<ProductStatus, AppSqliteError> {
    match value.as_str() {
        "draft" => Ok(ProductStatus::Draft),
        "published" => Ok(ProductStatus::Published),
        "paused" => Ok(ProductStatus::Paused),
        "archived" => Ok(ProductStatus::Archived),
        _ => Err(AppSqliteError::DecodeEnum { field, value }),
    }
}

fn parse_order_status(field: &'static str, value: String) -> Result<OrderStatus, AppSqliteError> {
    match value.as_str() {
        "needs_action" => Ok(OrderStatus::NeedsAction),
        "scheduled" => Ok(OrderStatus::Scheduled),
        "packed" => Ok(OrderStatus::Packed),
        "completed" => Ok(OrderStatus::Completed),
        "refunded" => Ok(OrderStatus::Refunded),
        _ => Err(AppSqliteError::DecodeEnum { field, value }),
    }
}

fn empty_string_to_none(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn empty_string_to_none_option(value: Option<String>) -> Option<String> {
    value.and_then(empty_string_to_none)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use radroots_studio_app_models::{
        BuyerContext, FarmId, FarmOrderMethod, FulfillmentWindowId, OrderId, OrderStatus,
        PickupLocationId, ProductId,
    };
    use rusqlite::{Connection, params};

    use crate::{AppSqliteError, AppSqliteStore, DatabaseTarget};

    use super::AppBuyerRepository;

    #[test]
    fn buyer_listings_and_product_detail_follow_catalog_truth() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let repository = AppBuyerRepository::new(connection);
        let farm_id = insert_farm(connection, "Willow Farm", "ready");
        let future_window_id = insert_window(
            connection,
            farm_id,
            Some(insert_pickup_location(connection, farm_id, "Barn pickup")),
            "Friday pickup",
            "2099-04-18T16:00:00Z",
            "2099-04-18T18:00:00Z",
        );

        insert_farm_setup_binding(connection, "acct_farmer", farm_id, true, false, false);
        let visible_product_id = insert_product(
            connection,
            farm_id,
            SeedProduct {
                title: "Salad mix",
                subtitle: "Spring blend",
                status: "published",
                unit_label: "bag",
                price_minor_units: Some(650),
                price_currency: "USD",
                stock_count: Some(8),
                availability_window_id: Some(future_window_id),
            },
        );
        insert_product(
            connection,
            farm_id,
            SeedProduct {
                title: "Pea shoots",
                subtitle: "Tray-grown",
                status: "draft",
                unit_label: "bag",
                price_minor_units: Some(450),
                price_currency: "USD",
                stock_count: Some(4),
                availability_window_id: Some(future_window_id),
            },
        );
        insert_product(
            connection,
            farm_id,
            SeedProduct {
                title: "Sold out carrots",
                subtitle: "",
                status: "published",
                unit_label: "bunch",
                price_minor_units: Some(500),
                price_currency: "USD",
                stock_count: Some(0),
                availability_window_id: Some(future_window_id),
            },
        );

        let listings = repository
            .load_buyer_listings("salad", &BTreeSet::from([FarmOrderMethod::Pickup]))
            .expect("buyer listings should load");
        let detail = repository
            .load_buyer_product_detail(visible_product_id)
            .expect("buyer detail should load")
            .expect("buyer detail should exist");

        assert_eq!(listings.rows.len(), 1);
        assert_eq!(listings.rows[0].title, "Salad mix");
        assert_eq!(
            listings.rows[0].fulfillment_methods,
            BTreeSet::from([FarmOrderMethod::Pickup])
        );
        assert_eq!(
            listings.rows[0].next_fulfillment_window_label.as_deref(),
            Some("Friday pickup")
        );
        assert_eq!(detail.selected_quantity, 1);
        assert_eq!(detail.detail_text.as_deref(), Some("Spring blend"));
    }

    #[test]
    fn buyer_cart_checkout_and_order_history_round_trip_for_guest_context() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let repository = AppBuyerRepository::new(connection);
        let context = BuyerContext::Guest;
        let farm_id = insert_farm(connection, "Willow Farm", "ready");
        let pickup_location_id = insert_pickup_location(connection, farm_id, "Barn pickup");
        let future_window_id = insert_window(
            connection,
            farm_id,
            Some(pickup_location_id),
            "Friday pickup",
            "2099-04-18T16:00:00Z",
            "2099-04-18T18:00:00Z",
        );

        insert_farm_setup_binding(connection, "acct_farmer", farm_id, true, false, false);
        let product_id = insert_product(
            connection,
            farm_id,
            SeedProduct {
                title: "Salad mix",
                subtitle: "Spring blend",
                status: "published",
                unit_label: "bag",
                price_minor_units: Some(650),
                price_currency: "USD",
                stock_count: Some(8),
                availability_window_id: Some(future_window_id),
            },
        );
        let listing = repository
            .load_buyer_product_detail(product_id)
            .expect("buyer detail should load")
            .expect("listing should exist")
            .listing;

        repository
            .replace_buyer_cart(
                &context,
                &radroots_studio_app_models::BuyerCartProjection {
                    farm_id: Some(farm_id),
                    farm_display_name: Some("Willow Farm".to_owned()),
                    lines: vec![radroots_studio_app_models::BuyerCartLineProjection {
                        product_id: listing.product_id,
                        farm_id: listing.farm_id,
                        farm_display_name: listing.farm_display_name.clone(),
                        title: listing.title.clone(),
                        quantity: 2,
                        unit_price: listing.price.clone(),
                        line_total_minor_units: 1300,
                        fulfillment_summary: "Friday pickup".to_owned(),
                    }],
                    subtotal_minor_units: Some(1300),
                    currency_code: Some("USD".to_owned()),
                    replace_confirmation: None,
                },
            )
            .expect("buyer cart should save");
        repository
            .save_buyer_checkout_draft(
                &context,
                &radroots_studio_app_models::BuyerCheckoutDraft {
                    name: "Casey Buyer".to_owned(),
                    email: "casey@example.com".to_owned(),
                    phone: "555-0101".to_owned(),
                    order_note: "Leave by the cooler".to_owned(),
                },
            )
            .expect("buyer checkout draft should save");

        let checkout = repository
            .load_buyer_checkout(&context)
            .expect("buyer checkout should load");
        let order_id = repository
            .place_buyer_order(&context)
            .expect("buyer checkout should place order");
        let buyer_orders = repository
            .load_buyer_orders(&context)
            .expect("buyer orders should load");
        let buyer_order_detail = repository
            .load_buyer_order_detail(&context, order_id)
            .expect("buyer order detail should load")
            .expect("buyer order detail should exist");
        let cart_after_checkout = repository
            .load_buyer_cart(&context)
            .expect("buyer cart should load after checkout");

        assert!(checkout.can_place_order);
        assert_eq!(checkout.summary.line_count, 1);
        assert_eq!(buyer_orders.rows.len(), 1);
        assert_eq!(
            buyer_orders.rows[0].status,
            radroots_studio_app_models::BuyerOrderStatus::Placed
        );
        assert_eq!(buyer_order_detail.items.len(), 1);
        assert_eq!(
            buyer_order_detail.order_note.as_deref(),
            Some("Leave by the cooler")
        );
        assert!(cart_after_checkout.lines.is_empty());
        assert_eq!(cart_after_checkout.farm_id, None);
        assert_eq!(
            read_order_status(connection, order_id),
            OrderStatus::NeedsAction
        );
        assert_eq!(
            read_order_context_key(connection, order_id).as_deref(),
            Some("guest")
        );
        assert_eq!(
            read_order_contact(connection, order_id),
            (
                "Casey Buyer".to_owned(),
                "casey@example.com".to_owned(),
                "555-0101".to_owned(),
                "Leave by the cooler".to_owned(),
            )
        );
        assert_eq!(row_count(connection, "local_outbox"), 0);
        assert_eq!(row_count(connection, "local_conflicts"), 0);
        assert_eq!(row_count(connection, "sync_checkpoints"), 0);
    }

    #[test]
    fn buyer_orders_filter_to_context_and_ignore_seller_orders() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let repository = AppBuyerRepository::new(connection);
        let farm_id = insert_farm(connection, "Willow Farm", "ready");

        insert_order(
            connection,
            OrderId::new(),
            farm_id,
            "R-100",
            "needs_action",
            None,
            "",
            "",
            "",
        );
        insert_order(
            connection,
            OrderId::new(),
            farm_id,
            "R-101",
            "scheduled",
            Some("guest"),
            "guest@example.com",
            "",
            "",
        );
        insert_order(
            connection,
            OrderId::new(),
            farm_id,
            "R-102",
            "packed",
            Some("account:acct_buyer"),
            "buyer@example.com",
            "",
            "",
        );

        let guest_orders = repository
            .load_buyer_orders(&BuyerContext::Guest)
            .expect("guest orders should load");
        let account_orders = repository
            .load_buyer_orders(&BuyerContext::account("acct_buyer"))
            .expect("account orders should load");

        assert_eq!(guest_orders.rows.len(), 1);
        assert_eq!(guest_orders.rows[0].order_number, "R-101");
        assert_eq!(account_orders.rows.len(), 1);
        assert_eq!(account_orders.rows[0].order_number, "R-102");
    }

    #[test]
    fn buyer_cart_rejects_cross_farm_lines() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let farm_id = FarmId::new();
        let other_farm_id = FarmId::new();

        let error = repository_error(&store, farm_id, other_farm_id);

        assert!(matches!(error, AppSqliteError::InvalidProjection { .. }));
    }

    fn repository_error(
        store: &AppSqliteStore,
        farm_id: FarmId,
        other_farm_id: FarmId,
    ) -> AppSqliteError {
        AppBuyerRepository::new(store.connection())
            .replace_buyer_cart(
                &BuyerContext::Guest,
                &radroots_studio_app_models::BuyerCartProjection {
                    farm_id: Some(farm_id),
                    farm_display_name: Some("Willow Farm".to_owned()),
                    lines: vec![radroots_studio_app_models::BuyerCartLineProjection {
                        product_id: ProductId::new(),
                        farm_id: other_farm_id,
                        farm_display_name: "Other Farm".to_owned(),
                        title: "Mismatch".to_owned(),
                        quantity: 1,
                        unit_price: radroots_studio_app_models::ProductPricePresentation {
                            amount_minor_units: 500,
                            currency_code: "USD".to_owned(),
                            unit_label: "bag".to_owned(),
                        },
                        line_total_minor_units: 500,
                        fulfillment_summary: "Friday pickup".to_owned(),
                    }],
                    subtotal_minor_units: Some(500),
                    currency_code: Some("USD".to_owned()),
                    replace_confirmation: None,
                },
            )
            .expect_err("cross-farm cart should fail")
    }

    fn insert_farm(connection: &Connection, display_name: &str, readiness: &str) -> FarmId {
        let farm_id = FarmId::new();

        connection
            .execute(
                "insert into farms (
                    id,
                    display_name,
                    readiness,
                    timezone,
                    currency_code,
                    created_at,
                    updated_at
                 ) values (?1, ?2, ?3, 'UTC', 'USD', '2026-04-20T08:00:00Z', '2026-04-20T08:00:00Z')",
                params![farm_id.to_string(), display_name, readiness],
            )
            .expect("farm insert should succeed");

        farm_id
    }

    fn insert_pickup_location(
        connection: &Connection,
        farm_id: FarmId,
        label: &str,
    ) -> PickupLocationId {
        let pickup_location_id = PickupLocationId::new();

        connection
            .execute(
                "insert into pickup_locations (
                    id,
                    farm_id,
                    label,
                    address_line,
                    directions,
                    is_default,
                    created_at,
                    updated_at
                 ) values (?1, ?2, ?3, '14 County Road', null, 1, '2026-04-20T08:00:00Z', '2026-04-20T08:00:00Z')",
                params![pickup_location_id.to_string(), farm_id.to_string(), label],
            )
            .expect("pickup location insert should succeed");

        pickup_location_id
    }

    fn insert_window(
        connection: &Connection,
        farm_id: FarmId,
        pickup_location_id: Option<PickupLocationId>,
        label: &str,
        starts_at: &str,
        ends_at: &str,
    ) -> FulfillmentWindowId {
        let fulfillment_window_id = FulfillmentWindowId::new();

        connection
            .execute(
                "insert into fulfillment_windows (
                    id,
                    farm_id,
                    starts_at,
                    ends_at,
                    capacity_limit,
                    created_at,
                    updated_at,
                    pickup_location_id,
                    label,
                    order_cutoff_at
                 ) values (?1, ?2, ?3, ?4, null, ?3, ?3, ?5, ?6, ?3)",
                params![
                    fulfillment_window_id.to_string(),
                    farm_id.to_string(),
                    starts_at,
                    ends_at,
                    pickup_location_id.map(|id| id.to_string()),
                    label,
                ],
            )
            .expect("window insert should succeed");

        fulfillment_window_id
    }

    fn insert_farm_setup_binding(
        connection: &Connection,
        account_id: &str,
        farm_id: FarmId,
        pickup_enabled: bool,
        delivery_enabled: bool,
        shipping_enabled: bool,
    ) {
        connection
            .execute(
                "insert into account_farm_setups (
                    account_id,
                    farm_name,
                    location_or_service_area,
                    pickup_enabled,
                    delivery_enabled,
                    shipping_enabled,
                    saved_farm_id,
                    saved_farm_display_name,
                    saved_farm_readiness,
                    updated_at
                 ) values (?1, 'Willow Farm', 'County Road', ?2, ?3, ?4, ?5, 'Willow Farm', 'ready', '2026-04-20T08:00:00Z')",
                params![
                    account_id,
                    i64::from(pickup_enabled),
                    i64::from(delivery_enabled),
                    i64::from(shipping_enabled),
                    farm_id.to_string(),
                ],
            )
            .expect("farm setup binding insert should succeed");
    }

    struct SeedProduct<'a> {
        title: &'a str,
        subtitle: &'a str,
        status: &'a str,
        unit_label: &'a str,
        price_minor_units: Option<u32>,
        price_currency: &'a str,
        stock_count: Option<u32>,
        availability_window_id: Option<FulfillmentWindowId>,
    }

    fn insert_product(
        connection: &Connection,
        farm_id: FarmId,
        product: SeedProduct<'_>,
    ) -> ProductId {
        let product_id = ProductId::new();

        connection
            .execute(
                "insert into products (
                    id,
                    farm_id,
                    title,
                    subtitle,
                    status,
                    unit_label,
                    price_minor_units,
                    price_currency,
                    stock_count,
                    availability_window_id,
                    updated_at
                 ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, '2026-04-20T09:00:00Z')",
                params![
                    product_id.to_string(),
                    farm_id.to_string(),
                    product.title,
                    product.subtitle,
                    product.status,
                    product.unit_label,
                    product.price_minor_units,
                    product.price_currency,
                    product.stock_count,
                    product.availability_window_id.map(|id| id.to_string()),
                ],
            )
            .expect("product insert should succeed");

        product_id
    }

    fn insert_order(
        connection: &Connection,
        order_id: OrderId,
        farm_id: FarmId,
        order_number: &str,
        status: &str,
        buyer_context_key: Option<&str>,
        buyer_email: &str,
        buyer_phone: &str,
        buyer_order_note: &str,
    ) {
        connection
            .execute(
                "insert into orders (
                    id,
                    farm_id,
                    fulfillment_window_id,
                    order_number,
                    customer_display_name,
                    status,
                    updated_at,
                    buyer_context_key,
                    buyer_email,
                    buyer_phone,
                    buyer_order_note
                 ) values (?1, ?2, null, ?3, 'Casey', ?4, '2026-04-20T10:00:00Z', ?5, ?6, ?7, ?8)",
                params![
                    order_id.to_string(),
                    farm_id.to_string(),
                    order_number,
                    status,
                    buyer_context_key,
                    buyer_email,
                    buyer_phone,
                    buyer_order_note,
                ],
            )
            .expect("order insert should succeed");
    }

    fn read_order_status(connection: &Connection, order_id: OrderId) -> OrderStatus {
        let status = connection
            .query_row(
                "select status from orders where id = ?1 limit 1",
                params![order_id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .expect("order status should load");

        super::parse_order_status("orders.status", status).expect("order status should parse")
    }

    fn read_order_context_key(connection: &Connection, order_id: OrderId) -> Option<String> {
        connection
            .query_row(
                "select buyer_context_key from orders where id = ?1 limit 1",
                params![order_id.to_string()],
                |row| row.get::<_, Option<String>>(0),
            )
            .expect("order context should load")
    }

    fn read_order_contact(
        connection: &Connection,
        order_id: OrderId,
    ) -> (String, String, String, String) {
        connection
            .query_row(
                "select customer_display_name, buyer_email, buyer_phone, buyer_order_note
                 from orders where id = ?1 limit 1",
                params![order_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .expect("order contact should load")
    }

    fn row_count(connection: &Connection, table_name: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) FROM {table_name}");

        connection
            .query_row(&sql, [], |row| row.get(0))
            .expect("row count query should succeed")
    }
}
