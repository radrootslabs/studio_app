use std::cmp::Ordering;

use radroots_studio_app_models::{
    FarmId, FulfillmentWindowId, ProductAttentionState, ProductAvailabilityState,
    ProductAvailabilitySummary, ProductEditorDraft, ProductId, ProductPricePresentation,
    ProductPublishBlocker, ProductStatus, ProductStockState, ProductStockSummary, ProductsFilter,
    ProductsListProjection, ProductsListRow, ProductsListSummary, ProductsSort,
};
use rusqlite::{Connection, OptionalExtension, params};

use crate::AppSqliteError;

const PRODUCTS_LOW_STOCK_THRESHOLD: u32 = 3;

pub struct AppProductsRepository<'a> {
    connection: &'a Connection,
}

impl<'a> AppProductsRepository<'a> {
    pub const fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub fn load_products(
        &self,
        farm_id: FarmId,
        search_query: &str,
        filter: ProductsFilter,
        sort: ProductsSort,
    ) -> Result<ProductsListProjection, AppSqliteError> {
        let now_utc = self.current_utc_timestamp()?;
        let mut records = self.load_product_records(farm_id)?;
        let summary = summarize_records(&records, &now_utc);
        let normalized_search = normalize_search_query(search_query);

        records.retain(|record| {
            record.matches_search(normalized_search.as_deref())
                && record.matches_filter(filter, &now_utc)
        });
        sort_records(&mut records, sort, &now_utc);

        Ok(ProductsListProjection {
            summary,
            rows: records
                .into_iter()
                .map(|record| record.into_list_row(&now_utc))
                .collect(),
        })
    }

    pub fn load_product_editor_draft(
        &self,
        product_id: ProductId,
    ) -> Result<Option<ProductEditorDraft>, AppSqliteError> {
        self.load_product_record_by_id(product_id)?
            .map(|record| Ok(record.into_editor_draft()))
            .transpose()
    }

    pub fn create_product_draft(&self, farm_id: FarmId) -> Result<ProductId, AppSqliteError> {
        let product_id = ProductId::new();

        self.connection
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
                 ) values (
                    ?1,
                    ?2,
                    '',
                    '',
                    'draft',
                    '',
                    null,
                    'USD',
                    null,
                    null,
                    strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 )",
                params![product_id.to_string(), farm_id.to_string()],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "create product draft",
                source,
            })?;

        Ok(product_id)
    }

    pub fn save_product_editor_draft(
        &self,
        product_id: ProductId,
        draft: &ProductEditorDraft,
    ) -> Result<bool, AppSqliteError> {
        let updated_rows = self
            .connection
            .execute(
                "update products
                 set
                    title = ?2,
                    subtitle = ?3,
                    status = ?4,
                    unit_label = ?5,
                    price_minor_units = ?6,
                    price_currency = ?7,
                    stock_count = ?8,
                    availability_window_id = ?9,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 where id = ?1",
                params![
                    product_id.to_string(),
                    draft.title.as_str(),
                    draft.subtitle.as_str(),
                    draft.status.storage_key(),
                    draft.unit_label.as_str(),
                    draft.price_minor_units,
                    normalize_currency_code(&draft.price_currency),
                    draft.stock_quantity,
                    draft.availability_window_id.map(|id| id.to_string()),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "save product editor draft",
                source,
            })?;

        Ok(updated_rows > 0)
    }

    pub fn update_product_stock(
        &self,
        product_id: ProductId,
        stock_quantity: u32,
    ) -> Result<bool, AppSqliteError> {
        let updated_rows = self
            .connection
            .execute(
                "update products
                 set
                    stock_count = ?2,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 where id = ?1",
                params![product_id.to_string(), stock_quantity],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "update product stock",
                source,
            })?;

        Ok(updated_rows > 0)
    }

    pub fn evaluate_product_publish_blockers(
        &self,
        product_id: ProductId,
    ) -> Result<Option<Vec<ProductPublishBlocker>>, AppSqliteError> {
        self.load_product_editor_draft(product_id)?
            .map(|draft| Ok(draft.publish_blockers()))
            .transpose()
    }

    fn current_utc_timestamp(&self) -> Result<String, AppSqliteError> {
        self.connection
            .query_row("select strftime('%Y-%m-%dT%H:%M:%SZ', 'now')", [], |row| {
                row.get(0)
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "load current utc timestamp",
                source,
            })
    }

    fn load_product_records(&self, farm_id: FarmId) -> Result<Vec<ProductRecord>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select
                    p.id,
                    p.farm_id,
                    p.title,
                    p.subtitle,
                    p.status,
                    p.unit_label,
                    p.price_minor_units,
                    p.price_currency,
                    p.stock_count,
                    p.availability_window_id,
                    fw.starts_at,
                    fw.ends_at,
                    p.updated_at
                 from products p
                 left join fulfillment_windows fw on fw.id = p.availability_window_id
                 where p.farm_id = ?1",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare products list query",
                source,
            })?;
        let rows = statement
            .query_map(params![farm_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<u32>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, Option<u32>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, String>(12)?,
                ))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query products list",
                source,
            })?;
        let mut records = Vec::new();

        for row in rows {
            let (
                product_id,
                farm_id,
                title,
                subtitle,
                status,
                unit_label,
                price_minor_units,
                price_currency,
                stock_count,
                availability_window_id,
                availability_starts_at,
                availability_ends_at,
                updated_at,
            ) = row.map_err(|source| AppSqliteError::Query {
                operation: "read products list",
                source,
            })?;

            records.push(ProductRecord {
                product_id: parse_typed_id("products.id", product_id)?,
                farm_id: parse_typed_id("products.farm_id", farm_id)?,
                title,
                subtitle,
                status: parse_product_status("products.status", status)?,
                unit_label,
                price_minor_units,
                price_currency,
                stock_count,
                availability_window_id: parse_optional_typed_id(
                    "products.availability_window_id",
                    availability_window_id,
                )?,
                availability_starts_at,
                availability_ends_at,
                updated_at,
            });
        }

        Ok(records)
    }

    fn load_product_record_by_id(
        &self,
        product_id: ProductId,
    ) -> Result<Option<ProductRecord>, AppSqliteError> {
        let row = self
            .connection
            .query_row(
                "select
                    p.id,
                    p.farm_id,
                    p.title,
                    p.subtitle,
                    p.status,
                    p.unit_label,
                    p.price_minor_units,
                    p.price_currency,
                    p.stock_count,
                    p.availability_window_id,
                    fw.starts_at,
                    fw.ends_at,
                    p.updated_at
                 from products p
                 left join fulfillment_windows fw on fw.id = p.availability_window_id
                 where p.id = ?1
                 limit 1",
                params![product_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, Option<u32>>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, Option<u32>>(8)?,
                        row.get::<_, Option<String>>(9)?,
                        row.get::<_, Option<String>>(10)?,
                        row.get::<_, Option<String>>(11)?,
                        row.get::<_, String>(12)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load product editor draft",
                source,
            })?;

        row.map(
            |(
                product_id,
                farm_id,
                title,
                subtitle,
                status,
                unit_label,
                price_minor_units,
                price_currency,
                stock_count,
                availability_window_id,
                availability_starts_at,
                availability_ends_at,
                updated_at,
            )| {
                Ok(ProductRecord {
                    product_id: parse_typed_id("products.id", product_id)?,
                    farm_id: parse_typed_id("products.farm_id", farm_id)?,
                    title,
                    subtitle,
                    status: parse_product_status("products.status", status)?,
                    unit_label,
                    price_minor_units,
                    price_currency,
                    stock_count,
                    availability_window_id: parse_optional_typed_id(
                        "products.availability_window_id",
                        availability_window_id,
                    )?,
                    availability_starts_at,
                    availability_ends_at,
                    updated_at,
                })
            },
        )
        .transpose()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProductRecord {
    product_id: ProductId,
    farm_id: FarmId,
    title: String,
    subtitle: String,
    status: ProductStatus,
    unit_label: String,
    price_minor_units: Option<u32>,
    price_currency: String,
    stock_count: Option<u32>,
    availability_window_id: Option<FulfillmentWindowId>,
    availability_starts_at: Option<String>,
    availability_ends_at: Option<String>,
    updated_at: String,
}

impl ProductRecord {
    fn into_list_row(self, now_utc: &str) -> ProductsListRow {
        let availability = self.availability_summary(now_utc);
        let attention_state = self.attention_state(now_utc, &availability);
        let stock = self.stock_summary();
        let price = self.price_presentation();

        ProductsListRow {
            product_id: self.product_id,
            farm_id: self.farm_id,
            title: self.title,
            subtitle: empty_string_to_none(self.subtitle),
            status: self.status,
            attention_state,
            availability,
            stock,
            price,
            updated_at: self.updated_at,
        }
    }

    fn into_editor_draft(self) -> ProductEditorDraft {
        ProductEditorDraft {
            title: self.title,
            subtitle: self.subtitle,
            unit_label: self.unit_label,
            price_minor_units: self.price_minor_units,
            price_currency: self.price_currency,
            stock_quantity: self.stock_count,
            availability_window_id: self.availability_window_id,
            status: self.status,
        }
    }

    fn matches_search(&self, search_query: Option<&str>) -> bool {
        let Some(search_query) = search_query else {
            return true;
        };

        self.title.to_lowercase().contains(search_query)
            || self.subtitle.to_lowercase().contains(search_query)
    }

    fn matches_filter(&self, filter: ProductsFilter, now_utc: &str) -> bool {
        match filter {
            ProductsFilter::All => true,
            ProductsFilter::Live => self.status == ProductStatus::Published,
            ProductsFilter::Drafts => self.status == ProductStatus::Draft,
            ProductsFilter::NeedAttention => self
                .attention_state(now_utc, &self.availability_summary(now_utc))
                .requires_attention(),
            ProductsFilter::Paused => self.status == ProductStatus::Paused,
            ProductsFilter::Archived => self.status == ProductStatus::Archived,
        }
    }

    fn availability_summary(&self, now_utc: &str) -> ProductAvailabilitySummary {
        match (&self.availability_starts_at, &self.availability_ends_at) {
            (Some(starts_at), Some(ends_at))
                if starts_at.as_str() <= now_utc && ends_at.as_str() >= now_utc =>
            {
                ProductAvailabilitySummary {
                    state: ProductAvailabilityState::Open,
                    label: "Pickup open".to_owned(),
                }
            }
            (Some(starts_at), Some(ends_at)) if starts_at.as_str() > now_utc => {
                ProductAvailabilitySummary {
                    state: ProductAvailabilityState::Scheduled,
                    label: format_window_label(starts_at, ends_at),
                }
            }
            (Some(_), Some(_)) => ProductAvailabilitySummary {
                state: ProductAvailabilityState::NoFutureWindow,
                label: "No future slot".to_owned(),
            },
            _ => ProductAvailabilitySummary {
                state: ProductAvailabilityState::MissingWindow,
                label: "Missing window".to_owned(),
            },
        }
    }

    fn attention_state(
        &self,
        now_utc: &str,
        availability: &ProductAvailabilitySummary,
    ) -> ProductAttentionState {
        if matches!(self.status, ProductStatus::Paused | ProductStatus::Archived) {
            return ProductAttentionState::Healthy;
        }

        let stock = self.stock_summary();
        if self.status == ProductStatus::Published {
            if stock.state == ProductStockState::SoldOut {
                return ProductAttentionState::SoldOut;
            }

            if stock.state == ProductStockState::LowStock {
                return ProductAttentionState::LowStock;
            }
        }

        match availability.state {
            ProductAvailabilityState::MissingWindow => {
                return ProductAttentionState::MissingAvailability;
            }
            ProductAvailabilityState::NoFutureWindow => {
                return ProductAttentionState::NoFutureAvailability;
            }
            ProductAvailabilityState::Scheduled | ProductAvailabilityState::Open => {}
        }

        if self
            .editor_draft_publish_blockers(now_utc)
            .into_iter()
            .any(|blocker| blocker != ProductPublishBlocker::AttachAvailability)
        {
            return ProductAttentionState::MissingDetails;
        }

        ProductAttentionState::Healthy
    }

    fn stock_summary(&self) -> ProductStockSummary {
        let state = match self.stock_count {
            None => ProductStockState::Unset,
            Some(0) => ProductStockState::SoldOut,
            Some(quantity) if quantity <= PRODUCTS_LOW_STOCK_THRESHOLD => {
                ProductStockState::LowStock
            }
            Some(_) => ProductStockState::InStock,
        };

        ProductStockSummary {
            quantity: self.stock_count,
            unit_label: empty_string_to_none(self.unit_label.clone()),
            state,
        }
    }

    fn price_presentation(&self) -> Option<ProductPricePresentation> {
        self.price_minor_units
            .filter(|amount| *amount > 0)
            .zip(empty_string_to_none(self.unit_label.clone()))
            .map(
                |(amount_minor_units, unit_label)| ProductPricePresentation {
                    amount_minor_units,
                    currency_code: self.price_currency.clone(),
                    unit_label,
                },
            )
    }

    fn editor_draft_publish_blockers(&self, _now_utc: &str) -> Vec<ProductPublishBlocker> {
        self.clone().into_editor_draft().publish_blockers()
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

fn summarize_records(records: &[ProductRecord], now_utc: &str) -> ProductsListSummary {
    ProductsListSummary {
        total_products: records.len() as u32,
        live_products: records
            .iter()
            .filter(|record| record.status == ProductStatus::Published)
            .count() as u32,
        draft_products: records
            .iter()
            .filter(|record| record.status == ProductStatus::Draft)
            .count() as u32,
        need_attention_products: records
            .iter()
            .filter(|record| {
                record
                    .attention_state(now_utc, &record.availability_summary(now_utc))
                    .requires_attention()
            })
            .count() as u32,
    }
}

fn sort_records(records: &mut [ProductRecord], sort: ProductsSort, now_utc: &str) {
    records.sort_by(|left, right| match sort {
        ProductsSort::Updated => right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| right.product_id.cmp(&left.product_id)),
        ProductsSort::Name => lower_cmp(&left.title, &right.title)
            .then_with(|| lower_cmp(&left.subtitle, &right.subtitle))
            .then_with(|| left.product_id.cmp(&right.product_id)),
        ProductsSort::Availability => availability_rank(left, now_utc)
            .cmp(&availability_rank(right, now_utc))
            .then_with(|| {
                option_string_cmp(&left.availability_starts_at, &right.availability_starts_at)
            })
            .then_with(|| lower_cmp(&left.title, &right.title))
            .then_with(|| left.product_id.cmp(&right.product_id)),
        ProductsSort::Stock => stock_quantity_rank(left)
            .cmp(&stock_quantity_rank(right))
            .then_with(|| lower_cmp(&left.title, &right.title))
            .then_with(|| left.product_id.cmp(&right.product_id)),
        ProductsSort::Price => price_rank(left)
            .cmp(&price_rank(right))
            .then_with(|| lower_cmp(&left.title, &right.title))
            .then_with(|| left.product_id.cmp(&right.product_id)),
    });
}

fn availability_rank(record: &ProductRecord, now_utc: &str) -> (u8, Option<String>) {
    let availability = record.availability_summary(now_utc);
    let rank = match availability.state {
        ProductAvailabilityState::Open => 0,
        ProductAvailabilityState::Scheduled => 1,
        ProductAvailabilityState::MissingWindow => 2,
        ProductAvailabilityState::NoFutureWindow => 3,
    };

    (rank, record.availability_starts_at.clone())
}

fn stock_quantity_rank(record: &ProductRecord) -> (u8, u32) {
    match record.stock_count {
        Some(quantity) => (0, quantity),
        None => (1, u32::MAX),
    }
}

fn price_rank(record: &ProductRecord) -> (u8, u32) {
    match record.price_minor_units.filter(|amount| *amount > 0) {
        Some(amount_minor_units) => (0, amount_minor_units),
        None => (1, u32::MAX),
    }
}

fn lower_cmp(left: &str, right: &str) -> Ordering {
    left.to_lowercase().cmp(&right.to_lowercase())
}

fn option_string_cmp(left: &Option<String>, right: &Option<String>) -> Ordering {
    match (left.as_deref(), right.as_deref()) {
        (Some(left), Some(right)) => left.cmp(right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
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

fn normalize_currency_code(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "USD".to_owned()
    } else {
        trimmed.to_ascii_uppercase()
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

#[cfg(test)]
mod tests {
    use radroots_studio_app_models::{
        FarmId, FulfillmentWindowId, ProductAttentionState, ProductAvailabilityState,
        ProductEditorDraft, ProductId, ProductPublishBlocker, ProductStatus, ProductStockState,
        ProductsFilter, ProductsSort,
    };
    use rusqlite::{Connection, params};

    use crate::{AppSqliteStore, DatabaseTarget};

    use super::AppProductsRepository;

    #[test]
    fn products_list_projection_is_typed_and_supports_search_filter_and_sort() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let repository = AppProductsRepository::new(connection);
        let farm_id = insert_farm(connection, "North Meadow Farm");
        let future_window_id = insert_window(
            connection,
            farm_id,
            "2099-04-18T16:00:00Z",
            "2099-04-18T18:00:00Z",
        );

        insert_product(
            connection,
            farm_id,
            SeedProduct {
                title: "Salad mix",
                subtitle: "Spring blend",
                status: "published",
                unit_label: "box",
                price_minor_units: Some(600),
                stock_count: Some(2),
                availability_window_id: Some(future_window_id),
                updated_at: "2026-04-18T10:00:00Z",
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
                price_minor_units: Some(300),
                stock_count: None,
                availability_window_id: None,
                updated_at: "2026-04-18T09:00:00Z",
            },
        );
        insert_product(
            connection,
            farm_id,
            SeedProduct {
                title: "Heirloom tomatoes",
                subtitle: "Brandywine",
                status: "published",
                unit_label: "lb",
                price_minor_units: Some(450),
                stock_count: Some(12),
                availability_window_id: Some(future_window_id),
                updated_at: "2026-04-18T08:00:00Z",
            },
        );
        insert_product(
            connection,
            farm_id,
            SeedProduct {
                title: "Carrot bunches",
                subtitle: "Nantes",
                status: "paused",
                unit_label: "each",
                price_minor_units: Some(400),
                stock_count: None,
                availability_window_id: None,
                updated_at: "2026-04-18T07:00:00Z",
            },
        );
        insert_product(
            connection,
            farm_id,
            SeedProduct {
                title: "Old beets",
                subtitle: "",
                status: "archived",
                unit_label: "bunch",
                price_minor_units: Some(250),
                stock_count: Some(4),
                availability_window_id: None,
                updated_at: "2026-04-18T06:00:00Z",
            },
        );

        let all_products = repository
            .load_products(farm_id, "", ProductsFilter::All, ProductsSort::Updated)
            .expect("products list should load");
        let attention_products = repository
            .load_products(
                farm_id,
                "",
                ProductsFilter::NeedAttention,
                ProductsSort::Name,
            )
            .expect("attention products should load");
        let searched_products = repository
            .load_products(farm_id, "pea", ProductsFilter::All, ProductsSort::Name)
            .expect("searched products should load");

        assert_eq!(all_products.summary.total_products, 5);
        assert_eq!(all_products.summary.live_products, 2);
        assert_eq!(all_products.summary.draft_products, 1);
        assert_eq!(all_products.summary.need_attention_products, 2);
        assert_eq!(all_products.rows[0].title, "Salad mix");
        assert_eq!(all_products.rows[1].title, "Pea shoots");
        assert_eq!(
            all_products.rows[0].attention_state,
            ProductAttentionState::LowStock
        );
        assert_eq!(
            all_products.rows[1].attention_state,
            ProductAttentionState::MissingAvailability
        );
        assert_eq!(
            all_products.rows[2].availability.state,
            ProductAvailabilityState::Scheduled
        );
        assert_eq!(
            attention_products
                .rows
                .iter()
                .map(|row| row.title.as_str())
                .collect::<Vec<_>>(),
            vec!["Pea shoots", "Salad mix"]
        );
        assert_eq!(searched_products.rows.len(), 1);
        assert_eq!(searched_products.rows[0].title, "Pea shoots");
        assert_eq!(
            searched_products.rows[0].subtitle.as_deref(),
            Some("Tray-grown")
        );
    }

    #[test]
    fn product_editor_draft_round_trips_through_sqlite() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let repository = AppProductsRepository::new(connection);
        let farm_id = insert_farm(connection, "Willow Farm");
        let window_id = insert_window(
            connection,
            farm_id,
            "2099-04-20T16:00:00Z",
            "2099-04-20T18:00:00Z",
        );
        let product_id = repository
            .create_product_draft(farm_id)
            .expect("draft product should create");
        let saved_draft = ProductEditorDraft {
            title: "Heirloom tomatoes".to_owned(),
            subtitle: "Brandywine".to_owned(),
            unit_label: "lb".to_owned(),
            price_minor_units: Some(450),
            price_currency: "usd".to_owned(),
            stock_quantity: Some(12),
            availability_window_id: Some(window_id),
            status: ProductStatus::Published,
        };

        let created_draft = repository
            .load_product_editor_draft(product_id)
            .expect("editor draft should load")
            .expect("created product should exist");

        assert_eq!(created_draft, ProductEditorDraft::default());
        assert!(
            repository
                .save_product_editor_draft(product_id, &saved_draft)
                .expect("editor draft should save")
        );

        let reloaded_draft = repository
            .load_product_editor_draft(product_id)
            .expect("reloaded draft should load")
            .expect("saved product should exist");

        assert_eq!(
            reloaded_draft,
            ProductEditorDraft {
                price_currency: "USD".to_owned(),
                ..saved_draft
            }
        );
    }

    #[test]
    fn stock_updates_and_publish_blockers_are_truthful() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        let repository = AppProductsRepository::new(connection);
        let farm_id = insert_farm(connection, "Oak Farm");
        let window_id = insert_window(
            connection,
            farm_id,
            "2099-04-22T16:00:00Z",
            "2099-04-22T18:00:00Z",
        );
        let product_id = repository
            .create_product_draft(farm_id)
            .expect("draft product should create");

        assert_eq!(
            repository
                .evaluate_product_publish_blockers(product_id)
                .expect("blockers should load"),
            Some(vec![
                ProductPublishBlocker::AddProductName,
                ProductPublishBlocker::ChooseUnit,
                ProductPublishBlocker::SetPrice,
                ProductPublishBlocker::AttachAvailability,
            ])
        );

        assert!(
            repository
                .save_product_editor_draft(
                    product_id,
                    &ProductEditorDraft {
                        title: "Salad mix".to_owned(),
                        subtitle: "Spring blend".to_owned(),
                        unit_label: "box".to_owned(),
                        price_minor_units: Some(600),
                        price_currency: "USD".to_owned(),
                        stock_quantity: None,
                        availability_window_id: Some(window_id),
                        status: ProductStatus::Published,
                    },
                )
                .expect("ready draft should save")
        );
        assert_eq!(
            repository
                .evaluate_product_publish_blockers(product_id)
                .expect("ready blockers should load"),
            Some(Vec::new())
        );

        assert!(
            repository
                .update_product_stock(product_id, 0)
                .expect("stock should update")
        );
        let sold_out_row = repository
            .load_products(farm_id, "", ProductsFilter::All, ProductsSort::Updated)
            .expect("products should load")
            .rows
            .into_iter()
            .find(|row| row.product_id == product_id)
            .expect("saved product row should exist");

        assert_eq!(sold_out_row.stock.quantity, Some(0));
        assert_eq!(sold_out_row.stock.state, ProductStockState::SoldOut);
        assert_eq!(sold_out_row.attention_state, ProductAttentionState::SoldOut);

        assert!(
            repository
                .update_product_stock(product_id, 8)
                .expect("stock should update again")
        );
        let restocked_row = repository
            .load_products(farm_id, "", ProductsFilter::All, ProductsSort::Updated)
            .expect("restocked products should load")
            .rows
            .into_iter()
            .find(|row| row.product_id == product_id)
            .expect("restocked product row should exist");

        assert_eq!(restocked_row.stock.quantity, Some(8));
        assert_eq!(restocked_row.stock.state, ProductStockState::InStock);
        assert_eq!(
            restocked_row.attention_state,
            ProductAttentionState::Healthy
        );
    }

    fn insert_farm(connection: &Connection, display_name: &str) -> FarmId {
        let farm_id = FarmId::new();

        connection
            .execute(
                "insert into farms (id, display_name, readiness, created_at, updated_at)
                 values (?1, ?2, 'ready', '2026-04-18T08:00:00Z', '2026-04-18T08:00:00Z')",
                params![farm_id.to_string(), display_name],
            )
            .expect("farm insert should succeed");

        farm_id
    }

    fn insert_window(
        connection: &Connection,
        farm_id: FarmId,
        starts_at: &str,
        ends_at: &str,
    ) -> FulfillmentWindowId {
        let window_id = FulfillmentWindowId::new();

        connection
            .execute(
                "insert into fulfillment_windows (id, farm_id, starts_at, ends_at, capacity_limit, created_at, updated_at)
                 values (?1, ?2, ?3, ?4, null, ?3, ?3)",
                params![window_id.to_string(), farm_id.to_string(), starts_at, ends_at],
            )
            .expect("fulfillment window insert should succeed");

        window_id
    }

    fn insert_product(
        connection: &Connection,
        farm_id: FarmId,
        seed: SeedProduct<'_>,
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
                 ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'USD', ?8, ?9, ?10)",
                params![
                    product_id.to_string(),
                    farm_id.to_string(),
                    seed.title,
                    seed.subtitle,
                    seed.status,
                    seed.unit_label,
                    seed.price_minor_units,
                    seed.stock_count,
                    seed.availability_window_id.map(|id| id.to_string()),
                    seed.updated_at,
                ],
            )
            .expect("product insert should succeed");

        product_id
    }

    struct SeedProduct<'a> {
        title: &'a str,
        subtitle: &'a str,
        status: &'a str,
        unit_label: &'a str,
        price_minor_units: Option<u32>,
        stock_count: Option<u32>,
        availability_window_id: Option<FulfillmentWindowId>,
        updated_at: &'a str,
    }
}
