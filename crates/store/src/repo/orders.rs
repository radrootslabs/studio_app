use std::collections::BTreeMap;

use radroots_studio_app_view::{
    FarmId, FulfillmentWindowId, FulfillmentWindowSummary, OrderDetailItemRow,
    OrderDetailProjection, OrderId, OrderPrimaryAction, OrderStatus, OrdersFilter,
    OrdersListProjection, OrdersListRow, OrdersListSummary, OrdersScreenQueryState,
    PackDayOutputCustomerOrder, PackDayOutputOrderState, PackDayOutputPackListEntry,
    PackDayOutputProductTotal, PackDayOutputQuantity, PackDayOutputSource, PackDayOutputWindow,
    PackDayPackListRow, PackDayProductTotalRow, PackDayProjection, PackDayRosterRow,
    PackDayScreenQueryState, ProductId, TradeAgreementStatus, TradeWorkflowProjection,
};
use rusqlite::{Connection, OptionalExtension, params};

use super::{
    order_detail::{order_detail_economics, order_detail_item_row, order_validation_receipts},
    parse_trade_revision_status,
    workflow::{StoredTradeWorkflowSnapshot, trade_workflow_projection_from_storage},
};
use crate::AppSqliteError;

pub struct AppOrdersRepository<'a> {
    connection: &'a Connection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SellerOrderDecisionExport {
    pub order_id: OrderId,
    pub farm_id: FarmId,
    pub status: OrderStatus,
    pub lines: Vec<SellerOrderDecisionLineExport>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SellerOrderDecisionLineExport {
    pub product_id: ProductId,
    pub listing_bin_id: Option<String>,
    pub quantity: u32,
    pub stock_count: Option<u32>,
    pub reserved_quantity: u32,
}

impl<'a> AppOrdersRepository<'a> {
    pub const fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub fn load_orders_list(
        &self,
        farm_id: FarmId,
        query: &OrdersScreenQueryState,
    ) -> Result<OrdersListProjection, AppSqliteError> {
        let mut records = self.load_order_records(farm_id, query.fulfillment_window_id)?;
        let summary = summarize_orders(&records);

        records.retain(|record| record.matches_filter(query.filter));

        Ok(OrdersListProjection {
            summary,
            rows: records
                .into_iter()
                .map(OrderRecord::into_list_row)
                .collect(),
        })
    }

    pub fn load_order_detail(
        &self,
        farm_id: FarmId,
        order_id: OrderId,
    ) -> Result<Option<OrderDetailProjection>, AppSqliteError> {
        let record = self
            .connection
            .query_row(
                "select
                    o.id,
                    o.farm_id,
                    o.order_number,
                    o.customer_display_name,
                    o.status,
                    o.fulfillment_window_id,
                    o.workflow_revision,
                    o.workflow_agreement,
                    o.workflow_inventory,
                    o.workflow_provenance_source,
                    o.workflow_provenance_last_event_id,
                    fw.label,
                    pl.label
                 from orders o
                 left join fulfillment_windows fw on fw.id = o.fulfillment_window_id
                 left join pickup_locations pl on pl.id = fw.pickup_location_id
                 where o.farm_id = ?1 and o.id = ?2
                 limit 1",
                params![farm_id.to_string(), order_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, Option<String>>(10)?,
                        row.get::<_, Option<String>>(11)?,
                        row.get::<_, Option<String>>(12)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load order detail",
                source,
            })?;

        record
            .map(
                |(
                    order_id,
                    farm_id,
                    order_number,
                    customer_display_name,
                    status,
                    fulfillment_window_id,
                    workflow_revision,
                    workflow_agreement,
                    workflow_inventory,
                    workflow_provenance_source,
                    workflow_provenance_last_event_id,
                    fulfillment_window_label,
                    pickup_location_label,
                )| {
                    let order_id: OrderId = parse_typed_id("orders.id", order_id)?;
                    let farm_id: FarmId = parse_typed_id("orders.farm_id", farm_id)?;
                    let status = parse_order_status("orders.status", status)?;
                    let revision =
                        parse_trade_revision_status("orders.workflow_revision", workflow_revision)?;
                    let items = self.load_order_detail_items(order_id.to_string())?;
                    let economics = order_detail_economics(&items)?;
                    let workflow =
                        trade_workflow_projection_from_storage(StoredTradeWorkflowSnapshot {
                            order_id,
                            revision,
                            economics: economics.clone(),
                            agreement: workflow_agreement,
                            inventory: workflow_inventory,
                            provenance_source: workflow_provenance_source,
                            provenance_last_event_id: workflow_provenance_last_event_id,
                        })?;
                    let validation_receipts = order_validation_receipts(self.connection, order_id)?;
                    Ok(OrderDetailProjection {
                        order_id,
                        farm_id,
                        order_number,
                        customer_display_name,
                        status,
                        fulfillment_window_id: parse_optional_typed_id(
                            "orders.fulfillment_window_id",
                            fulfillment_window_id,
                        )?,
                        fulfillment_window_label: empty_string_to_none(fulfillment_window_label),
                        pickup_location_label: empty_string_to_none(pickup_location_label),
                        items,
                        economics,
                        validation_receipts,
                        primary_action: primary_action_for_order(status, &workflow),
                        workflow,
                    })
                },
            )
            .transpose()
    }

    pub fn load_seller_order_decision_export(
        &self,
        farm_id: FarmId,
        order_id: OrderId,
    ) -> Result<Option<SellerOrderDecisionExport>, AppSqliteError> {
        let Some((order_id, farm_id, status)) = self
            .connection
            .query_row(
                "select id, farm_id, status
                 from orders
                 where farm_id = ?1 and id = ?2
                 limit 1",
                params![farm_id.to_string(), order_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load seller order decision export",
                source,
            })?
        else {
            return Ok(None);
        };
        let order_id = parse_typed_id("orders.id", order_id)?;
        let farm_id = parse_typed_id("orders.farm_id", farm_id)?;
        let status = parse_order_status("orders.status", status)?;
        let lines = self.load_seller_order_decision_lines(order_id)?;

        Ok(Some(SellerOrderDecisionExport {
            order_id,
            farm_id,
            status,
            lines,
        }))
    }

    pub fn load_pack_day(
        &self,
        farm_id: FarmId,
        query: &PackDayScreenQueryState,
    ) -> Result<PackDayProjection, AppSqliteError> {
        let fulfillment_window = if let Some(fulfillment_window_id) = query.fulfillment_window_id {
            self.load_fulfillment_window_by_id(farm_id, fulfillment_window_id)?
        } else {
            self.load_next_upcoming_fulfillment_window(farm_id)?
                .or(self.load_first_active_order_window(farm_id)?)
        };

        let Some(fulfillment_window) = fulfillment_window else {
            return Ok(PackDayProjection::default());
        };

        let totals_by_product =
            self.load_pack_day_totals(farm_id, fulfillment_window.fulfillment_window_id)?;
        let pack_list =
            self.load_pack_day_pack_list(farm_id, fulfillment_window.fulfillment_window_id)?;
        let pickup_roster =
            self.load_pack_day_roster(farm_id, fulfillment_window.fulfillment_window_id)?;

        Ok(PackDayProjection {
            fulfillment_window: Some(fulfillment_window),
            reminders: Default::default(),
            totals_by_product,
            pack_list,
            pickup_roster,
        })
    }

    pub fn load_pack_day_output_source(
        &self,
        farm_id: FarmId,
        fulfillment_window_id: FulfillmentWindowId,
    ) -> Result<Option<PackDayOutputSource>, AppSqliteError> {
        let Some(fulfillment_window) =
            self.load_pack_day_output_window(farm_id, fulfillment_window_id)?
        else {
            return Ok(None);
        };

        let totals_by_product = self.load_pack_day_output_totals(farm_id, fulfillment_window_id)?;
        let pack_list = self.load_pack_day_output_pack_list(farm_id, fulfillment_window_id)?;
        let pickup_roster = self.load_pack_day_output_roster(farm_id, fulfillment_window_id)?;

        Ok(Some(PackDayOutputSource {
            fulfillment_window,
            totals_by_product,
            pack_list,
            pickup_roster,
        }))
    }

    fn load_order_records(
        &self,
        farm_id: FarmId,
        fulfillment_window_id: Option<FulfillmentWindowId>,
    ) -> Result<Vec<OrderRecord>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select
                    o.id,
                    o.farm_id,
                    o.fulfillment_window_id,
                    o.order_number,
                    o.customer_display_name,
                    o.status,
                    o.workflow_revision,
                    o.workflow_agreement,
                    o.workflow_inventory,
                    o.workflow_provenance_source,
                    o.workflow_provenance_last_event_id,
                    fw.label,
                    pl.label
                 from orders o
                 left join fulfillment_windows fw on fw.id = o.fulfillment_window_id
                 left join pickup_locations pl on pl.id = fw.pickup_location_id
                 where o.farm_id = ?1
                   and (?2 is null or o.fulfillment_window_id = ?2)
                 order by o.updated_at desc, o.id desc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare orders list",
                source,
            })?;
        let rows = statement
            .query_map(
                params![
                    farm_id.to_string(),
                    fulfillment_window_id.map(|id| id.to_string())
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, Option<String>>(10)?,
                        row.get::<_, Option<String>>(11)?,
                        row.get::<_, Option<String>>(12)?,
                    ))
                },
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "query orders list",
                source,
            })?;
        let mut records = Vec::new();

        for row in rows {
            let (
                order_id,
                farm_id,
                fulfillment_window_id,
                order_number,
                customer_display_name,
                status,
                workflow_revision,
                workflow_agreement,
                workflow_inventory,
                workflow_provenance_source,
                workflow_provenance_last_event_id,
                fulfillment_window_label,
                pickup_location_label,
            ) = row.map_err(|source| AppSqliteError::Query {
                operation: "read orders list",
                source,
            })?;
            let order_id: OrderId = parse_typed_id("orders.id", order_id)?;
            let farm_id: FarmId = parse_typed_id("orders.farm_id", farm_id)?;
            let status = parse_order_status("orders.status", status)?;
            let revision =
                parse_trade_revision_status("orders.workflow_revision", workflow_revision)?;
            let items = self.load_order_detail_items(order_id.to_string())?;
            let economics = order_detail_economics(&items)?;
            let workflow = trade_workflow_projection_from_storage(StoredTradeWorkflowSnapshot {
                order_id,
                revision,
                economics,
                agreement: workflow_agreement,
                inventory: workflow_inventory,
                provenance_source: workflow_provenance_source,
                provenance_last_event_id: workflow_provenance_last_event_id,
            })?;

            records.push(OrderRecord {
                order_id,
                farm_id,
                fulfillment_window_id: parse_optional_typed_id(
                    "orders.fulfillment_window_id",
                    fulfillment_window_id,
                )?,
                order_number,
                customer_display_name,
                fulfillment_window_label: empty_string_to_none(fulfillment_window_label),
                pickup_location_label: empty_string_to_none(pickup_location_label),
                status,
                workflow,
            });
        }

        Ok(records)
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
                operation: "prepare order detail items",
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
                operation: "query order detail items",
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
                operation: "read order detail items",
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

    fn load_seller_order_decision_lines(
        &self,
        order_id: OrderId,
    ) -> Result<Vec<SellerOrderDecisionLineExport>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select id, quantity_value, listing_bin_id
                 from order_lines
                 where order_id = ?1
                 order by sort_index asc, id asc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare seller order decision lines",
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
                operation: "query seller order decision lines",
                source,
            })?;
        let mut lines = Vec::new();

        for row in rows {
            let (line_id, quantity, listing_bin_id) =
                row.map_err(|source| AppSqliteError::Query {
                    operation: "read seller order decision line",
                    source,
                })?;
            let product_id = parse_order_line_product_id(line_id.as_str(), order_id)?;
            let quantity =
                u32::try_from(quantity).map_err(|_| AppSqliteError::InvalidProjection {
                    reason: "seller order decision quantity must be non-negative",
                })?;
            if quantity == 0 {
                return Err(AppSqliteError::InvalidProjection {
                    reason: "seller order decision quantity must be positive",
                });
            }
            lines.push(SellerOrderDecisionLineExport {
                product_id,
                listing_bin_id: empty_string_to_none(listing_bin_id),
                quantity,
                stock_count: self.load_product_stock_count(product_id)?,
                reserved_quantity: self.load_reserved_product_quantity(product_id, order_id)?,
            });
        }

        Ok(lines)
    }

    fn load_product_stock_count(
        &self,
        product_id: ProductId,
    ) -> Result<Option<u32>, AppSqliteError> {
        let stock_count = self
            .connection
            .query_row(
                "select stock_count from products where id = ?1 limit 1",
                params![product_id.to_string()],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load seller order decision product stock",
                source,
            })?
            .flatten();

        stock_count
            .map(|value| {
                u32::try_from(value).map_err(|_| AppSqliteError::InvalidProjection {
                    reason: "seller order decision product stock must be non-negative",
                })
            })
            .transpose()
    }

    fn load_reserved_product_quantity(
        &self,
        product_id: ProductId,
        excluding_order_id: OrderId,
    ) -> Result<u32, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select ol.id, ol.quantity_value
                 from order_lines ol
                 inner join orders o on o.id = ol.order_id
                 where o.status in ('scheduled', 'packed')
                   and o.id <> ?1",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare seller order decision reservations",
                source,
            })?;
        let rows = statement
            .query_map(params![excluding_order_id.to_string()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query seller order decision reservations",
                source,
            })?;
        let mut reserved_quantity = 0_u32;

        for row in rows {
            let (line_id, quantity) = row.map_err(|source| AppSqliteError::Query {
                operation: "read seller order decision reservation",
                source,
            })?;
            let Some(reserved_product_id) = parse_order_line_product_id_lossy(line_id.as_str())
            else {
                continue;
            };
            if reserved_product_id != product_id {
                continue;
            }
            let quantity =
                u32::try_from(quantity).map_err(|_| AppSqliteError::InvalidProjection {
                    reason: "seller order decision reserved quantity must be non-negative",
                })?;
            reserved_quantity = reserved_quantity.checked_add(quantity).ok_or(
                AppSqliteError::InvalidProjection {
                    reason: "seller order decision reserved quantity overflowed",
                },
            )?;
        }

        Ok(reserved_quantity)
    }

    fn load_fulfillment_window_by_id(
        &self,
        farm_id: FarmId,
        fulfillment_window_id: FulfillmentWindowId,
    ) -> Result<Option<FulfillmentWindowSummary>, AppSqliteError> {
        self.connection
            .query_row(
                "select id, starts_at, ends_at
                 from fulfillment_windows
                 where farm_id = ?1 and id = ?2
                 limit 1",
                params![farm_id.to_string(), fulfillment_window_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load pack day fulfillment window",
                source,
            })?
            .map(|(fulfillment_window_id, starts_at, ends_at)| {
                Ok(FulfillmentWindowSummary {
                    fulfillment_window_id: parse_typed_id(
                        "fulfillment_windows.id",
                        fulfillment_window_id,
                    )?,
                    farm_id,
                    starts_at,
                    ends_at,
                })
            })
            .transpose()
    }

    fn load_next_upcoming_fulfillment_window(
        &self,
        farm_id: FarmId,
    ) -> Result<Option<FulfillmentWindowSummary>, AppSqliteError> {
        self.connection
            .query_row(
                "select id, starts_at, ends_at
                 from fulfillment_windows
                 where farm_id = ?1 and starts_at >= strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 order by starts_at asc, id asc
                 limit 1",
                params![farm_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load next pack day fulfillment window",
                source,
            })?
            .map(|(fulfillment_window_id, starts_at, ends_at)| {
                Ok(FulfillmentWindowSummary {
                    fulfillment_window_id: parse_typed_id(
                        "fulfillment_windows.id",
                        fulfillment_window_id,
                    )?,
                    farm_id,
                    starts_at,
                    ends_at,
                })
            })
            .transpose()
    }

    fn load_first_active_order_window(
        &self,
        farm_id: FarmId,
    ) -> Result<Option<FulfillmentWindowSummary>, AppSqliteError> {
        self.connection
            .query_row(
                "select fw.id, fw.starts_at, fw.ends_at
                 from orders o
                 join fulfillment_windows fw on fw.id = o.fulfillment_window_id
                 where o.farm_id = ?1
                   and o.status in ('needs_action', 'scheduled', 'packed')
                 order by fw.starts_at asc, fw.id asc
                 limit 1",
                params![farm_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load active pack day fulfillment window",
                source,
            })?
            .map(|(fulfillment_window_id, starts_at, ends_at)| {
                Ok(FulfillmentWindowSummary {
                    fulfillment_window_id: parse_typed_id(
                        "fulfillment_windows.id",
                        fulfillment_window_id,
                    )?,
                    farm_id,
                    starts_at,
                    ends_at,
                })
            })
            .transpose()
    }

    fn load_pack_day_output_window(
        &self,
        farm_id: FarmId,
        fulfillment_window_id: FulfillmentWindowId,
    ) -> Result<Option<PackDayOutputWindow>, AppSqliteError> {
        self.connection
            .query_row(
                "select
                    fw.id,
                    fw.farm_id,
                    f.display_name,
                    pl.label,
                    fw.starts_at,
                    fw.ends_at
                 from fulfillment_windows fw
                 join farms f on f.id = fw.farm_id
                 left join pickup_locations pl on pl.id = fw.pickup_location_id
                 where fw.farm_id = ?1 and fw.id = ?2
                 limit 1",
                params![farm_id.to_string(), fulfillment_window_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load pack day output window",
                source,
            })?
            .map(
                |(
                    window_id,
                    row_farm_id,
                    farm_display_name,
                    pickup_location_label,
                    starts_at,
                    ends_at,
                )| {
                    Ok(PackDayOutputWindow {
                        fulfillment_window_id: parse_typed_id("fulfillment_windows.id", window_id)?,
                        farm_id: parse_typed_id("fulfillment_windows.farm_id", row_farm_id)?,
                        farm_display_name,
                        pickup_location_label: empty_string_to_none(pickup_location_label),
                        starts_at,
                        ends_at,
                    })
                },
            )
            .transpose()
    }

    fn load_pack_day_totals(
        &self,
        farm_id: FarmId,
        fulfillment_window_id: FulfillmentWindowId,
    ) -> Result<Vec<PackDayProductTotalRow>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select l.title, l.quantity_value, l.quantity_unit_label
                 from order_lines l
                 join orders o on o.id = l.order_id
                 where o.farm_id = ?1
                   and o.fulfillment_window_id = ?2
                   and o.status in ('needs_action', 'scheduled', 'packed')
                 order by l.title asc, l.sort_index asc, l.id asc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare pack day totals",
                source,
            })?;
        let rows = statement
            .query_map(
                params![farm_id.to_string(), fulfillment_window_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, u32>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "query pack day totals",
                source,
            })?;
        let mut totals = BTreeMap::<(String, String), u32>::new();

        for row in rows {
            let (title, quantity_value, quantity_unit_label) =
                row.map_err(|source| AppSqliteError::Query {
                    operation: "read pack day totals",
                    source,
                })?;
            *totals.entry((title, quantity_unit_label)).or_insert(0) += quantity_value;
        }

        Ok(totals
            .into_iter()
            .map(
                |((title, quantity_unit_label), quantity_value)| PackDayProductTotalRow {
                    title,
                    quantity_display: format_quantity_display(quantity_value, &quantity_unit_label),
                },
            )
            .collect())
    }

    fn load_pack_day_pack_list(
        &self,
        farm_id: FarmId,
        fulfillment_window_id: FulfillmentWindowId,
    ) -> Result<Vec<PackDayPackListRow>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select o.customer_display_name, l.title, l.quantity_display
                 from order_lines l
                 join orders o on o.id = l.order_id
                 where o.farm_id = ?1
                   and o.fulfillment_window_id = ?2
                   and o.status in ('needs_action', 'scheduled', 'packed')
                 order by l.title asc, o.customer_display_name asc, o.order_number asc, l.sort_index asc, l.id asc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare pack day pack list",
                source,
            })?;
        let rows = statement
            .query_map(
                params![farm_id.to_string(), fulfillment_window_id.to_string()],
                |row| {
                    Ok(PackDayPackListRow {
                        title: row.get(1)?,
                        quantity_display: format!(
                            "{}: {}",
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(2)?
                        ),
                    })
                },
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "query pack day pack list",
                source,
            })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| AppSqliteError::Query {
                operation: "read pack day pack list",
                source,
            })
    }

    fn load_pack_day_output_totals(
        &self,
        farm_id: FarmId,
        fulfillment_window_id: FulfillmentWindowId,
    ) -> Result<Vec<PackDayOutputProductTotal>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select l.title, l.quantity_value, l.quantity_unit_label
                 from order_lines l
                 join orders o on o.id = l.order_id
                 where o.farm_id = ?1
                   and o.fulfillment_window_id = ?2
                   and o.status in ('needs_action', 'scheduled', 'packed')
                 order by l.title asc, l.sort_index asc, l.id asc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare pack day output totals",
                source,
            })?;
        let rows = statement
            .query_map(
                params![farm_id.to_string(), fulfillment_window_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, u32>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "query pack day output totals",
                source,
            })?;
        let mut totals = BTreeMap::<(String, String), u32>::new();

        for row in rows {
            let (title, quantity_value, quantity_unit_label) =
                row.map_err(|source| AppSqliteError::Query {
                    operation: "read pack day output totals",
                    source,
                })?;
            *totals.entry((title, quantity_unit_label)).or_insert(0) += quantity_value;
        }

        Ok(totals
            .into_iter()
            .map(
                |((title, quantity_unit_label), quantity_value)| PackDayOutputProductTotal {
                    title,
                    quantity: PackDayOutputQuantity::new(quantity_value, quantity_unit_label),
                },
            )
            .collect())
    }

    fn load_pack_day_output_pack_list(
        &self,
        farm_id: FarmId,
        fulfillment_window_id: FulfillmentWindowId,
    ) -> Result<Vec<PackDayOutputPackListEntry>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select
                    o.id,
                    o.order_number,
                    o.customer_display_name,
                    o.status,
                    l.title,
                    l.quantity_value,
                    l.quantity_unit_label
                 from order_lines l
                 join orders o on o.id = l.order_id
                 where o.farm_id = ?1
                   and o.fulfillment_window_id = ?2
                   and o.status in ('needs_action', 'scheduled', 'packed')
                 order by l.title asc, o.customer_display_name asc, o.order_number asc, l.sort_index asc, l.id asc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare pack day output pack list",
                source,
            })?;
        let rows = statement
            .query_map(
                params![farm_id.to_string(), fulfillment_window_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, u32>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                },
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "query pack day output pack list",
                source,
            })?;
        let mut pack_list = Vec::new();

        for row in rows {
            let (
                order_id,
                order_number,
                customer_display_name,
                status,
                title,
                quantity_value,
                quantity_unit_label,
            ) = row.map_err(|source| AppSqliteError::Query {
                operation: "read pack day output pack list",
                source,
            })?;
            pack_list.push(PackDayOutputPackListEntry {
                order_id: parse_typed_id("orders.id", order_id)?,
                order_number,
                customer_display_name,
                order_state: parse_pack_day_output_order_state("orders.status", status)?,
                title,
                quantity: PackDayOutputQuantity::new(quantity_value, quantity_unit_label),
            });
        }

        Ok(pack_list)
    }

    fn load_pack_day_roster(
        &self,
        farm_id: FarmId,
        fulfillment_window_id: FulfillmentWindowId,
    ) -> Result<Vec<PackDayRosterRow>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select id, order_number, customer_display_name
                 from orders
                 where farm_id = ?1
                   and fulfillment_window_id = ?2
                   and status in ('needs_action', 'scheduled', 'packed')
                 order by customer_display_name asc, order_number asc, id asc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare pack day roster",
                source,
            })?;
        let rows = statement
            .query_map(
                params![farm_id.to_string(), fulfillment_window_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "query pack day roster",
                source,
            })?;
        let mut roster = Vec::new();

        for row in rows {
            let (order_id, order_number, customer_display_name) =
                row.map_err(|source| AppSqliteError::Query {
                    operation: "read pack day roster",
                    source,
                })?;
            roster.push(PackDayRosterRow {
                order_id: parse_typed_id("orders.id", order_id)?,
                order_number,
                customer_display_name,
            });
        }

        Ok(roster)
    }

    fn load_pack_day_output_roster(
        &self,
        farm_id: FarmId,
        fulfillment_window_id: FulfillmentWindowId,
    ) -> Result<Vec<PackDayOutputCustomerOrder>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select id, order_number, customer_display_name, status
                 from orders
                 where farm_id = ?1
                   and fulfillment_window_id = ?2
                   and status in ('needs_action', 'scheduled', 'packed')
                 order by customer_display_name asc, order_number asc, id asc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare pack day output roster",
                source,
            })?;
        let rows = statement
            .query_map(
                params![farm_id.to_string(), fulfillment_window_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "query pack day output roster",
                source,
            })?;
        let mut roster = Vec::new();

        for row in rows {
            let (order_id, order_number, customer_display_name, status) =
                row.map_err(|source| AppSqliteError::Query {
                    operation: "read pack day output roster",
                    source,
                })?;
            roster.push(PackDayOutputCustomerOrder {
                order_id: parse_typed_id("orders.id", order_id)?,
                order_number,
                customer_display_name,
                order_state: parse_pack_day_output_order_state("orders.status", status)?,
            });
        }

        Ok(roster)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OrderRecord {
    order_id: OrderId,
    farm_id: FarmId,
    fulfillment_window_id: Option<FulfillmentWindowId>,
    order_number: String,
    customer_display_name: String,
    fulfillment_window_label: Option<String>,
    pickup_location_label: Option<String>,
    status: OrderStatus,
    workflow: TradeWorkflowProjection,
}

impl OrderRecord {
    fn matches_filter(&self, filter: OrdersFilter) -> bool {
        match filter {
            OrdersFilter::All => true,
            OrdersFilter::NeedsAction => {
                matches!(
                    self.status,
                    OrderStatus::NeedsAction | OrderStatus::NeedsReview
                )
            }
            OrdersFilter::Scheduled => self.status == OrderStatus::Scheduled,
            OrdersFilter::Packed => self.status == OrderStatus::Packed,
            OrdersFilter::Completed => self.status == OrderStatus::Completed,
        }
    }

    fn into_list_row(self) -> OrdersListRow {
        OrdersListRow {
            order_id: self.order_id,
            farm_id: self.farm_id,
            fulfillment_window_id: self.fulfillment_window_id,
            order_number: self.order_number,
            customer_display_name: self.customer_display_name,
            fulfillment_window_label: self.fulfillment_window_label,
            pickup_location_label: self.pickup_location_label,
            status: self.status,
            primary_action: primary_action_for_order(self.status, &self.workflow),
            workflow: self.workflow,
        }
    }
}

fn summarize_orders(records: &[OrderRecord]) -> OrdersListSummary {
    let mut summary = OrdersListSummary {
        total_orders: records.len() as u32,
        ..OrdersListSummary::default()
    };

    for record in records {
        match record.status {
            OrderStatus::NeedsAction | OrderStatus::NeedsReview => summary.needs_action_orders += 1,
            OrderStatus::Scheduled => summary.scheduled_orders += 1,
            OrderStatus::Packed => summary.packed_orders += 1,
            OrderStatus::Completed | OrderStatus::Declined => {}
        }
    }

    summary
}

fn primary_action_for_order(
    status: OrderStatus,
    workflow: &TradeWorkflowProjection,
) -> Option<OrderPrimaryAction> {
    match status {
        OrderStatus::NeedsAction if workflow.agreement == TradeAgreementStatus::PendingRhi => None,
        OrderStatus::NeedsAction => Some(OrderPrimaryAction::Review),
        OrderStatus::Scheduled
        | OrderStatus::Packed
        | OrderStatus::Completed
        | OrderStatus::Declined
        | OrderStatus::NeedsReview => None,
    }
}

fn format_quantity_display(quantity_value: u32, quantity_unit_label: &str) -> String {
    if quantity_unit_label.trim().is_empty() {
        quantity_value.to_string()
    } else {
        format!("{quantity_value} {}", quantity_unit_label.trim())
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

fn parse_order_line_product_id(
    line_id: &str,
    order_id: OrderId,
) -> Result<ProductId, AppSqliteError> {
    let prefix = format!("{order_id}:");
    let Some(product_id) = line_id.strip_prefix(prefix.as_str()) else {
        return Err(AppSqliteError::InvalidProjection {
            reason: "seller order decision line id must include order id prefix",
        });
    };

    parse_typed_id("order_lines.product_id", product_id.to_owned())
}

fn parse_order_line_product_id_lossy(line_id: &str) -> Option<ProductId> {
    line_id
        .rsplit_once(':')
        .and_then(|(_, product_id)| product_id.parse().ok())
}

fn parse_order_status(field: &'static str, value: String) -> Result<OrderStatus, AppSqliteError> {
    match value.as_str() {
        "needs_action" => Ok(OrderStatus::NeedsAction),
        "scheduled" => Ok(OrderStatus::Scheduled),
        "packed" => Ok(OrderStatus::Packed),
        "completed" => Ok(OrderStatus::Completed),
        "declined" => Ok(OrderStatus::Declined),
        "needs_review" => Ok(OrderStatus::NeedsReview),
        _ => Err(AppSqliteError::DecodeEnum { field, value }),
    }
}

fn parse_pack_day_output_order_state(
    field: &'static str,
    value: String,
) -> Result<PackDayOutputOrderState, AppSqliteError> {
    let status = parse_order_status(field, value)?;
    PackDayOutputOrderState::from_order_status(status).ok_or(AppSqliteError::InvalidProjection {
        reason: "pack day output source may only include needs_action, scheduled, or packed orders",
    })
}

fn empty_string_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim().to_owned();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

#[cfg(test)]
mod tests {
    use radroots_studio_app_view::{
        FarmId, FulfillmentWindowId, OrderId, OrderPrimaryAction, OrderStatus, OrdersFilter,
        OrdersScreenQueryState, PackDayOutputOrderState, PackDayProductTotalRow,
        PackDayScreenQueryState, PickupLocationId, TradeAgreementStatus, TradeInventoryStatus,
        TradeRevisionStatus, TradeWorkflowSource,
    };
    use rusqlite::{Connection, params};

    use crate::{AppSqliteError, AppSqliteStore, DatabaseTarget};

    #[test]
    fn orders_list_loads_summary_rows_and_window_filter_truthfully() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let farm_id = FarmId::new();
        let other_farm_id = FarmId::new();
        let fulfillment_window_id = FulfillmentWindowId::new();
        let other_window_id = FulfillmentWindowId::new();

        insert_farm(
            connection,
            farm_id,
            "Willow farm",
            "ready",
            "2026-04-17T08:00:00Z",
        );
        insert_farm(
            connection,
            other_farm_id,
            "Other farm",
            "ready",
            "2026-04-17T08:00:00Z",
        );
        let pickup_location_id = insert_pickup_location(connection, farm_id, "North barn", true);
        insert_window(
            connection,
            fulfillment_window_id,
            farm_id,
            Some(pickup_location_id),
            "Friday pickup",
            "2099-04-18T16:00:00Z",
            "2099-04-18T18:00:00Z",
            "2099-04-17T18:00:00Z",
        );
        insert_window(
            connection,
            other_window_id,
            farm_id,
            None,
            "Saturday pickup",
            "2099-04-19T16:00:00Z",
            "2099-04-19T18:00:00Z",
            "2099-04-18T18:00:00Z",
        );

        insert_order(
            connection,
            OrderId::new(),
            farm_id,
            Some(fulfillment_window_id),
            "R-100",
            "Casey",
            "needs_action",
            "2026-04-17T10:00:00Z",
        );
        insert_order(
            connection,
            OrderId::new(),
            farm_id,
            Some(fulfillment_window_id),
            "R-101",
            "Taylor",
            "scheduled",
            "2026-04-17T11:00:00Z",
        );
        insert_order(
            connection,
            OrderId::new(),
            farm_id,
            Some(fulfillment_window_id),
            "R-102",
            "Robin",
            "packed",
            "2026-04-17T12:00:00Z",
        );
        insert_order(
            connection,
            OrderId::new(),
            farm_id,
            Some(other_window_id),
            "R-103",
            "Morgan",
            "completed",
            "2026-04-17T13:00:00Z",
        );
        insert_order(
            connection,
            OrderId::new(),
            farm_id,
            None,
            "R-104",
            "Alex",
            "needs_review",
            "2026-04-17T14:00:00Z",
        );
        insert_order(
            connection,
            OrderId::new(),
            other_farm_id,
            Some(fulfillment_window_id),
            "R-999",
            "Other",
            "needs_action",
            "2026-04-17T15:00:00Z",
        );

        let projection = store
            .load_orders_list(
                farm_id,
                &OrdersScreenQueryState {
                    filter: OrdersFilter::NeedsAction,
                    fulfillment_window_id: Some(fulfillment_window_id),
                },
            )
            .expect("orders list should load");

        assert_eq!(projection.summary.total_orders, 3);
        assert_eq!(projection.summary.needs_action_orders, 1);
        assert_eq!(projection.summary.scheduled_orders, 1);
        assert_eq!(projection.summary.packed_orders, 1);
        assert_eq!(projection.rows.len(), 1);
        assert_eq!(projection.rows[0].order_number, "R-100");
        assert_eq!(
            projection.rows[0].fulfillment_window_label.as_deref(),
            Some("Friday pickup")
        );
        assert_eq!(
            projection.rows[0].pickup_location_label.as_deref(),
            Some("North barn")
        );
        assert_eq!(
            projection.rows[0].primary_action,
            Some(OrderPrimaryAction::Review)
        );
    }

    #[test]
    fn order_detail_loads_items_and_context() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let farm_id = FarmId::new();
        let fulfillment_window_id = FulfillmentWindowId::new();
        let order_id = OrderId::new();

        insert_farm(
            connection,
            farm_id,
            "Willow farm",
            "ready",
            "2026-04-17T08:00:00Z",
        );
        let pickup_location_id = insert_pickup_location(connection, farm_id, "North barn", true);
        insert_window(
            connection,
            fulfillment_window_id,
            farm_id,
            Some(pickup_location_id),
            "Friday pickup",
            "2099-04-18T16:00:00Z",
            "2099-04-18T18:00:00Z",
            "2099-04-17T18:00:00Z",
        );
        insert_order(
            connection,
            order_id,
            farm_id,
            Some(fulfillment_window_id),
            "R-100",
            "Casey",
            "scheduled",
            "2026-04-17T10:00:00Z",
        );
        insert_order_line(
            connection,
            "line-1",
            order_id,
            "Salad mix",
            2,
            "bags",
            "2 bags",
            0,
        );
        insert_order_line(
            connection, "line-2", order_id, "Carrots", 1, "bunches", "1 bunch", 1,
        );

        let detail = store
            .load_order_detail(farm_id, order_id)
            .expect("order detail should load")
            .expect("order detail should exist");

        assert_eq!(detail.order_number, "R-100");
        assert_eq!(detail.customer_display_name, "Casey");
        assert_eq!(detail.status, OrderStatus::Scheduled);
        assert_eq!(
            detail.fulfillment_window_label.as_deref(),
            Some("Friday pickup")
        );
        assert_eq!(detail.pickup_location_label.as_deref(), Some("North barn"));
        assert_eq!(detail.items.len(), 2);
        assert_eq!(detail.items[0].title, "Salad mix");
        assert_eq!(
            detail.items[0]
                .unit_price
                .as_ref()
                .map(|price| price.amount_minor_units),
            Some(650)
        );
        assert_eq!(detail.items[0].line_total_minor_units, Some(1300));
        assert_eq!(detail.items[1].quantity_display, "1 bunch");
        assert_eq!(detail.economics.total_minor_units, Some(1950));
        assert_eq!(detail.economics.currency_code.as_deref(), Some("USD"));
        assert_eq!(detail.primary_action, None);
    }

    #[test]
    fn seller_order_projections_fail_closed_for_invalid_workflow_revision() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let farm_id = FarmId::new();
        let order_id = OrderId::new();

        insert_farm(
            connection,
            farm_id,
            "Willow farm",
            "ready",
            "2026-04-17T08:00:00Z",
        );
        insert_order(
            connection,
            order_id,
            farm_id,
            None,
            "R-100",
            "Casey",
            "scheduled",
            "2026-04-17T10:00:00Z",
        );
        set_order_workflow_revision(
            connection,
            order_id,
            TradeRevisionStatus::Updated.storage_key(),
        );

        let list = store
            .load_orders_list(
                farm_id,
                &OrdersScreenQueryState {
                    filter: OrdersFilter::All,
                    fulfillment_window_id: None,
                },
            )
            .expect("valid revision should load in seller list");
        let detail = store
            .load_order_detail(farm_id, order_id)
            .expect("valid revision should load in seller detail")
            .expect("seller detail should exist");

        assert_eq!(list.rows[0].workflow.revision, TradeRevisionStatus::Updated);
        assert_eq!(detail.workflow.revision, TradeRevisionStatus::Updated);

        corrupt_order_workflow_revision(connection, order_id, "future_revision");

        let list_error = store
            .load_orders_list(
                farm_id,
                &OrdersScreenQueryState {
                    filter: OrdersFilter::All,
                    fulfillment_window_id: None,
                },
            )
            .expect_err("invalid revision should fail seller list projection");
        let detail_error = store
            .load_order_detail(farm_id, order_id)
            .expect_err("invalid revision should fail seller detail projection");

        assert_decode_enum(list_error, "orders.workflow_revision", "future_revision");
        assert_decode_enum(detail_error, "orders.workflow_revision", "future_revision");
    }

    #[test]
    fn seller_order_projections_read_workflow_display_snapshot() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let farm_id = FarmId::new();
        let order_id = OrderId::new();

        insert_farm(
            connection,
            farm_id,
            "Willow farm",
            "ready",
            "2026-04-17T08:00:00Z",
        );
        insert_order(
            connection,
            order_id,
            farm_id,
            None,
            "R-100",
            "Casey",
            "scheduled",
            "2026-04-17T10:00:00Z",
        );
        insert_order_line(
            connection,
            "line-1",
            order_id,
            "Salad mix",
            2,
            "bags",
            "2 bags",
            0,
        );
        set_order_workflow_revision(
            connection,
            order_id,
            TradeRevisionStatus::Updated.storage_key(),
        );
        set_order_workflow_display_projection(
            connection,
            order_id,
            "confirmed",
            "reserved",
            "local_events",
            Some("seller-workflow-event"),
        );

        let list = store
            .load_orders_list(
                farm_id,
                &OrdersScreenQueryState {
                    filter: OrdersFilter::All,
                    fulfillment_window_id: None,
                },
            )
            .expect("seller list should load");
        let detail = store
            .load_order_detail(farm_id, order_id)
            .expect("seller detail should load")
            .expect("seller detail should exist");
        let workflow = &list.rows[0].workflow;

        assert_eq!(workflow.agreement, TradeAgreementStatus::Confirmed);
        assert_eq!(workflow.revision, TradeRevisionStatus::Updated);
        assert_eq!(workflow.inventory, TradeInventoryStatus::Reserved);
        assert_eq!(
            workflow.provenance.primary_source,
            TradeWorkflowSource::LocalEvents
        );
        assert_eq!(
            workflow.provenance.last_event_id.as_deref(),
            Some("seller-workflow-event")
        );
        assert_eq!(workflow.economics.total_minor_units, Some(1300));
        assert_eq!(workflow.economics.currency_code.as_deref(), Some("USD"));
        assert_eq!(detail.workflow, *workflow);
        assert_eq!(list.rows[0].primary_action, None);
        assert_eq!(detail.primary_action, None);
    }

    #[test]
    fn seller_order_projections_fail_closed_for_invalid_workflow_snapshot_keys() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let farm_id = FarmId::new();
        let order_id = OrderId::new();

        insert_farm(
            connection,
            farm_id,
            "Willow farm",
            "ready",
            "2026-04-17T08:00:00Z",
        );
        insert_order(
            connection,
            order_id,
            farm_id,
            None,
            "R-100",
            "Casey",
            "scheduled",
            "2026-04-17T10:00:00Z",
        );
        set_order_workflow_display_projection(
            connection,
            order_id,
            "confirmed",
            "reserved",
            "local_events",
            Some("seller-workflow-event"),
        );

        for (column, expected_field) in [
            ("workflow_agreement", "orders.workflow_agreement"),
            ("workflow_inventory", "orders.workflow_inventory"),
            (
                "workflow_provenance_source",
                "orders.workflow_provenance_source",
            ),
        ] {
            set_order_workflow_display_projection(
                connection,
                order_id,
                "confirmed",
                "reserved",
                "local_events",
                Some("seller-workflow-event"),
            );
            corrupt_order_workflow_display_projection(connection, order_id, column, "future_state");

            let list_error = store
                .load_orders_list(
                    farm_id,
                    &OrdersScreenQueryState {
                        filter: OrdersFilter::All,
                        fulfillment_window_id: None,
                    },
                )
                .expect_err("invalid workflow snapshot should fail seller list projection");
            let detail_error = store
                .load_order_detail(farm_id, order_id)
                .expect_err("invalid workflow snapshot should fail seller detail projection");

            assert_decode_enum(list_error, expected_field, "future_state");
            assert_decode_enum(detail_error, expected_field, "future_state");
        }
    }

    #[test]
    fn pack_day_defaults_to_next_window_and_projects_totals_pack_list_and_roster() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let farm_id = FarmId::new();
        let next_window_id = FulfillmentWindowId::new();
        let later_window_id = FulfillmentWindowId::new();
        let scheduled_order_id = OrderId::new();
        let packed_order_id = OrderId::new();

        insert_farm(
            connection,
            farm_id,
            "Willow farm",
            "ready",
            "2026-04-17T08:00:00Z",
        );
        let pickup_location_id = insert_pickup_location(connection, farm_id, "North barn", true);
        insert_window(
            connection,
            next_window_id,
            farm_id,
            Some(pickup_location_id),
            "Friday pickup",
            "2099-04-18T16:00:00Z",
            "2099-04-18T18:00:00Z",
            "2099-04-17T18:00:00Z",
        );
        insert_window(
            connection,
            later_window_id,
            farm_id,
            Some(pickup_location_id),
            "Saturday pickup",
            "2099-04-19T16:00:00Z",
            "2099-04-19T18:00:00Z",
            "2099-04-18T18:00:00Z",
        );
        insert_order(
            connection,
            scheduled_order_id,
            farm_id,
            Some(next_window_id),
            "R-100",
            "Casey",
            "scheduled",
            "2026-04-17T10:00:00Z",
        );
        insert_order(
            connection,
            packed_order_id,
            farm_id,
            Some(next_window_id),
            "R-101",
            "Taylor",
            "packed",
            "2026-04-17T11:00:00Z",
        );
        insert_order(
            connection,
            OrderId::new(),
            farm_id,
            Some(next_window_id),
            "R-102",
            "Robin",
            "completed",
            "2026-04-17T12:00:00Z",
        );
        insert_order(
            connection,
            OrderId::new(),
            farm_id,
            Some(later_window_id),
            "R-200",
            "Morgan",
            "scheduled",
            "2026-04-17T13:00:00Z",
        );
        insert_order_line(
            connection,
            "line-1",
            scheduled_order_id,
            "Salad mix",
            2,
            "bags",
            "2 bags",
            0,
        );
        insert_order_line(
            connection,
            "line-2",
            packed_order_id,
            "Salad mix",
            1,
            "bags",
            "1 bag",
            0,
        );
        insert_order_line(
            connection,
            "line-3",
            packed_order_id,
            "Carrots",
            3,
            "bunches",
            "3 bunches",
            1,
        );

        let projection = store
            .load_pack_day(farm_id, &PackDayScreenQueryState::default())
            .expect("pack day should load");

        assert_eq!(
            projection
                .fulfillment_window
                .expect("window should exist")
                .fulfillment_window_id,
            next_window_id
        );
        assert_eq!(
            projection.totals_by_product,
            vec![
                PackDayProductTotalRow {
                    title: "Carrots".to_owned(),
                    quantity_display: "3 bunches".to_owned(),
                },
                PackDayProductTotalRow {
                    title: "Salad mix".to_owned(),
                    quantity_display: "3 bags".to_owned(),
                },
            ]
        );
        assert_eq!(projection.pack_list.len(), 3);
        assert_eq!(projection.pack_list[0].title, "Carrots");
        assert_eq!(
            projection.pack_list[0].quantity_display,
            "Taylor: 3 bunches"
        );
        assert_eq!(projection.pickup_roster.len(), 2);
        assert_eq!(projection.pickup_roster[0].order_id, scheduled_order_id);
        assert_eq!(projection.pickup_roster[1].order_id, packed_order_id);
    }

    #[test]
    fn pack_day_output_source_projects_canonical_records_without_screen_strings() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let farm_id = FarmId::new();
        let fulfillment_window_id = FulfillmentWindowId::new();
        let scheduled_order_id = OrderId::new();
        let packed_order_id = OrderId::new();

        insert_farm(
            connection,
            farm_id,
            "Willow farm",
            "ready",
            "2026-04-17T08:00:00Z",
        );
        let pickup_location_id = insert_pickup_location(connection, farm_id, "North barn", true);
        insert_window(
            connection,
            fulfillment_window_id,
            farm_id,
            Some(pickup_location_id),
            "Friday pickup",
            "2099-04-18T16:00:00Z",
            "2099-04-18T18:00:00Z",
            "2099-04-17T18:00:00Z",
        );
        insert_order(
            connection,
            scheduled_order_id,
            farm_id,
            Some(fulfillment_window_id),
            "R-100",
            "Casey",
            "scheduled",
            "2026-04-17T10:00:00Z",
        );
        insert_order(
            connection,
            packed_order_id,
            farm_id,
            Some(fulfillment_window_id),
            "R-101",
            "Taylor",
            "packed",
            "2026-04-17T11:00:00Z",
        );
        insert_order(
            connection,
            OrderId::new(),
            farm_id,
            Some(fulfillment_window_id),
            "R-102",
            "Robin",
            "completed",
            "2026-04-17T12:00:00Z",
        );
        insert_order_line(
            connection,
            "line-export-1",
            scheduled_order_id,
            "Salad mix",
            2,
            "bags",
            "Casey should not leak into export quantity",
            0,
        );
        insert_order_line(
            connection,
            "line-export-2",
            packed_order_id,
            "Salad mix",
            1,
            "bags",
            "1 bag",
            0,
        );
        insert_order_line(
            connection,
            "line-export-3",
            packed_order_id,
            "Carrots",
            3,
            "bunches",
            "3 bunches",
            1,
        );

        let source = store
            .load_pack_day_output_source(farm_id, fulfillment_window_id)
            .expect("output source should load")
            .expect("output source should exist");

        assert_eq!(source.fulfillment_window.farm_display_name, "Willow farm");
        assert_eq!(
            source.fulfillment_window.pickup_location_label.as_deref(),
            Some("North barn")
        );
        assert_eq!(source.totals_by_product.len(), 2);
        assert_eq!(source.totals_by_product[0].title, "Carrots");
        assert_eq!(source.totals_by_product[0].quantity.value, 3);
        assert_eq!(source.totals_by_product[0].quantity.unit_label, "bunches");
        assert_eq!(source.totals_by_product[1].title, "Salad mix");
        assert_eq!(source.totals_by_product[1].quantity.value, 3);
        assert_eq!(source.totals_by_product[1].quantity.unit_label, "bags");
        assert_eq!(source.pack_list.len(), 3);
        assert_eq!(source.pack_list[0].customer_display_name, "Taylor");
        assert_eq!(
            source.pack_list[0].order_state,
            PackDayOutputOrderState::Packed
        );
        assert_eq!(source.pack_list[0].quantity.value, 3);
        assert_eq!(source.pack_list[1].customer_display_name, "Casey");
        assert_eq!(source.pack_list[1].quantity.value, 2);
        assert_eq!(source.pickup_roster.len(), 2);
        assert_eq!(
            source
                .pickup_roster
                .iter()
                .map(|row| row.order_state)
                .collect::<Vec<_>>(),
            vec![
                PackDayOutputOrderState::Scheduled,
                PackDayOutputOrderState::Packed,
            ]
        );
        assert!(
            source
                .pack_list
                .iter()
                .all(|row| row.order_number != "R-102")
        );
    }

    #[test]
    fn orders_list_stays_aligned_with_today_needs_action_order_boundary() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let farm_id = FarmId::new();
        let fulfillment_window_id = FulfillmentWindowId::new();

        insert_farm(
            connection,
            farm_id,
            "Willow farm",
            "ready",
            "2026-04-17T08:00:00Z",
        );
        insert_window(
            connection,
            fulfillment_window_id,
            farm_id,
            None,
            "Friday pickup",
            "2099-04-18T16:00:00Z",
            "2099-04-18T18:00:00Z",
            "2099-04-17T18:00:00Z",
        );
        for index in 0..5 {
            insert_order(
                connection,
                OrderId::new(),
                farm_id,
                Some(fulfillment_window_id),
                &format!("R-10{index}"),
                "Casey",
                "needs_action",
                &format!("2026-04-17T0{index}:00:00Z"),
            );
        }

        let today = store
            .load_today_agenda(Some(farm_id))
            .expect("today agenda should load");
        let orders = store
            .load_orders_list(
                farm_id,
                &OrdersScreenQueryState {
                    filter: OrdersFilter::NeedsAction,
                    fulfillment_window_id: None,
                },
            )
            .expect("orders list should load");

        assert_eq!(today.orders_needing_action.len(), 4);
        assert_eq!(orders.rows.len(), 5);
        let today_numbers = today
            .orders_needing_action
            .iter()
            .map(|row| row.order_number.as_str())
            .collect::<Vec<_>>();
        let orders_numbers = orders
            .rows
            .iter()
            .take(4)
            .map(|row| row.order_number.as_str())
            .collect::<Vec<_>>();
        assert_eq!(today_numbers, orders_numbers);
    }

    fn insert_farm(
        connection: &Connection,
        farm_id: FarmId,
        display_name: &str,
        readiness: &str,
        created_at: &str,
    ) {
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
                 ) values (?1, ?2, ?3, 'UTC', 'USD', ?4, ?4)",
                params![farm_id.to_string(), display_name, readiness, created_at],
            )
            .expect("farm insert should succeed");
    }

    fn insert_pickup_location(
        connection: &Connection,
        farm_id: FarmId,
        label: &str,
        is_default: bool,
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
                 ) values (?1, ?2, ?3, '14 County Road', null, ?4, '2026-04-17T08:00:00Z', '2026-04-17T08:00:00Z')",
                params![
                    pickup_location_id.to_string(),
                    farm_id.to_string(),
                    label,
                    if is_default { 1_i64 } else { 0_i64 },
                ],
            )
            .expect("pickup location insert should succeed");

        pickup_location_id
    }

    fn insert_window(
        connection: &Connection,
        fulfillment_window_id: FulfillmentWindowId,
        farm_id: FarmId,
        pickup_location_id: Option<PickupLocationId>,
        label: &str,
        starts_at: &str,
        ends_at: &str,
        order_cutoff_at: &str,
    ) {
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
                 ) values (?1, ?2, ?3, ?4, null, ?3, ?3, ?5, ?6, ?7)",
                params![
                    fulfillment_window_id.to_string(),
                    farm_id.to_string(),
                    starts_at,
                    ends_at,
                    pickup_location_id.map(|value| value.to_string()),
                    label,
                    order_cutoff_at,
                ],
            )
            .expect("fulfillment window insert should succeed");
    }

    fn insert_order(
        connection: &Connection,
        order_id: OrderId,
        farm_id: FarmId,
        fulfillment_window_id: Option<FulfillmentWindowId>,
        order_number: &str,
        customer_display_name: &str,
        status: &str,
        updated_at: &str,
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
                    updated_at
                 ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    order_id.to_string(),
                    farm_id.to_string(),
                    fulfillment_window_id.map(|id| id.to_string()),
                    order_number,
                    customer_display_name,
                    status,
                    updated_at,
                ],
            )
            .expect("order insert should succeed");
    }

    fn set_order_workflow_revision(
        connection: &Connection,
        order_id: OrderId,
        workflow_revision: &str,
    ) {
        connection
            .execute(
                "update orders set workflow_revision = ?1 where id = ?2",
                params![workflow_revision, order_id.to_string()],
            )
            .expect("order workflow revision update should succeed");
    }

    fn corrupt_order_workflow_revision(
        connection: &Connection,
        order_id: OrderId,
        workflow_revision: &str,
    ) {
        connection
            .execute_batch("pragma ignore_check_constraints = on")
            .expect("check constraints should disable");
        set_order_workflow_revision(connection, order_id, workflow_revision);
        connection
            .execute_batch("pragma ignore_check_constraints = off")
            .expect("check constraints should re-enable");
    }

    fn set_order_workflow_display_projection(
        connection: &Connection,
        order_id: OrderId,
        agreement: &str,
        inventory: &str,
        provenance_source: &str,
        provenance_last_event_id: Option<&str>,
    ) {
        connection
            .execute(
                "update orders
                 set workflow_agreement = ?1,
                     workflow_inventory = ?2,
                     workflow_provenance_source = ?3,
                     workflow_provenance_last_event_id = ?4
                 where id = ?5",
                params![
                    agreement,
                    inventory,
                    provenance_source,
                    provenance_last_event_id,
                    order_id.to_string(),
                ],
            )
            .expect("order workflow display projection update should succeed");
    }

    fn corrupt_order_workflow_display_projection(
        connection: &Connection,
        order_id: OrderId,
        column: &str,
        value: &str,
    ) {
        connection
            .execute_batch("pragma ignore_check_constraints = on")
            .expect("check constraints should disable");
        let statement = match column {
            "workflow_agreement" => "update orders set workflow_agreement = ?1 where id = ?2",
            "workflow_inventory" => "update orders set workflow_inventory = ?1 where id = ?2",
            "workflow_provenance_source" => {
                "update orders set workflow_provenance_source = ?1 where id = ?2"
            }
            _ => panic!("unsupported workflow display projection column {column}"),
        };
        connection
            .execute(statement, params![value, order_id.to_string()])
            .expect("order workflow display projection corruption should succeed");
        connection
            .execute_batch("pragma ignore_check_constraints = off")
            .expect("check constraints should re-enable");
    }

    fn assert_decode_enum(error: AppSqliteError, expected_field: &str, expected_value: &str) {
        match error {
            AppSqliteError::DecodeEnum { field, value } => {
                assert_eq!(field, expected_field);
                assert_eq!(value, expected_value);
            }
            other => panic!("expected DecodeEnum error, got {other:?}"),
        }
    }

    fn insert_order_line(
        connection: &Connection,
        line_id: &str,
        order_id: OrderId,
        title: &str,
        quantity_value: u32,
        quantity_unit_label: &str,
        quantity_display: &str,
        sort_index: i64,
    ) {
        connection
            .execute(
                "insert into order_lines (
                    id,
                    order_id,
                    title,
                    quantity_value,
                    quantity_unit_label,
                    quantity_display,
                    unit_price_minor_units,
                    price_currency,
                    sort_index
                 ) values (?1, ?2, ?3, ?4, ?5, ?6, 650, 'USD', ?7)",
                params![
                    line_id,
                    order_id.to_string(),
                    title,
                    quantity_value,
                    quantity_unit_label,
                    quantity_display,
                    sort_index,
                ],
            )
            .expect("order line insert should succeed");
    }
}
