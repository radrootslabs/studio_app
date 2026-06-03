use std::collections::{BTreeMap, BTreeSet};

use radroots_studio_app_view::{
    BuyerCartLineProjection, BuyerCartProjection, BuyerCartReplaceConfirmationProjection,
    BuyerCheckoutDisabledReason, BuyerCheckoutDraft, BuyerCheckoutProjection,
    BuyerCheckoutSummaryProjection, BuyerContext, BuyerListingRow, BuyerListingsProjection,
    BuyerOrderDetailProjection, BuyerOrderStatus, BuyerOrdersListRow, BuyerOrdersProjection,
    BuyerProductDetailProjection, FarmId, FarmOrderMethod, FulfillmentWindowId, OrderDetailItemRow,
    OrderId, OrderStatus, ProductAvailabilityState, ProductAvailabilitySummary, ProductId,
    ProductPricePresentation, ProductStatus, ProductStockState, ProductStockSummary,
    RepeatDemandEligibility, RepeatDemandHandoffProjection, TradePaymentDisplayStatus,
    TradeRevisionStatus, TradeWorkflowProjection,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use super::order_detail::{order_detail_economics, order_detail_item_row};
use crate::AppSqliteError;

const BUYER_LOW_STOCK_THRESHOLD: u32 = 3;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BuyerRepeatDemandApplyOutcome {
    Applied,
    ConfirmationRequired(BuyerCartReplaceConfirmationProjection),
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuyerOrderLocalEventExport {
    pub order_id: OrderId,
    pub farm_id: FarmId,
    pub farm_display_name: String,
    pub order_number: String,
    pub status: String,
    pub buyer_context_key: String,
    pub buyer_name: String,
    pub buyer_email: String,
    pub buyer_phone: String,
    pub buyer_order_note: String,
    pub updated_at: String,
    pub fulfillment_window_id: Option<FulfillmentWindowId>,
    pub fulfillment_window_label: Option<String>,
    pub fulfillment_starts_at: Option<String>,
    pub fulfillment_ends_at: Option<String>,
    pub lines: Vec<BuyerOrderLocalEventLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuyerOrderLocalEventLine {
    pub product_id: ProductId,
    pub title: String,
    pub quantity: u32,
    pub quantity_unit_label: String,
    pub quantity_display: String,
    pub unit_price_minor_units: Option<u32>,
    pub price_currency: String,
    pub listing_bin_id: Option<String>,
    pub farm_key: Option<String>,
    pub listing_addr: Option<String>,
    pub listing_event_id: Option<String>,
    pub listing_relays: Vec<String>,
    pub seller_pubkey: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuyerOrderCoordinationState {
    Pending,
    Synced,
    Failed,
}

impl BuyerOrderCoordinationState {
    fn from_storage_key(field: &'static str, value: String) -> Result<Self, AppSqliteError> {
        match value.as_str() {
            "pending" => Ok(Self::Pending),
            "synced" => Ok(Self::Synced),
            "failed" => Ok(Self::Failed),
            _ => Err(AppSqliteError::DecodeEnum { field, value }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuyerOrderCoordinationRecord {
    pub order_id: OrderId,
    pub buyer_context_key: String,
    pub record_id: Option<String>,
    pub state: BuyerOrderCoordinationState,
    pub payload_json: Option<String>,
    pub attempt_count: u32,
    pub last_error_message: Option<String>,
}

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
            let snapshot = self.load_buyer_cart_line_snapshot(line.product_id)?;
            let listing_relays_json = encode_listing_relays(&snapshot.listing_relays)?;
            self.connection
                .execute(
                    "insert into buyer_cart_lines (
                        buyer_context_key,
                        product_id,
                        quantity,
                        listing_bin_id,
                        quantity_unit_label,
                        unit_price_minor_units,
                        price_currency,
                        farm_key,
                        listing_addr,
                        listing_event_id,
                        listing_relays_json,
                        seller_pubkey,
                        updated_at
                     ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
                    params![
                        context_key.as_str(),
                        line.product_id.to_string(),
                        i64::from(line.quantity),
                        snapshot.listing_bin_id.as_deref(),
                        line.unit_price.unit_label.as_str(),
                        line.unit_price.amount_minor_units,
                        normalize_currency_code(&line.unit_price.currency_code),
                        snapshot.farm_key.as_deref(),
                        snapshot.listing_addr.as_deref(),
                        snapshot.listing_event_id.as_deref(),
                        listing_relays_json.as_deref(),
                        snapshot.seller_pubkey.as_deref(),
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
        let place_order_disabled_reason =
            buyer_checkout_disabled_reason(context, &cart, fulfillment_summary.as_ref(), &draft);

        Ok(BuyerCheckoutProjection {
            draft: draft.clone(),
            summary: BuyerCheckoutSummaryProjection {
                farm_display_name: cart.farm_display_name.clone(),
                fulfillment_summary: fulfillment_summary.clone(),
                line_count: cart.lines.len() as u32,
                subtotal_minor_units: cart.subtotal_minor_units,
                currency_code: cart.currency_code.clone(),
            },
            can_place_order: place_order_disabled_reason.is_none(),
            place_order_disabled_reason,
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

        if let Some(disabled_reason) = checkout.place_order_disabled_reason {
            return Err(AppSqliteError::InvalidProjection {
                reason: buyer_checkout_disabled_error(disabled_reason),
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
                let listing_relays_json = encode_listing_relays(&line.listing.listing_relays)?;
                self.connection
                    .execute(
                        "insert into order_lines (
                            id,
                            order_id,
                            title,
                            quantity_value,
                            quantity_unit_label,
                            quantity_display,
                            listing_bin_id,
                            unit_price_minor_units,
                            price_currency,
                            farm_key,
                            listing_addr,
                            listing_event_id,
                            listing_relays_json,
                            seller_pubkey,
                            sort_index
                         ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                        params![
                            format!("{}:{}", order_id, line.listing.product_id),
                            order_id.to_string(),
                            line.listing.title,
                            i64::from(line.quantity),
                            line.listing.unit_label.as_str(),
                            format_quantity_display(line.quantity, &line.listing.unit_label),
                            line.listing.listing_bin_id.as_deref(),
                            line.listing.price_minor_units,
                            normalize_currency_code(&line.listing.price_currency),
                            line.listing.farm_key.as_deref(),
                            line.listing.listing_addr.as_deref(),
                            line.listing.listing_event_id.as_deref(),
                            listing_relays_json.as_deref(),
                            line.listing.seller_pubkey.as_deref(),
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
            self.insert_pending_buyer_order_coordination(context_key.as_str(), order_id)?;

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

    pub fn load_buyer_order_coordination_record(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
    ) -> Result<Option<BuyerOrderCoordinationRecord>, AppSqliteError> {
        let context_key = context.storage_key();

        self.connection
            .query_row(
                "select
                    order_id,
                    buyer_context_key,
                    record_id,
                    state,
                    payload_json,
                    attempt_count,
                    last_error_message
                 from buyer_order_coordination_records
                 where buyer_context_key = ?1 and order_id = ?2
                 limit 1",
                params![context_key.as_str(), order_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, Option<String>>(6)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load buyer order coordination record",
                source,
            })?
            .map(buyer_order_coordination_record_from_row)
            .transpose()
    }

    pub fn load_recoverable_buyer_order_coordination_records(
        &self,
        context: &BuyerContext,
    ) -> Result<Vec<BuyerOrderCoordinationRecord>, AppSqliteError> {
        let context_key = context.storage_key();
        let mut statement = self
            .connection
            .prepare(
                "select
                    order_id,
                    buyer_context_key,
                    record_id,
                    state,
                    payload_json,
                    attempt_count,
                    last_error_message
                 from buyer_order_coordination_records
                 where buyer_context_key = ?1 and state in ('pending', 'failed')
                 order by updated_at asc, order_id asc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare recoverable buyer order coordination records",
                source,
            })?;
        let rows = statement
            .query_map(params![context_key.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query recoverable buyer order coordination records",
                source,
            })?;

        rows.map(|row| {
            buyer_order_coordination_record_from_row(row.map_err(|source| {
                AppSqliteError::Query {
                    operation: "read recoverable buyer order coordination record",
                    source,
                }
            })?)
        })
        .collect()
    }

    pub fn buyer_order_coordination_is_synced(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
    ) -> Result<bool, AppSqliteError> {
        Ok(self
            .load_buyer_order_coordination_record(context, order_id)?
            .is_some_and(|record| record.state == BuyerOrderCoordinationState::Synced))
    }

    pub fn prepare_buyer_order_coordination_attempt(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
        record_id: &str,
        payload_json: &str,
    ) -> Result<bool, AppSqliteError> {
        let context_key = context.storage_key();
        let changed = self
            .connection
            .execute(
                "insert into buyer_order_coordination_records (
                    order_id,
                    buyer_context_key,
                    record_id,
                    state,
                    payload_json,
                    attempt_count,
                    last_error_message,
                    created_at,
                    updated_at,
                    synced_at
                 ) values (
                    ?1,
                    ?2,
                    ?3,
                    'pending',
                    ?4,
                    1,
                    null,
                    strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                    null
                 )
                 on conflict(order_id) do update set
                    record_id = excluded.record_id,
                    state = 'pending',
                    payload_json = excluded.payload_json,
                    attempt_count = buyer_order_coordination_records.attempt_count + 1,
                    last_error_message = null,
                    updated_at = excluded.updated_at,
                    synced_at = null
                 where buyer_order_coordination_records.buyer_context_key = excluded.buyer_context_key
                    and buyer_order_coordination_records.state <> 'synced'",
                params![
                    order_id.to_string(),
                    context_key.as_str(),
                    record_id,
                    payload_json
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare buyer order coordination attempt",
                source,
            })?;

        Ok(changed == 1)
    }

    pub fn mark_buyer_order_coordination_synced(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
    ) -> Result<bool, AppSqliteError> {
        let context_key = context.storage_key();
        let changed = self
            .connection
            .execute(
                "update buyer_order_coordination_records
                 set
                    state = 'synced',
                    last_error_message = null,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                    synced_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 where buyer_context_key = ?1 and order_id = ?2",
                params![context_key.as_str(), order_id.to_string()],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "mark buyer order coordination synced",
                source,
            })?;

        Ok(changed == 1)
    }

    pub fn mark_buyer_order_coordination_failed(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
        error_message: &str,
    ) -> Result<bool, AppSqliteError> {
        let context_key = context.storage_key();
        let changed = self
            .connection
            .execute(
                "update buyer_order_coordination_records
                 set
                    state = 'failed',
                    last_error_message = ?3,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                    synced_at = null
                 where buyer_context_key = ?1 and order_id = ?2",
                params![context_key.as_str(), order_id.to_string(), error_message],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "mark buyer order coordination failed",
                source,
            })?;

        Ok(changed == 1)
    }

    pub fn load_buyer_orders(
        &self,
        context: &BuyerContext,
    ) -> Result<BuyerOrdersProjection, AppSqliteError> {
        let now_utc = self.current_utc_timestamp()?;
        let visible_listings = self.visible_listing_index(&now_utc)?;
        let context_key = context.storage_key();
        let mut statement = self
            .connection
            .prepare(
                "select
                    o.id,
                    o.farm_id,
                    o.order_number,
                    o.status,
                    o.workflow_revision,
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
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
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
                workflow_revision,
                farm_display_name,
                fulfillment_label,
                fulfillment_starts_at,
                fulfillment_ends_at,
            ) = row.map_err(|source| AppSqliteError::Query {
                operation: "read buyer orders list",
                source,
            })?;
            let order_id = parse_typed_id("orders.id", order_id)?;
            let farm_id = parse_typed_id("orders.farm_id", farm_id)?;
            let buyer_status = BuyerOrderStatus::from(parse_order_status("orders.status", status)?);

            orders.push(BuyerOrdersListRow {
                order_id,
                farm_id,
                order_number,
                repeat_demand: self.build_repeat_demand_handoff(
                    order_id,
                    farm_id,
                    farm_display_name.as_str(),
                    &visible_listings,
                )?,
                farm_display_name,
                fulfillment_summary: format_fulfillment_summary(
                    fulfillment_label,
                    fulfillment_starts_at,
                    fulfillment_ends_at,
                ),
                status: buyer_status,
                workflow: TradeWorkflowProjection::from_buyer_order_status(order_id, buyer_status)
                    .with_revision(TradeRevisionStatus::from_storage_key(
                        workflow_revision.as_str(),
                    )),
            });
        }

        Ok(BuyerOrdersProjection { rows: orders })
    }

    pub fn load_buyer_order_detail(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
    ) -> Result<Option<BuyerOrderDetailProjection>, AppSqliteError> {
        let now_utc = self.current_utc_timestamp()?;
        let visible_listings = self.visible_listing_index(&now_utc)?;
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
                    o.workflow_revision,
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
                        row.get::<_, String>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                        row.get::<_, Option<String>>(9)?,
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
                    workflow_revision,
                    farm_display_name,
                    fulfillment_label,
                    fulfillment_starts_at,
                    fulfillment_ends_at,
                )| {
                    let order_id: OrderId = parse_typed_id("orders.id", order_id)?;
                    let farm_id: FarmId = parse_typed_id("orders.farm_id", farm_id)?;
                    let status =
                        BuyerOrderStatus::from(parse_order_status("orders.status", status)?);
                    let items = self.load_order_detail_items(order_id.to_string())?;
                    let economics = order_detail_economics(&items)?;
                    let payment = TradePaymentDisplayStatus::NotRecorded;
                    let workflow =
                        TradeWorkflowProjection::from_buyer_order_status(order_id, status)
                            .with_revision(TradeRevisionStatus::from_storage_key(
                                workflow_revision.as_str(),
                            ))
                            .with_economics_and_payment(economics.clone(), payment);
                    Ok(BuyerOrderDetailProjection {
                        order_id,
                        farm_id,
                        order_number,
                        farm_display_name: farm_display_name.clone(),
                        fulfillment_summary: format_fulfillment_summary(
                            fulfillment_label,
                            fulfillment_starts_at,
                            fulfillment_ends_at,
                        ),
                        status,
                        items,
                        economics,
                        payment,
                        workflow,
                        order_note: empty_string_to_none(order_note),
                        repeat_demand: self.build_repeat_demand_handoff(
                            order_id,
                            farm_id,
                            farm_display_name.as_str(),
                            &visible_listings,
                        )?,
                    })
                },
            )
            .transpose()
    }

    pub fn load_buyer_order_local_event_export(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
    ) -> Result<Option<BuyerOrderLocalEventExport>, AppSqliteError> {
        let context_key = context.storage_key();
        let Some(record) = self
            .connection
            .query_row(
                "select
                    o.id,
                    o.farm_id,
                    o.order_number,
                    o.status,
                    o.buyer_context_key,
                    o.customer_display_name,
                    o.buyer_email,
                    o.buyer_phone,
                    o.buyer_order_note,
                    o.updated_at,
                    f.display_name,
                    fw.id,
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
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, String>(10)?,
                        row.get::<_, Option<String>>(11)?,
                        row.get::<_, Option<String>>(12)?,
                        row.get::<_, Option<String>>(13)?,
                        row.get::<_, Option<String>>(14)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load buyer order local event export header",
                source,
            })?
        else {
            return Ok(None);
        };
        let (
            order_id,
            farm_id,
            order_number,
            status,
            buyer_context_key,
            buyer_name,
            buyer_email,
            buyer_phone,
            buyer_order_note,
            updated_at,
            farm_display_name,
            fulfillment_window_id,
            fulfillment_window_label,
            fulfillment_starts_at,
            fulfillment_ends_at,
        ) = record;
        let order_id = parse_typed_id("orders.id", order_id)?;
        let farm_id = parse_typed_id("orders.farm_id", farm_id)?;
        let lines = self.load_buyer_order_local_event_lines(order_id)?;

        Ok(Some(BuyerOrderLocalEventExport {
            order_id,
            farm_id,
            farm_display_name,
            order_number,
            status,
            buyer_context_key: buyer_context_key.unwrap_or_else(|| context_key.clone()),
            buyer_name,
            buyer_email,
            buyer_phone,
            buyer_order_note,
            updated_at,
            fulfillment_window_id: parse_optional_typed_id(
                "orders.fulfillment_window_id",
                fulfillment_window_id,
            )?,
            fulfillment_window_label: empty_string_to_none_option(fulfillment_window_label),
            fulfillment_starts_at,
            fulfillment_ends_at,
            lines,
        }))
    }

    pub fn apply_buyer_repeat_demand_to_cart(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
        replace_existing: bool,
    ) -> Result<BuyerRepeatDemandApplyOutcome, AppSqliteError> {
        let Some((farm_id, farm_display_name)) =
            self.load_buyer_order_repeat_demand_header(context, order_id)?
        else {
            return Ok(BuyerRepeatDemandApplyOutcome::Unavailable);
        };
        let now_utc = self.current_utc_timestamp()?;
        let visible_listings = self.visible_listing_index(&now_utc)?;
        let Some(candidate) = self.build_repeat_demand_candidate(
            order_id,
            farm_id,
            farm_display_name.as_str(),
            &visible_listings,
        )?
        else {
            return Ok(BuyerRepeatDemandApplyOutcome::Unavailable);
        };
        if candidate.available_lines.is_empty() {
            return Ok(BuyerRepeatDemandApplyOutcome::Unavailable);
        }

        let current_cart = self.load_buyer_cart(context)?;
        if !replace_existing
            && !current_cart.is_empty()
            && current_cart.farm_id != Some(candidate.farm_id)
        {
            let current_farm_display_name = current_cart
                .farm_display_name
                .clone()
                .or_else(|| {
                    current_cart
                        .lines
                        .first()
                        .map(|line| line.farm_display_name.clone())
                })
                .ok_or(AppSqliteError::InvalidProjection {
                    reason: "buyer cart farm display name is missing",
                })?;

            return Ok(BuyerRepeatDemandApplyOutcome::ConfirmationRequired(
                BuyerCartReplaceConfirmationProjection {
                    current_farm_display_name,
                    incoming_farm_display_name: candidate.farm_display_name,
                },
            ));
        }

        let next_cart = next_buyer_cart_for_repeat_demand(
            current_cart,
            candidate.farm_id,
            candidate.farm_display_name.as_str(),
            &candidate.available_lines,
            replace_existing,
        )?;
        self.replace_buyer_cart(context, &next_cart)?;

        Ok(BuyerRepeatDemandApplyOutcome::Applied)
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

    fn insert_pending_buyer_order_coordination(
        &self,
        context_key: &str,
        order_id: OrderId,
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "insert into buyer_order_coordination_records (
                    order_id,
                    buyer_context_key,
                    state,
                    attempt_count,
                    created_at,
                    updated_at
                 ) values (
                    ?1,
                    ?2,
                    'pending',
                    0,
                    strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 )",
                params![order_id.to_string(), context_key],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "insert pending buyer order coordination record",
                source,
            })?;

        Ok(())
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
                    p.listing_bin_id,
                    (
                        select li.farm_key
                        from local_interop_imports li
                        where li.projected_kind = 'listing'
                           and li.projected_id = p.id
                        order by li.local_seq desc
                        limit 1
                    ),
                    (
                        select li.listing_addr
                        from local_interop_imports li
                        where li.projected_kind = 'listing'
                           and li.projected_id = p.id
                           and li.listing_addr is not null
                           and trim(li.listing_addr) <> ''
                        order by li.local_seq desc
                        limit 1
                    ),
                    (
                        select li.event_id
                        from local_interop_imports li
                        where li.projected_kind = 'listing'
                           and li.projected_id = p.id
                           and li.event_id is not null
                           and trim(li.event_id) <> ''
                        order by li.local_seq desc
                        limit 1
                    ),
                    (
                        select li.owner_pubkey
                        from local_interop_imports li
                        where li.projected_kind = 'listing'
                           and li.projected_id = p.id
                           and li.owner_pubkey is not null
                           and trim(li.owner_pubkey) <> ''
                        order by li.local_seq desc
                        limit 1
                    ),
                    (
                        select li.relay_delivery_json
                        from local_interop_imports li
                        where li.projected_kind = 'listing'
                           and li.projected_id = p.id
                           and li.relay_delivery_json is not null
                           and trim(li.relay_delivery_json) <> ''
                        order by li.local_seq desc
                        limit 1
                    ),
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
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, Option<String>>(15)?,
                    row.get::<_, Option<u32>>(16)?,
                    row.get::<_, Option<String>>(17)?,
                    row.get::<_, Option<String>>(18)?,
                    row.get::<_, Option<String>>(19)?,
                    row.get::<_, Option<String>>(20)?,
                    row.get::<_, Option<String>>(21)?,
                    row.get::<_, i64>(22)?,
                    row.get::<_, i64>(23)?,
                    row.get::<_, i64>(24)?,
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
                listing_bin_id,
                farm_key,
                listing_addr,
                listing_event_id,
                seller_pubkey,
                listing_relay_delivery_json,
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
                listing_bin_id: listing_bin_id.and_then(empty_string_to_none),
                farm_key: farm_key.and_then(empty_string_to_none),
                listing_addr: listing_addr.and_then(empty_string_to_none),
                listing_event_id: listing_event_id.and_then(empty_string_to_none),
                listing_relays: listing_relays_from_json(listing_relay_delivery_json)?,
                seller_pubkey: seller_pubkey.and_then(empty_string_to_none),
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

    fn load_buyer_cart_line_snapshot(
        &self,
        product_id: ProductId,
    ) -> Result<BuyerCartLineSnapshot, AppSqliteError> {
        Ok(self
            .load_listing_record_by_id(product_id)?
            .map(|listing| BuyerCartLineSnapshot {
                listing_bin_id: listing.listing_bin_id,
                farm_key: listing.farm_key,
                listing_addr: listing.listing_addr,
                listing_event_id: listing.listing_event_id,
                listing_relays: listing.listing_relays,
                seller_pubkey: listing.seller_pubkey,
            })
            .unwrap_or_default())
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
                    coalesce(nullif(bcl.quantity_unit_label, ''), p.unit_label),
                    coalesce(bcl.unit_price_minor_units, p.price_minor_units),
                    coalesce(nullif(bcl.price_currency, ''), p.price_currency),
                    coalesce(nullif(bcl.listing_bin_id, ''), p.listing_bin_id),
                    coalesce(nullif(bcl.farm_key, ''), (
                        select li.farm_key
                        from local_interop_imports li
                        where li.projected_kind = 'listing'
                           and li.projected_id = p.id
                        order by li.local_seq desc
                        limit 1
                    )),
                    coalesce(nullif(bcl.listing_addr, ''), (
                        select li.listing_addr
                        from local_interop_imports li
                        where li.projected_kind = 'listing'
                           and li.projected_id = p.id
                           and li.listing_addr is not null
                           and trim(li.listing_addr) <> ''
                        order by li.local_seq desc
                        limit 1
                    )),
                    coalesce(nullif(bcl.listing_event_id, ''), (
                        select li.event_id
                        from local_interop_imports li
                        where li.projected_kind = 'listing'
                           and li.projected_id = p.id
                           and li.event_id is not null
                           and trim(li.event_id) <> ''
                        order by li.local_seq desc
                        limit 1
                    )),
                    coalesce(nullif(bcl.listing_relays_json, ''), (
                        select li.relay_delivery_json
                        from local_interop_imports li
                        where li.projected_kind = 'listing'
                           and li.projected_id = p.id
                           and li.relay_delivery_json is not null
                           and trim(li.relay_delivery_json) <> ''
                        order by li.local_seq desc
                        limit 1
                    )),
                    coalesce(nullif(bcl.seller_pubkey, ''), (
                        select li.owner_pubkey
                        from local_interop_imports li
                        where li.projected_kind = 'listing'
                           and li.projected_id = p.id
                           and li.owner_pubkey is not null
                           and trim(li.owner_pubkey) <> ''
                        order by li.local_seq desc
                        limit 1
                    )),
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
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, Option<String>>(15)?,
                    row.get::<_, Option<String>>(16)?,
                    row.get::<_, Option<u32>>(17)?,
                    row.get::<_, Option<String>>(18)?,
                    row.get::<_, Option<String>>(19)?,
                    row.get::<_, Option<String>>(20)?,
                    row.get::<_, Option<String>>(21)?,
                    row.get::<_, Option<String>>(22)?,
                    row.get::<_, i64>(23)?,
                    row.get::<_, i64>(24)?,
                    row.get::<_, i64>(25)?,
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
                listing_bin_id,
                farm_key,
                listing_addr,
                listing_event_id,
                listing_relays_json,
                seller_pubkey,
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
                listing_bin_id: listing_bin_id.and_then(empty_string_to_none),
                farm_key: farm_key.and_then(empty_string_to_none),
                listing_addr: listing_addr.and_then(empty_string_to_none),
                listing_event_id: listing_event_id.and_then(empty_string_to_none),
                listing_relays: listing_relays_from_json(listing_relays_json)?,
                seller_pubkey: seller_pubkey.and_then(empty_string_to_none),
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
                "select
                    title,
                    quantity_display,
                    quantity_value,
                    quantity_unit_label,
                    unit_price_minor_units,
                    price_currency
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
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<u32>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query buyer order detail items",
                source,
            })?;
        let mut items = Vec::new();

        for row in rows {
            let (
                title,
                quantity_display,
                quantity_value,
                quantity_unit_label,
                unit_price_minor_units,
                price_currency,
            ) = row.map_err(|source| AppSqliteError::Query {
                operation: "read buyer order detail items",
                source,
            })?;
            items.push(order_detail_item_row(
                title,
                quantity_display,
                quantity_value,
                quantity_unit_label,
                unit_price_minor_units,
                price_currency,
            )?);
        }

        Ok(items)
    }

    fn load_buyer_order_local_event_lines(
        &self,
        order_id: OrderId,
    ) -> Result<Vec<BuyerOrderLocalEventLine>, AppSqliteError> {
        let order_id_string = order_id.to_string();
        let mut statement = self
            .connection
            .prepare(
                "select
                    ol.id,
                    ol.title,
                    ol.quantity_value,
                    ol.quantity_unit_label,
                    ol.quantity_display,
                    ol.unit_price_minor_units,
                    ol.price_currency,
                    ol.listing_bin_id,
                    ol.farm_key,
                    ol.listing_addr,
                    ol.listing_event_id,
                    ol.listing_relays_json,
                    ol.seller_pubkey
                 from order_lines ol
                 where ol.order_id = ?1
                 order by ol.sort_index asc, ol.id asc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare buyer order local event lines",
                source,
            })?;
        let rows = statement
            .query_map(params![order_id_string.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<u32>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                ))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query buyer order local event lines",
                source,
            })?;
        let mut lines = Vec::new();

        for row in rows {
            let (
                line_id,
                title,
                quantity,
                quantity_unit_label,
                quantity_display,
                unit_price_minor_units,
                price_currency,
                listing_bin_id,
                farm_key,
                listing_addr,
                listing_event_id,
                listing_relays_json,
                seller_pubkey,
            ) = row.map_err(|source| AppSqliteError::Query {
                operation: "read buyer order local event line",
                source,
            })?;
            let product_id = parse_order_line_product_id(line_id.as_str(), order_id)?;
            let quantity =
                u32::try_from(quantity).map_err(|_| AppSqliteError::InvalidProjection {
                    reason: "buyer order local event quantity must be non-negative",
                })?;
            if quantity == 0 {
                return Err(AppSqliteError::InvalidProjection {
                    reason: "buyer order local event quantity must be positive",
                });
            }

            lines.push(BuyerOrderLocalEventLine {
                product_id,
                title,
                quantity,
                quantity_unit_label,
                quantity_display,
                unit_price_minor_units,
                price_currency: price_currency.unwrap_or_else(|| "USD".to_owned()),
                listing_bin_id: listing_bin_id.and_then(empty_string_to_none),
                farm_key: farm_key.and_then(empty_string_to_none),
                listing_addr: listing_addr.and_then(empty_string_to_none),
                listing_event_id: listing_event_id.and_then(empty_string_to_none),
                listing_relays: listing_relays_from_json(listing_relays_json)?,
                seller_pubkey: seller_pubkey.and_then(empty_string_to_none),
            });
        }

        Ok(lines)
    }

    fn load_repeat_demand_order_lines(
        &self,
        order_id: OrderId,
    ) -> Result<Vec<RepeatDemandOrderLine>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select id, quantity_value, listing_addr
                 from order_lines
                 where order_id = ?1
                 order by sort_index asc, id asc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare repeat demand order lines",
                source,
            })?;
        let rows = statement
            .query_map(params![order_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query repeat demand order lines",
                source,
            })?;
        let mut order_lines = Vec::new();

        for row in rows {
            let (line_id, quantity_value, listing_addr) =
                row.map_err(|source| AppSqliteError::Query {
                    operation: "read repeat demand order lines",
                    source,
                })?;
            let product_id = parse_repeat_demand_product_id(line_id.as_str())?;
            let quantity =
                u32::try_from(quantity_value).map_err(|_| AppSqliteError::InvalidProjection {
                    reason: "repeat demand quantity must be non-negative",
                })?;
            if quantity == 0 {
                return Err(AppSqliteError::InvalidProjection {
                    reason: "repeat demand quantity must be positive",
                });
            }

            order_lines.push(RepeatDemandOrderLine {
                product_id,
                quantity,
                listing_addr: listing_addr.and_then(empty_string_to_none),
            });
        }

        Ok(order_lines)
    }

    fn load_buyer_order_repeat_demand_header(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
    ) -> Result<Option<(FarmId, String)>, AppSqliteError> {
        let context_key = context.storage_key();

        self.connection
            .query_row(
                "select o.farm_id, f.display_name
                 from orders o
                 inner join farms f on f.id = o.farm_id
                 where o.buyer_context_key = ?1 and o.id = ?2
                 limit 1",
                params![context_key.as_str(), order_id.to_string()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load buyer repeat demand header",
                source,
            })?
            .map(|(farm_id, farm_display_name)| {
                Ok((
                    parse_typed_id("orders.farm_id", farm_id)?,
                    farm_display_name,
                ))
            })
            .transpose()
    }

    fn visible_listing_index(&self, now_utc: &str) -> Result<VisibleListingIndex, AppSqliteError> {
        Ok(VisibleListingIndex::from_records(
            self.load_listing_records()?
                .into_iter()
                .filter(|record| record.is_buyer_visible(now_utc)),
        ))
    }

    fn build_repeat_demand_handoff(
        &self,
        order_id: OrderId,
        farm_id: FarmId,
        farm_display_name: &str,
        visible_listings: &VisibleListingIndex,
    ) -> Result<Option<RepeatDemandHandoffProjection>, AppSqliteError> {
        Ok(self
            .build_repeat_demand_candidate(order_id, farm_id, farm_display_name, visible_listings)?
            .map(|candidate| candidate.handoff))
    }

    fn build_repeat_demand_candidate(
        &self,
        order_id: OrderId,
        farm_id: FarmId,
        farm_display_name: &str,
        visible_listings: &VisibleListingIndex,
    ) -> Result<Option<RepeatDemandCandidate>, AppSqliteError> {
        let order_lines = self.load_repeat_demand_order_lines(order_id)?;
        if order_lines.is_empty() {
            return Ok(None);
        }

        let mut available_lines = Vec::new();
        let mut unavailable_item_count = 0u32;

        for order_line in &order_lines {
            if let Some(listing) = visible_listings.resolve(order_line).filter(|listing| {
                listing
                    .stock_count
                    .is_some_and(|quantity| quantity >= order_line.quantity)
            }) {
                available_lines.push(BuyerCartLineRecord {
                    listing,
                    quantity: order_line.quantity,
                });
            } else {
                unavailable_item_count = unavailable_item_count.checked_add(1).ok_or(
                    AppSqliteError::InvalidProjection {
                        reason: "repeat demand unavailable count overflowed",
                    },
                )?;
            }
        }

        let available_item_count = available_lines.len() as u32;
        let eligibility = if available_item_count == 0 {
            RepeatDemandEligibility::Unavailable
        } else if unavailable_item_count == 0 {
            RepeatDemandEligibility::Eligible
        } else {
            RepeatDemandEligibility::Partial
        };
        let current_farm = available_lines
            .first()
            .map(|line| (line.listing.farm_id, line.listing.farm_display_name.clone()))
            .unwrap_or_else(|| (farm_id, farm_display_name.to_owned()));
        let (current_farm_id, current_farm_display_name) = current_farm;

        Ok(Some(RepeatDemandCandidate {
            farm_id: current_farm_id,
            farm_display_name: current_farm_display_name,
            available_lines,
            handoff: RepeatDemandHandoffProjection {
                order_id,
                farm_id: current_farm_id,
                eligibility,
                available_item_count,
                unavailable_item_count,
            },
        }))
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
    listing_bin_id: Option<String>,
    farm_key: Option<String>,
    listing_addr: Option<String>,
    listing_event_id: Option<String>,
    listing_relays: Vec<String>,
    seller_pubkey: Option<String>,
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
            listing_relays: self.listing_relays,
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct VisibleListingIndex {
    by_product_id: BTreeMap<ProductId, BuyerListingRecord>,
    by_listing_addr: BTreeMap<String, BuyerListingRecord>,
}

impl VisibleListingIndex {
    fn from_records(records: impl IntoIterator<Item = BuyerListingRecord>) -> Self {
        let mut index = Self::default();
        for record in records {
            if let Some(listing_addr) = record.listing_addr.as_deref() {
                index
                    .by_listing_addr
                    .insert(listing_addr.to_owned(), record.clone());
            }
            index.by_product_id.insert(record.product_id, record);
        }
        index
    }

    fn resolve(&self, order_line: &RepeatDemandOrderLine) -> Option<BuyerListingRecord> {
        self.by_product_id
            .get(&order_line.product_id)
            .or_else(|| {
                order_line
                    .listing_addr
                    .as_deref()
                    .and_then(|listing_addr| self.by_listing_addr.get(listing_addr))
            })
            .cloned()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct BuyerCartLineSnapshot {
    listing_bin_id: Option<String>,
    farm_key: Option<String>,
    listing_addr: Option<String>,
    listing_event_id: Option<String>,
    listing_relays: Vec<String>,
    seller_pubkey: Option<String>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct RepeatDemandOrderLine {
    product_id: ProductId,
    quantity: u32,
    listing_addr: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RepeatDemandCandidate {
    farm_id: FarmId,
    farm_display_name: String,
    available_lines: Vec<BuyerCartLineRecord>,
    handoff: RepeatDemandHandoffProjection,
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

fn next_buyer_cart_for_repeat_demand(
    mut current_cart: BuyerCartProjection,
    farm_id: FarmId,
    farm_display_name: &str,
    lines: &[BuyerCartLineRecord],
    replace_existing: bool,
) -> Result<BuyerCartProjection, AppSqliteError> {
    if replace_existing || current_cart.is_empty() || current_cart.farm_id != Some(farm_id) {
        current_cart.lines.clear();
    }

    current_cart.farm_id = Some(farm_id);
    current_cart.farm_display_name = Some(farm_display_name.to_owned());
    current_cart.replace_confirmation = None;

    for line in lines {
        let incoming_line = line.clone().into_projection()?;

        if let Some(existing_line) = current_cart
            .lines
            .iter_mut()
            .find(|existing| existing.product_id == incoming_line.product_id)
        {
            existing_line.quantity = existing_line
                .quantity
                .checked_add(incoming_line.quantity)
                .ok_or(AppSqliteError::InvalidProjection {
                    reason: "buyer cart quantity overflow",
                })?;
            existing_line.line_total_minor_units = existing_line
                .unit_price
                .amount_minor_units
                .checked_mul(existing_line.quantity)
                .ok_or(AppSqliteError::InvalidProjection {
                    reason: "buyer cart line total overflow",
                })?;
            existing_line.fulfillment_summary = incoming_line.fulfillment_summary.clone();
            continue;
        }

        current_cart.lines.push(incoming_line);
    }

    refresh_buyer_cart_summary(&mut current_cart)?;

    Ok(current_cart)
}

fn parse_repeat_demand_product_id(line_id: &str) -> Result<ProductId, AppSqliteError> {
    let Some((_, product_id)) = line_id.rsplit_once(':') else {
        return Err(AppSqliteError::InvalidProjection {
            reason: "repeat demand order line is missing a product id",
        });
    };

    parse_typed_id("order_lines.id", product_id.to_owned())
}

fn parse_order_line_product_id(
    line_id: &str,
    order_id: OrderId,
) -> Result<ProductId, AppSqliteError> {
    let order_id = order_id.to_string();
    let prefix = format!("{order_id}:");
    let Some(product_id) = line_id.strip_prefix(prefix.as_str()) else {
        return Err(AppSqliteError::InvalidProjection {
            reason: "buyer order local event line is missing its order id prefix",
        });
    };

    parse_typed_id("order_lines.id", product_id.to_owned())
}

fn refresh_buyer_cart_summary(cart: &mut BuyerCartProjection) -> Result<(), AppSqliteError> {
    if cart.lines.is_empty() {
        cart.subtotal_minor_units = None;
        cart.currency_code = None;
        cart.replace_confirmation = None;
        return Ok(());
    }

    let mut subtotal_minor_units = 0_u32;
    let mut currency_code = None;

    for line in &cart.lines {
        subtotal_minor_units = subtotal_minor_units
            .checked_add(line.line_total_minor_units)
            .ok_or(AppSqliteError::InvalidProjection {
                reason: "buyer cart subtotal overflowed",
            })?;
        currency_code.get_or_insert_with(|| line.unit_price.currency_code.clone());
    }

    cart.subtotal_minor_units = Some(subtotal_minor_units);
    cart.currency_code = Some(currency_code.unwrap_or_default());

    Ok(())
}

fn shared_fulfillment_summary(lines: &[BuyerCartLineProjection]) -> Option<String> {
    let first = lines.first()?.fulfillment_summary.clone();

    lines
        .iter()
        .all(|line| line.fulfillment_summary == first)
        .then_some(first)
}

fn buyer_checkout_disabled_reason(
    context: &BuyerContext,
    cart: &BuyerCartProjection,
    fulfillment_summary: Option<&String>,
    draft: &BuyerCheckoutDraft,
) -> Option<BuyerCheckoutDisabledReason> {
    if cart.lines.is_empty() {
        return Some(BuyerCheckoutDisabledReason::EmptyCart);
    }
    if fulfillment_summary.is_none() {
        return Some(BuyerCheckoutDisabledReason::MissingFulfillment);
    }
    if draft.name.trim().is_empty() {
        return Some(BuyerCheckoutDisabledReason::MissingName);
    }
    if draft.email.trim().is_empty() {
        return Some(BuyerCheckoutDisabledReason::MissingEmail);
    }
    if matches!(context, BuyerContext::Guest) {
        return Some(BuyerCheckoutDisabledReason::AccountRequired);
    }
    None
}

fn buyer_checkout_disabled_error(reason: BuyerCheckoutDisabledReason) -> &'static str {
    match reason {
        BuyerCheckoutDisabledReason::EmptyCart => "buyer checkout cart is empty",
        BuyerCheckoutDisabledReason::MissingFulfillment => {
            "buyer checkout fulfillment is unavailable"
        }
        BuyerCheckoutDisabledReason::MissingName => "buyer checkout buyer name is missing",
        BuyerCheckoutDisabledReason::MissingEmail => "buyer checkout buyer email is missing",
        BuyerCheckoutDisabledReason::AccountRequired => {
            "buyer checkout requires a selected account"
        }
    }
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

fn buyer_order_coordination_record_from_row(
    row: (
        String,
        String,
        Option<String>,
        String,
        Option<String>,
        i64,
        Option<String>,
    ),
) -> Result<BuyerOrderCoordinationRecord, AppSqliteError> {
    let (order_id, buyer_context_key, record_id, state, payload_json, attempt_count, last_error) =
        row;
    let attempt_count =
        u32::try_from(attempt_count).map_err(|_| AppSqliteError::InvalidProjection {
            reason: "buyer order coordination attempt count must be non-negative",
        })?;

    Ok(BuyerOrderCoordinationRecord {
        order_id: parse_typed_id("buyer_order_coordination_records.order_id", order_id)?,
        buyer_context_key,
        record_id: record_id.and_then(empty_string_to_none),
        state: BuyerOrderCoordinationState::from_storage_key(
            "buyer_order_coordination_records.state",
            state,
        )?,
        payload_json: payload_json.and_then(empty_string_to_none),
        attempt_count,
        last_error_message: last_error.and_then(empty_string_to_none),
    })
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
        "declined" => Ok(OrderStatus::Declined),
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

fn encode_listing_relays(relays: &[String]) -> Result<Option<String>, AppSqliteError> {
    let relays = normalized_listing_relays(relays.iter().map(String::as_str));
    if relays.is_empty() {
        return Ok(None);
    }

    serde_json::to_string(&relays)
        .map(Some)
        .map_err(|_| AppSqliteError::InvalidProjection {
            reason: "listing relay provenance must encode",
        })
}

fn listing_relays_from_json(value: Option<String>) -> Result<Vec<String>, AppSqliteError> {
    let Some(value) = empty_string_to_none_option(value) else {
        return Ok(Vec::new());
    };
    let value = serde_json::from_str::<Value>(value.as_str()).map_err(|_| {
        AppSqliteError::InvalidProjection {
            reason: "listing relay provenance json must decode",
        }
    })?;
    if let Some(relays) = value.as_array() {
        return Ok(relays_from_json_array(relays));
    }

    let relay_key = match value.get("state").and_then(Value::as_str) {
        Some("acknowledged") => Some("acknowledged_relays"),
        Some("observed") => Some("observed_relays"),
        _ => None,
    };
    if let Some(key) = relay_key {
        return Ok(value
            .get(key)
            .and_then(Value::as_array)
            .map(|relays| relays_from_json_array(relays))
            .unwrap_or_default());
    }

    Ok(Vec::new())
}

fn relays_from_json_array(relays: &[Value]) -> Vec<String> {
    normalized_listing_relays(relays.iter().filter_map(Value::as_str))
}

fn normalized_listing_relays<'a>(relays: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for relay in relays {
        let relay = relay.trim();
        if !relay.is_empty() && seen.insert(relay.to_owned()) {
            normalized.push(relay.to_owned());
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use radroots_studio_app_view::{
        BuyerCheckoutDisabledReason, BuyerContext, FarmId, FarmOrderMethod, FulfillmentWindowId,
        OrderId, PickupLocationId, ProductId, TradePaymentDisplayStatus,
    };
    use rusqlite::{Connection, params};
    use serde_json::json;

    use crate::{AppSqliteError, AppSqliteStore, BuyerRepeatDemandApplyOutcome, DatabaseTarget};

    use super::AppBuyerRepository;

    #[test]
    fn listing_relays_from_json_uses_only_acknowledged_or_observed_relays() {
        assert_eq!(
            super::listing_relays_from_json(Some(
                json!({
                    "state": "acknowledged",
                    "target_relays": ["wss://target.example"],
                    "connected_relays": ["wss://connected.example"],
                    "acknowledged_relays": ["wss://ack.example"]
                })
                .to_string()
            ))
            .expect("acknowledged relays"),
            vec!["wss://ack.example"]
        );
        assert_eq!(
            super::listing_relays_from_json(Some(
                json!({
                    "state": "observed",
                    "target_relays": ["wss://target.example"],
                    "connected_relays": ["wss://connected.example"],
                    "observed_relays": ["wss://observed.example"]
                })
                .to_string()
            ))
            .expect("observed relays"),
            vec!["wss://observed.example"]
        );
        assert!(
            super::listing_relays_from_json(Some(
                json!({
                    "state": "observed",
                    "target_relays": ["wss://target.example"],
                    "connected_relays": ["wss://connected.example"],
                    "observed_relays": []
                })
                .to_string()
            ))
            .expect("unknown observed relays")
            .is_empty()
        );
        assert!(
            super::listing_relays_from_json(Some(
                json!({
                    "state": "pending",
                    "target_relays": ["wss://target.example"],
                    "connected_relays": ["wss://connected.example"]
                })
                .to_string()
            ))
            .expect("pending relays")
            .is_empty()
        );
    }

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
    fn buyer_checkout_requires_account_before_order_write() {
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
                &radroots_studio_app_view::BuyerCartProjection {
                    farm_id: Some(farm_id),
                    farm_display_name: Some("Willow Farm".to_owned()),
                    lines: vec![radroots_studio_app_view::BuyerCartLineProjection {
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
                &radroots_studio_app_view::BuyerCheckoutDraft {
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
        let error = repository
            .place_buyer_order(&context)
            .expect_err("guest checkout should require an account");
        let cart_after_checkout = repository
            .load_buyer_cart(&context)
            .expect("buyer cart should remain after blocked checkout");

        assert!(matches!(error, AppSqliteError::InvalidProjection { .. }));
        assert!(!checkout.can_place_order);
        assert_eq!(
            checkout.place_order_disabled_reason,
            Some(BuyerCheckoutDisabledReason::AccountRequired)
        );
        assert_eq!(checkout.summary.line_count, 1);
        assert_eq!(cart_after_checkout.lines.len(), 1);
        assert_eq!(cart_after_checkout.farm_id, Some(farm_id));
        assert_eq!(row_count(connection, "orders"), 0);
        assert_eq!(row_count(connection, "order_lines"), 0);
        assert_eq!(row_count(connection, "buyer_order_coordination_records"), 0);
        assert_eq!(row_count(connection, "local_outbox"), 0);
        assert_eq!(row_count(connection, "local_conflicts"), 0);
        assert_eq!(row_count(connection, "sync_checkpoints"), 0);
    }

    #[test]
    fn buyer_order_history_derives_repeat_demand_from_current_listing_truth() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let repository = AppBuyerRepository::new(connection);
        let context = BuyerContext::account("acct_buyer");
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
        let available_product_id = insert_product(
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
        let unavailable_product_id = insert_product(
            connection,
            farm_id,
            SeedProduct {
                title: "Pea shoots",
                subtitle: "Tray-grown",
                status: "published",
                unit_label: "bag",
                price_minor_units: Some(450),
                price_currency: "USD",
                stock_count: Some(4),
                availability_window_id: Some(future_window_id),
            },
        );
        let available_listing = repository
            .load_buyer_product_detail(available_product_id)
            .expect("available buyer detail should load")
            .expect("available listing should exist")
            .listing;
        let unavailable_listing = repository
            .load_buyer_product_detail(unavailable_product_id)
            .expect("unavailable buyer detail should load")
            .expect("unavailable listing should exist")
            .listing;

        repository
            .replace_buyer_cart(
                &context,
                &radroots_studio_app_view::BuyerCartProjection {
                    farm_id: Some(farm_id),
                    farm_display_name: Some("Willow Farm".to_owned()),
                    lines: vec![
                        radroots_studio_app_view::BuyerCartLineProjection {
                            product_id: available_listing.product_id,
                            farm_id: available_listing.farm_id,
                            farm_display_name: available_listing.farm_display_name.clone(),
                            title: available_listing.title.clone(),
                            quantity: 2,
                            unit_price: available_listing.price.clone(),
                            line_total_minor_units: 1300,
                            fulfillment_summary: "Friday pickup".to_owned(),
                        },
                        radroots_studio_app_view::BuyerCartLineProjection {
                            product_id: unavailable_listing.product_id,
                            farm_id: unavailable_listing.farm_id,
                            farm_display_name: unavailable_listing.farm_display_name.clone(),
                            title: unavailable_listing.title.clone(),
                            quantity: 1,
                            unit_price: unavailable_listing.price.clone(),
                            line_total_minor_units: 450,
                            fulfillment_summary: "Friday pickup".to_owned(),
                        },
                    ],
                    subtotal_minor_units: Some(1750),
                    currency_code: Some("USD".to_owned()),
                    replace_confirmation: None,
                },
            )
            .expect("buyer cart should save");
        repository
            .save_buyer_checkout_draft(
                &context,
                &radroots_studio_app_view::BuyerCheckoutDraft {
                    name: "Casey Buyer".to_owned(),
                    email: "casey@example.com".to_owned(),
                    phone: String::new(),
                    order_note: String::new(),
                },
            )
            .expect("buyer checkout draft should save");
        let order_id = repository
            .place_buyer_order(&context)
            .expect("buyer checkout should place order");

        connection
            .execute(
                "update products set status = 'archived' where id = ?1",
                params![unavailable_product_id.to_string()],
            )
            .expect("product should archive");

        let buyer_orders = repository
            .load_buyer_orders(&context)
            .expect("buyer orders should load");
        let buyer_order_detail = repository
            .load_buyer_order_detail(&context, order_id)
            .expect("buyer order detail should load")
            .expect("buyer order detail should exist");
        let row_repeat_demand = buyer_orders.rows[0]
            .repeat_demand
            .as_ref()
            .expect("repeat demand should derive for buyer order row");
        let detail_repeat_demand = buyer_order_detail
            .repeat_demand
            .as_ref()
            .expect("repeat demand should derive for buyer order detail");

        assert_eq!(buyer_orders.rows.len(), 1);
        assert_eq!(
            row_repeat_demand.eligibility,
            radroots_studio_app_view::RepeatDemandEligibility::Partial
        );
        assert_eq!(row_repeat_demand.available_item_count, 1);
        assert_eq!(row_repeat_demand.unavailable_item_count, 1);
        assert_eq!(detail_repeat_demand, row_repeat_demand);
        assert_eq!(buyer_order_detail.items.len(), 2);
        assert!(
            buyer_order_detail
                .items
                .iter()
                .any(|item| item.line_total_minor_units == Some(1300))
        );
        assert!(
            buyer_order_detail
                .items
                .iter()
                .any(|item| item.line_total_minor_units == Some(450))
        );
        assert_eq!(buyer_order_detail.economics.total_minor_units, Some(1750));
        assert_eq!(
            buyer_order_detail.economics.currency_code.as_deref(),
            Some("USD")
        );
        assert_eq!(
            buyer_order_detail.payment,
            TradePaymentDisplayStatus::NotRecorded
        );
    }

    #[test]
    fn buyer_repeat_demand_requires_current_stock_for_full_historical_quantity() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let repository = AppBuyerRepository::new(connection);
        let context = BuyerContext::account("acct_buyer");
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
                &radroots_studio_app_view::BuyerCartProjection {
                    farm_id: Some(farm_id),
                    farm_display_name: Some("Willow Farm".to_owned()),
                    lines: vec![radroots_studio_app_view::BuyerCartLineProjection {
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
                &radroots_studio_app_view::BuyerCheckoutDraft {
                    name: "Casey Buyer".to_owned(),
                    email: "casey@example.com".to_owned(),
                    phone: String::new(),
                    order_note: String::new(),
                },
            )
            .expect("buyer checkout draft should save");
        let order_id = repository
            .place_buyer_order(&context)
            .expect("buyer checkout should place order");

        connection
            .execute(
                "update products set stock_count = 1 where id = ?1",
                params![product_id.to_string()],
            )
            .expect("product stock should lower");

        let repeat_demand = repository
            .load_buyer_order_detail(&context, order_id)
            .expect("buyer order detail should load")
            .expect("buyer order detail should exist")
            .repeat_demand
            .expect("repeat demand should stay visible for unavailable reorder");

        assert_eq!(
            repeat_demand.eligibility,
            radroots_studio_app_view::RepeatDemandEligibility::Unavailable
        );
        assert_eq!(repeat_demand.available_item_count, 0);
        assert_eq!(repeat_demand.unavailable_item_count, 1);
        assert_eq!(
            repository
                .apply_buyer_repeat_demand_to_cart(&context, order_id, false)
                .expect("repeat demand apply should load"),
            BuyerRepeatDemandApplyOutcome::Unavailable
        );
        assert!(
            repository
                .load_buyer_cart(&context)
                .expect("buyer cart should reload")
                .lines
                .is_empty()
        );
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
                &radroots_studio_app_view::BuyerCartProjection {
                    farm_id: Some(farm_id),
                    farm_display_name: Some("Willow Farm".to_owned()),
                    lines: vec![radroots_studio_app_view::BuyerCartLineProjection {
                        product_id: ProductId::new(),
                        farm_id: other_farm_id,
                        farm_display_name: "Other Farm".to_owned(),
                        title: "Mismatch".to_owned(),
                        quantity: 1,
                        unit_price: radroots_studio_app_view::ProductPricePresentation {
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

    fn row_count(connection: &Connection, table_name: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) FROM {table_name}");

        connection
            .query_row(&sql, [], |row| row.get(0))
            .expect("row count query should succeed")
    }
}
