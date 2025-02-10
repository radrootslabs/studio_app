use crate::{error::ModelError, types::DatabaseConnection};

pub fn parse_query_value(value: &serde_json::Value) -> Result<String, ModelError> {
    match value {
        serde_json::Value::String(s) => Ok(s.to_string()),
        serde_json::Value::Number(n) if n.is_i64() => Ok(n.to_string()),
        serde_json::Value::Bool(b) => Ok((*b as u8).to_string()),
        _ => Err(ModelError::InvalidArgument(format!(
            "Unsupported value type: {:?}",
            value
        ))),
    }
}

pub async fn db_conn(db_path: std::path::PathBuf) -> DatabaseConnection {
    sqlx::sqlite::SqlitePoolOptions::new()
        .connect(db_path.to_str().unwrap())
        .await
        .unwrap()
}
