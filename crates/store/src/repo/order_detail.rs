use radroots_studio_app_view::{OrderDetailItemRow, ProductPricePresentation, TradeEconomicsProjection};

use crate::AppSqliteError;

pub(super) fn order_detail_item_row(
    title: String,
    quantity_display: String,
    quantity_value: i64,
    quantity_unit_label: String,
    unit_price_minor_units: Option<u32>,
    price_currency: Option<String>,
) -> Result<OrderDetailItemRow, AppSqliteError> {
    let quantity =
        u32::try_from(quantity_value).map_err(|_| AppSqliteError::InvalidProjection {
            reason: "order detail item quantity must be non-negative",
        })?;
    let currency_code = price_currency
        .as_deref()
        .map(normalize_currency_code)
        .unwrap_or_else(|| normalize_currency_code(""));
    let line_total_minor_units = unit_price_minor_units
        .map(|amount| {
            amount
                .checked_mul(quantity)
                .ok_or(AppSqliteError::InvalidProjection {
                    reason: "order detail line total overflowed",
                })
        })
        .transpose()?;
    let unit_price = unit_price_minor_units.map(|amount_minor_units| ProductPricePresentation {
        amount_minor_units,
        currency_code,
        unit_label: quantity_unit_label.trim().to_owned(),
    });

    Ok(OrderDetailItemRow {
        title,
        quantity_display,
        unit_price,
        line_total_minor_units,
    })
}

pub(super) fn order_detail_economics(
    items: &[OrderDetailItemRow],
) -> Result<TradeEconomicsProjection, AppSqliteError> {
    let mut total_minor_units = 0_u32;
    let mut currency_code = None::<String>;

    for item in items {
        let (Some(unit_price), Some(line_total_minor_units)) =
            (item.unit_price.as_ref(), item.line_total_minor_units)
        else {
            return Ok(TradeEconomicsProjection::default());
        };
        if let Some(existing_currency) = currency_code.as_deref() {
            if existing_currency != unit_price.currency_code.as_str() {
                return Ok(TradeEconomicsProjection::default());
            }
        } else {
            currency_code = Some(unit_price.currency_code.clone());
        }
        total_minor_units = total_minor_units
            .checked_add(line_total_minor_units)
            .ok_or(AppSqliteError::InvalidProjection {
                reason: "order detail total overflowed",
            })?;
    }

    Ok(
        currency_code.map_or_else(TradeEconomicsProjection::default, |currency_code| {
            TradeEconomicsProjection {
                subtotal_minor_units: Some(total_minor_units),
                discount_total_minor_units: None,
                adjustment_total_minor_units: None,
                total_minor_units: Some(total_minor_units),
                currency_code: Some(currency_code),
            }
        }),
    )
}

fn normalize_currency_code(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "USD".to_owned()
    } else {
        trimmed.to_ascii_uppercase()
    }
}
