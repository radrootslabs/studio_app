use radroots_studio_app_view::{
    OrderDetailItemRow, OrderId, ProductPricePresentation, TradeEconomicsProjection,
    TradeValidationReceiptProjection, TradeValidationReceiptProofSystem,
    TradeValidationReceiptResult, TradeValidationReceiptType,
};
use sqlx::Row;

use crate::AppSqliteDatabase;

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

pub(super) fn order_validation_receipts(
    connection: &AppSqliteDatabase,
    order_id: OrderId,
) -> Result<Vec<TradeValidationReceiptProjection>, AppSqliteError> {
    let mut statement = connection
        .prepare(
            "SELECT
                event_id,
                result,
                receipt_type,
                proof_system,
                event_set_root,
                reducer_output_root,
                public_values_hash,
                target_event_id,
                event_created_at
             FROM order_validation_receipts
             WHERE order_id = ?1
             ORDER BY event_created_at DESC, event_id DESC",
        )
        .map_err(|source| AppSqliteError::Query {
            operation: "prepare order validation receipts",
            source,
        })?;
    let rows = statement
        .query_map(crate::app_sqlite_params![order_id.to_string()], |row| {
            Ok((
                row.try_get::<String, _>(0)?,
                row.try_get::<String, _>(1)?,
                row.try_get::<String, _>(2)?,
                row.try_get::<String, _>(3)?,
                row.try_get::<String, _>(4)?,
                row.try_get::<String, _>(5)?,
                row.try_get::<String, _>(6)?,
                row.try_get::<String, _>(7)?,
                row.try_get::<i64, _>(8)?,
            ))
        })
        .map_err(|source| AppSqliteError::Query {
            operation: "query order validation receipts",
            source,
        })?;
    let mut receipts = Vec::new();

    for row in rows {
        let (
            event_id,
            result,
            receipt_type,
            proof_system,
            event_set_root,
            reducer_output_root,
            public_values_hash,
            target_event_id,
            event_created_at,
        ) = row.map_err(|source| AppSqliteError::Query {
            operation: "read order validation receipt",
            source,
        })?;

        receipts.push(TradeValidationReceiptProjection {
            event_id,
            result: parse_validation_receipt_result(result)?,
            receipt_type: parse_validation_receipt_type(receipt_type)?,
            proof_system: parse_validation_receipt_proof_system(proof_system)?,
            event_set_root,
            reducer_output_root,
            public_values_hash,
            target_event_id,
            recorded_at: u64::try_from(event_created_at).map_err(|_| {
                AppSqliteError::InvalidProjection {
                    reason: "order_validation_receipts.event_created_at must be non-negative",
                }
            })?,
        });
    }

    Ok(receipts)
}

fn normalize_currency_code(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "USD".to_owned()
    } else {
        trimmed.to_ascii_uppercase()
    }
}

fn parse_validation_receipt_result(
    value: String,
) -> Result<TradeValidationReceiptResult, AppSqliteError> {
    match value.as_str() {
        "valid" => Ok(TradeValidationReceiptResult::Valid),
        "needs_review" => Ok(TradeValidationReceiptResult::NeedsReview),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "order_validation_receipts.result",
            value,
        }),
    }
}

fn parse_validation_receipt_type(
    value: String,
) -> Result<TradeValidationReceiptType, AppSqliteError> {
    match value.as_str() {
        "listing_validation" => Ok(TradeValidationReceiptType::ListingValidation),
        "trade_transition" => Ok(TradeValidationReceiptType::TradeTransition),
        "inventory_state" => Ok(TradeValidationReceiptType::InventoryState),
        "state_checkpoint" => Ok(TradeValidationReceiptType::StateCheckpoint),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "order_validation_receipts.receipt_type",
            value,
        }),
    }
}

fn parse_validation_receipt_proof_system(
    value: String,
) -> Result<TradeValidationReceiptProofSystem, AppSqliteError> {
    match value.as_str() {
        "none" => Ok(TradeValidationReceiptProofSystem::None),
        "sp1_core" => Ok(TradeValidationReceiptProofSystem::Sp1Core),
        "sp1_compressed" => Ok(TradeValidationReceiptProofSystem::Sp1Compressed),
        "sp1_groth16" => Ok(TradeValidationReceiptProofSystem::Sp1Groth16),
        "sp1_plonk" => Ok(TradeValidationReceiptProofSystem::Sp1Plonk),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "order_validation_receipts.proof_system",
            value,
        }),
    }
}
