pub type DatabaseConnection = sqlx::Pool<sqlx::Sqlite>;

#[derive(Debug)]
pub enum ModelsQueryBindValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
}
