use radroots_studio_app_view::{
    FarmId, FarmReadiness, FarmSummary, FulfillmentWindowSummary, OrderListRow, OrderStatus,
    ProductListRow, ProductStatus, TodayAgendaProjection, TodaySetupTask, TodaySetupTaskKind,
    TodaySummary,
};
use rusqlite::{Connection, OptionalExtension, Params, params};

use crate::AppSqliteError;

pub const TODAY_AGENDA_LIST_LIMIT: i64 = 4;
pub const TODAY_AGENDA_LOW_STOCK_THRESHOLD: u32 = 3;

pub struct AppTodayAgendaRepository<'a> {
    connection: &'a Connection,
}

impl<'a> AppTodayAgendaRepository<'a> {
    pub const fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub fn load(&self, farm_id: Option<FarmId>) -> Result<TodayAgendaProjection, AppSqliteError> {
        let Some(farm) = self.load_farm_summary(farm_id)? else {
            return Ok(TodayAgendaProjection::default());
        };

        Ok(TodayAgendaProjection {
            farm: Some(farm.clone()),
            summary: Some(self.load_today_summary(farm.farm_id)?),
            reminders: Default::default(),
            orders_needing_action: self.load_orders_needing_action(farm.farm_id)?,
            low_stock_products: self.load_low_stock_products(farm.farm_id)?,
            draft_products: self.load_draft_products(farm.farm_id)?,
            next_fulfillment_window: self.load_next_fulfillment_window(farm.farm_id)?,
            setup_checklist: self.load_setup_checklist(&farm)?,
        })
    }

    pub fn save_farm_summary(&self, farm: &FarmSummary) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "insert into farms (id, display_name, readiness, created_at, updated_at)
                 values (
                    ?1,
                    ?2,
                    ?3,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 )
                 on conflict(id) do update set
                    display_name = excluded.display_name,
                    readiness = excluded.readiness,
                    updated_at = excluded.updated_at",
                params![
                    farm.farm_id.to_string(),
                    farm.display_name,
                    farm_readiness_storage_key(farm.readiness),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "save today farm summary",
                source,
            })?;

        Ok(())
    }

    fn load_farm_summary(
        &self,
        farm_id: Option<FarmId>,
    ) -> Result<Option<FarmSummary>, AppSqliteError> {
        let farm_row = if let Some(farm_id) = farm_id {
            self.connection
                .query_row(
                    "select id, display_name, readiness from farms where id = ?1 limit 1",
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
                    operation: "load today farm summary",
                    source,
                })?
        } else {
            self.connection
                .query_row(
                    "select id, display_name, readiness from farms order by created_at asc, id asc limit 1",
                    [],
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
                    operation: "load today farm summary",
                    source,
                })?
        };

        farm_row
            .map(|(farm_id, display_name, readiness)| {
                Ok(FarmSummary {
                    farm_id: parse_typed_id("farms.id", farm_id)?,
                    display_name,
                    readiness: parse_farm_readiness("farms.readiness", readiness)?,
                })
            })
            .transpose()
    }

    fn load_today_summary(&self, farm_id: FarmId) -> Result<TodaySummary, AppSqliteError> {
        Ok(TodaySummary {
            farm_id,
            orders_needing_action: self.count_u32(
                "count today orders needing action",
                "select count(*) from orders where farm_id = ?1 and status = 'needs_action'",
                params![farm_id.to_string()],
            )?,
            low_stock_products: self.count_u32(
                "count today low-stock products",
                "select count(*) from products where farm_id = ?1 and status = 'published' and stock_count <= ?2",
                params![farm_id.to_string(), TODAY_AGENDA_LOW_STOCK_THRESHOLD],
            )?,
            draft_products: self.count_u32(
                "count today draft products",
                "select count(*) from products where farm_id = ?1 and status = 'draft'",
                params![farm_id.to_string()],
            )?,
            reminders_due_soon: 0,
            recovery_actions_open: 0,
        })
    }

    fn load_orders_needing_action(
        &self,
        farm_id: FarmId,
    ) -> Result<Vec<OrderListRow>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select id, fulfillment_window_id, order_number, customer_display_name \
                 from orders \
                 where farm_id = ?1 and status = 'needs_action' \
                 order by updated_at desc, id desc \
                 limit ?2",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare today orders needing action",
                source,
            })?;
        let rows = statement
            .query_map(
                params![farm_id.to_string(), TODAY_AGENDA_LIST_LIMIT],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "query today orders needing action",
                source,
            })?;
        let mut orders = Vec::new();

        for row in rows {
            let (order_id, fulfillment_window_id, order_number, customer_display_name) = row
                .map_err(|source| AppSqliteError::Query {
                    operation: "read today orders needing action",
                    source,
                })?;

            orders.push(OrderListRow {
                order_id: parse_typed_id("orders.id", order_id)?,
                farm_id,
                fulfillment_window_id: parse_optional_typed_id(
                    "orders.fulfillment_window_id",
                    fulfillment_window_id,
                )?,
                order_number,
                customer_display_name,
                status: OrderStatus::NeedsAction,
            });
        }

        Ok(orders)
    }

    fn load_low_stock_products(
        &self,
        farm_id: FarmId,
    ) -> Result<Vec<ProductListRow>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select id, title, coalesce(stock_count, 0) \
                 from products \
                 where farm_id = ?1 and status = 'published' and stock_count <= ?2 \
                 order by stock_count asc, updated_at desc, id desc \
                 limit ?3",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare today low-stock products",
                source,
            })?;
        let rows = statement
            .query_map(
                params![
                    farm_id.to_string(),
                    TODAY_AGENDA_LOW_STOCK_THRESHOLD,
                    TODAY_AGENDA_LIST_LIMIT
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, u32>(2)?,
                    ))
                },
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "query today low-stock products",
                source,
            })?;
        let mut products = Vec::new();

        for row in rows {
            let (product_id, title, stock_count) = row.map_err(|source| AppSqliteError::Query {
                operation: "read today low-stock products",
                source,
            })?;

            products.push(ProductListRow {
                product_id: parse_typed_id("products.id", product_id)?,
                farm_id,
                title,
                status: ProductStatus::Published,
                stock_count,
            });
        }

        Ok(products)
    }

    fn load_draft_products(&self, farm_id: FarmId) -> Result<Vec<ProductListRow>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select id, title, coalesce(stock_count, 0) \
                 from products \
                 where farm_id = ?1 and status = 'draft' \
                 order by updated_at desc, id desc \
                 limit ?2",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare today draft products",
                source,
            })?;
        let rows = statement
            .query_map(
                params![farm_id.to_string(), TODAY_AGENDA_LIST_LIMIT],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, u32>(2)?,
                    ))
                },
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "query today draft products",
                source,
            })?;
        let mut products = Vec::new();

        for row in rows {
            let (product_id, title, stock_count) = row.map_err(|source| AppSqliteError::Query {
                operation: "read today draft products",
                source,
            })?;

            products.push(ProductListRow {
                product_id: parse_typed_id("products.id", product_id)?,
                farm_id,
                title,
                status: ProductStatus::Draft,
                stock_count,
            });
        }

        Ok(products)
    }

    fn load_next_fulfillment_window(
        &self,
        farm_id: FarmId,
    ) -> Result<Option<FulfillmentWindowSummary>, AppSqliteError> {
        self.connection
            .query_row(
                "select id, starts_at, ends_at \
                 from fulfillment_windows \
                 where farm_id = ?1 and starts_at >= strftime('%Y-%m-%dT%H:%M:%SZ', 'now') \
                 order by starts_at asc, id asc \
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
                operation: "load today next fulfillment window",
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

    fn load_setup_checklist(
        &self,
        farm: &FarmSummary,
    ) -> Result<Vec<TodaySetupTask>, AppSqliteError> {
        if farm.readiness != FarmReadiness::Incomplete {
            return Ok(Vec::new());
        }

        Ok(vec![
            TodaySetupTask {
                kind: TodaySetupTaskKind::AddFulfillmentWindow,
                is_complete: self.exists(
                    "check today fulfillment window setup",
                    "select exists(select 1 from fulfillment_windows where farm_id = ?1)",
                    params![farm.farm_id.to_string()],
                )?,
            },
            TodaySetupTask {
                kind: TodaySetupTaskKind::PublishProduct,
                is_complete: self.exists(
                    "check today published product setup",
                    "select exists(select 1 from products where farm_id = ?1 and status = 'published')",
                    params![farm.farm_id.to_string()],
                )?,
            },
        ])
    }

    fn count_u32<P: Params>(
        &self,
        operation: &'static str,
        sql: &'static str,
        params: P,
    ) -> Result<u32, AppSqliteError> {
        self.connection
            .query_row(sql, params, |row| row.get::<_, u32>(0))
            .map_err(|source| AppSqliteError::Query { operation, source })
    }

    fn exists<P: Params>(
        &self,
        operation: &'static str,
        sql: &'static str,
        params: P,
    ) -> Result<bool, AppSqliteError> {
        self.connection
            .query_row(sql, params, |row| row.get::<_, i64>(0))
            .map(|value| value == 1)
            .map_err(|source| AppSqliteError::Query { operation, source })
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

fn parse_farm_readiness(
    field: &'static str,
    value: String,
) -> Result<FarmReadiness, AppSqliteError> {
    match value.as_str() {
        "incomplete" => Ok(FarmReadiness::Incomplete),
        "ready" => Ok(FarmReadiness::Ready),
        _ => Err(AppSqliteError::DecodeEnum { field, value }),
    }
}

fn farm_readiness_storage_key(readiness: FarmReadiness) -> &'static str {
    match readiness {
        FarmReadiness::Incomplete => "incomplete",
        FarmReadiness::Ready => "ready",
    }
}

#[cfg(test)]
mod tests {
    use radroots_studio_app_view::{FarmId, FulfillmentWindowId, ProductId, TodaySetupTaskKind};
    use rusqlite::{Connection, params};

    use crate::{AppSqliteStore, DatabaseTarget};

    use super::{TODAY_AGENDA_LIST_LIMIT, TODAY_AGENDA_LOW_STOCK_THRESHOLD};

    #[test]
    fn today_agenda_returns_default_when_no_farm_exists() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");

        let projection = store
            .load_today_agenda(None)
            .expect("empty today agenda should load");

        assert_eq!(
            projection,
            radroots_studio_app_view::TodayAgendaProjection::default()
        );
    }

    #[test]
    fn today_agenda_loads_truthful_projection_for_selected_farm() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let farm_id = FarmId::new();
        let other_farm_id = FarmId::new();
        let earliest_window_id = FulfillmentWindowId::new();
        let later_window_id = FulfillmentWindowId::new();

        insert_farm(
            connection,
            farm_id,
            "Willow Farm",
            "ready",
            "2026-04-17T08:00:00Z",
        );
        insert_farm(
            connection,
            other_farm_id,
            "Other Farm",
            "ready",
            "2026-04-18T08:00:00Z",
        );
        insert_window(
            connection,
            earliest_window_id,
            farm_id,
            "2099-04-18T16:00:00Z",
            "2099-04-18T18:00:00Z",
        );
        insert_window(
            connection,
            later_window_id,
            farm_id,
            "2099-04-19T16:00:00Z",
            "2099-04-19T18:00:00Z",
        );
        insert_window(
            connection,
            FulfillmentWindowId::new(),
            other_farm_id,
            "2099-04-17T10:00:00Z",
            "2099-04-17T12:00:00Z",
        );

        for index in 0..5 {
            insert_order(
                connection,
                farm_id,
                Some(earliest_window_id),
                &format!("R-10{index}"),
                "Casey",
                "needs_action",
                &format!("2026-04-17T0{index}:00:00Z"),
            );
        }
        insert_order(
            connection,
            farm_id,
            Some(earliest_window_id),
            "R-200",
            "Taylor",
            "scheduled",
            "2026-04-17T11:00:00Z",
        );
        insert_order(
            connection,
            other_farm_id,
            None,
            "R-999",
            "Other",
            "needs_action",
            "2026-04-17T12:00:00Z",
        );

        insert_product(
            connection,
            farm_id,
            "Carrots",
            "published",
            1,
            "2026-04-17T10:00:00Z",
        );
        insert_product(
            connection,
            farm_id,
            "Greens",
            "published",
            TODAY_AGENDA_LOW_STOCK_THRESHOLD,
            "2026-04-17T09:00:00Z",
        );
        insert_product(
            connection,
            farm_id,
            "Tomatoes",
            "published",
            TODAY_AGENDA_LOW_STOCK_THRESHOLD + 1,
            "2026-04-17T08:00:00Z",
        );
        for index in 0..5 {
            insert_product(
                connection,
                farm_id,
                &format!("Draft {index}"),
                "draft",
                0,
                &format!("2026-04-17T1{index}:00:00Z"),
            );
        }
        insert_product(
            connection,
            other_farm_id,
            "Other Draft",
            "draft",
            0,
            "2026-04-17T14:00:00Z",
        );

        let projection = store
            .load_today_agenda(Some(farm_id))
            .expect("today agenda should load");
        let summary = projection.summary.expect("summary should exist");
        let farm = projection.farm.expect("farm should exist");
        let next_window = projection
            .next_fulfillment_window
            .expect("next window should exist");

        assert_eq!(farm.farm_id, farm_id);
        assert_eq!(farm.display_name, "Willow Farm");
        assert_eq!(summary.orders_needing_action, 5);
        assert_eq!(summary.low_stock_products, 2);
        assert_eq!(summary.draft_products, 5);
        assert_eq!(
            projection.orders_needing_action.len() as i64,
            TODAY_AGENDA_LIST_LIMIT
        );
        assert_eq!(projection.orders_needing_action[0].order_number, "R-104");
        assert_eq!(projection.low_stock_products.len(), 2);
        assert_eq!(projection.low_stock_products[0].title, "Carrots");
        assert_eq!(projection.low_stock_products[1].title, "Greens");
        assert_eq!(
            projection.draft_products.len() as i64,
            TODAY_AGENDA_LIST_LIMIT
        );
        assert_eq!(projection.draft_products[0].title, "Draft 4");
        assert_eq!(next_window.fulfillment_window_id, earliest_window_id);
        assert_eq!(next_window.starts_at, "2099-04-18T16:00:00Z");
        assert!(projection.setup_checklist.is_empty());
    }

    #[test]
    fn today_agenda_uses_primary_farm_and_builds_setup_checklist_for_incomplete_farm() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let primary_farm_id = FarmId::new();
        let secondary_farm_id = FarmId::new();

        insert_farm(
            connection,
            primary_farm_id,
            "First Farm",
            "incomplete",
            "2026-04-17T08:00:00Z",
        );
        insert_farm(
            connection,
            secondary_farm_id,
            "Second Farm",
            "ready",
            "2026-04-18T08:00:00Z",
        );
        insert_product(
            connection,
            primary_farm_id,
            "Unpublished Lettuce",
            "draft",
            0,
            "2026-04-17T09:00:00Z",
        );
        insert_product(
            connection,
            secondary_farm_id,
            "Published Beets",
            "published",
            5,
            "2026-04-17T10:00:00Z",
        );
        insert_window(
            connection,
            FulfillmentWindowId::new(),
            secondary_farm_id,
            "2099-04-20T16:00:00Z",
            "2099-04-20T18:00:00Z",
        );

        let projection = store
            .load_today_agenda(None)
            .expect("default farm today agenda should load");
        let farm = projection.farm.expect("farm should exist");

        assert_eq!(farm.farm_id, primary_farm_id);
        assert_eq!(projection.summary.expect("summary").draft_products, 1);
        assert_eq!(projection.setup_checklist.len(), 2);
        assert_eq!(
            projection.setup_checklist[0].kind,
            TodaySetupTaskKind::AddFulfillmentWindow
        );
        assert!(!projection.setup_checklist[0].is_complete);
        assert_eq!(
            projection.setup_checklist[1].kind,
            TodaySetupTaskKind::PublishProduct
        );
        assert!(!projection.setup_checklist[1].is_complete);
        assert!(projection.next_fulfillment_window.is_none());
    }

    #[test]
    fn saved_farm_summary_round_trips_into_today_projection() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let farm = radroots_studio_app_view::FarmSummary {
            farm_id: FarmId::new(),
            display_name: "North field farm".to_owned(),
            readiness: radroots_studio_app_view::FarmReadiness::Incomplete,
        };

        store
            .save_farm_summary(&farm)
            .expect("farm summary should save");

        let projection = store
            .load_today_agenda(Some(farm.farm_id))
            .expect("today agenda should load");

        assert_eq!(projection.farm, Some(farm));
        assert_eq!(
            projection.summary.expect("summary").orders_needing_action,
            0
        );
        assert_eq!(projection.setup_checklist.len(), 2);
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
                "insert into farms (id, display_name, readiness, created_at, updated_at) \
                 values (?1, ?2, ?3, ?4, ?4)",
                params![farm_id.to_string(), display_name, readiness, created_at],
            )
            .expect("farm insert should succeed");
    }

    fn insert_window(
        connection: &Connection,
        fulfillment_window_id: FulfillmentWindowId,
        farm_id: FarmId,
        starts_at: &str,
        ends_at: &str,
    ) {
        connection
            .execute(
                "insert into fulfillment_windows (id, farm_id, starts_at, ends_at, capacity_limit, created_at, updated_at) \
                 values (?1, ?2, ?3, ?4, null, ?3, ?3)",
                params![
                    fulfillment_window_id.to_string(),
                    farm_id.to_string(),
                    starts_at,
                    ends_at
                ],
            )
            .expect("fulfillment window insert should succeed");
    }

    fn insert_product(
        connection: &Connection,
        farm_id: FarmId,
        title: &str,
        status: &str,
        stock_count: u32,
        updated_at: &str,
    ) -> ProductId {
        let product_id = ProductId::new();

        connection
            .execute(
                "insert into products (id, farm_id, title, status, stock_count, updated_at) \
                 values (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    product_id.to_string(),
                    farm_id.to_string(),
                    title,
                    status,
                    stock_count,
                    updated_at
                ],
            )
            .expect("product insert should succeed");

        product_id
    }

    fn insert_order(
        connection: &Connection,
        farm_id: FarmId,
        fulfillment_window_id: Option<FulfillmentWindowId>,
        order_number: &str,
        customer_display_name: &str,
        status: &str,
        updated_at: &str,
    ) {
        connection
            .execute(
                "insert into orders (id, farm_id, fulfillment_window_id, order_number, customer_display_name, status, updated_at) \
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    radroots_studio_app_view::OrderId::new().to_string(),
                    farm_id.to_string(),
                    fulfillment_window_id.map(|id| id.to_string()),
                    order_number,
                    customer_display_name,
                    status,
                    updated_at
                ],
            )
            .expect("order insert should succeed");
    }
}
