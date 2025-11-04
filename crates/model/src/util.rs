use crate::error::ModelError;
use crate::types::BindValue;

pub fn parse_query_value(value: &serde_json::Value) -> Result<BindValue, ModelError> {
    match value {
        serde_json::Value::String(s) => Ok(BindValue::String(s.clone())),
        serde_json::Value::Number(n) => {
            Ok(BindValue::Number(n.as_f64().ok_or_else(|| {
                ModelError::InvalidArgument("invalid number".to_string())
            })?))
        }
        serde_json::Value::Bool(b) => Ok(BindValue::Boolean(*b)),
        serde_json::Value::Null => Ok(BindValue::Null),
        _ => Err(ModelError::InvalidArgument(
            "Unsupported bind value".to_string(),
        )),
    }
}
