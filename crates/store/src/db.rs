#![forbid(unsafe_code)]

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use sqlx::{
    Connection,
    sqlite::{SqliteArguments, SqliteConnectOptions, SqliteConnection, SqliteRow},
};

#[derive(Clone, Debug)]
pub(crate) enum AppSqliteValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
}

pub(crate) type AppSqliteParams = Vec<AppSqliteValue>;

pub(crate) trait IntoAppSqliteParams {
    fn into_app_sqlite_params(self) -> AppSqliteParams;
}

pub(crate) trait OptionalSqliteResult<T> {
    fn optional(self) -> Result<Option<T>, sqlx::Error>;
}

pub(crate) trait ToAppSqliteValue {
    fn to_app_sqlite_value(&self) -> AppSqliteValue;
}

impl ToAppSqliteValue for str {
    fn to_app_sqlite_value(&self) -> AppSqliteValue {
        AppSqliteValue::Text(self.to_owned())
    }
}

impl ToAppSqliteValue for &str {
    fn to_app_sqlite_value(&self) -> AppSqliteValue {
        AppSqliteValue::Text((*self).to_owned())
    }
}

impl ToAppSqliteValue for String {
    fn to_app_sqlite_value(&self) -> AppSqliteValue {
        AppSqliteValue::Text(self.clone())
    }
}

impl ToAppSqliteValue for &String {
    fn to_app_sqlite_value(&self) -> AppSqliteValue {
        AppSqliteValue::Text((*self).clone())
    }
}

impl ToAppSqliteValue for bool {
    fn to_app_sqlite_value(&self) -> AppSqliteValue {
        AppSqliteValue::Integer(i64::from(*self))
    }
}

impl ToAppSqliteValue for i64 {
    fn to_app_sqlite_value(&self) -> AppSqliteValue {
        AppSqliteValue::Integer(*self)
    }
}

impl ToAppSqliteValue for &i64 {
    fn to_app_sqlite_value(&self) -> AppSqliteValue {
        AppSqliteValue::Integer(**self)
    }
}

impl ToAppSqliteValue for i32 {
    fn to_app_sqlite_value(&self) -> AppSqliteValue {
        AppSqliteValue::Integer(i64::from(*self))
    }
}

impl ToAppSqliteValue for u32 {
    fn to_app_sqlite_value(&self) -> AppSqliteValue {
        AppSqliteValue::Integer(i64::from(*self))
    }
}

impl ToAppSqliteValue for usize {
    fn to_app_sqlite_value(&self) -> AppSqliteValue {
        AppSqliteValue::Integer(i64::try_from(*self).unwrap_or(i64::MAX))
    }
}

impl ToAppSqliteValue for f64 {
    fn to_app_sqlite_value(&self) -> AppSqliteValue {
        AppSqliteValue::Real(*self)
    }
}

impl<T> ToAppSqliteValue for Option<T>
where
    T: ToAppSqliteValue,
{
    fn to_app_sqlite_value(&self) -> AppSqliteValue {
        self.as_ref()
            .map(ToAppSqliteValue::to_app_sqlite_value)
            .unwrap_or(AppSqliteValue::Null)
    }
}

impl IntoAppSqliteParams for AppSqliteParams {
    fn into_app_sqlite_params(self) -> AppSqliteParams {
        self
    }
}

impl IntoAppSqliteParams for [(); 0] {
    fn into_app_sqlite_params(self) -> AppSqliteParams {
        Vec::new()
    }
}

impl<T, const N: usize> IntoAppSqliteParams for [T; N]
where
    T: ToAppSqliteValue,
{
    fn into_app_sqlite_params(self) -> AppSqliteParams {
        self.into_iter()
            .map(|value| value.to_app_sqlite_value())
            .collect()
    }
}

impl<T> IntoAppSqliteParams for Vec<T>
where
    T: ToAppSqliteValue,
{
    fn into_app_sqlite_params(self) -> AppSqliteParams {
        self.into_iter()
            .map(|value| value.to_app_sqlite_value())
            .collect()
    }
}

impl<T> OptionalSqliteResult<T> for Result<T, sqlx::Error> {
    fn optional(self) -> Result<Option<T>, sqlx::Error> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

pub(crate) fn app_sqlite_value<T>(value: &T) -> AppSqliteValue
where
    T: ToAppSqliteValue + ?Sized,
{
    value.to_app_sqlite_value()
}

#[derive(Clone)]
pub(crate) struct AppSqliteDatabase {
    connection: Arc<Mutex<SqliteConnection>>,
}

impl AppSqliteDatabase {
    pub(crate) fn open_path(path: &Path) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);
        Self::connect(options)
    }

    pub(crate) fn open_in_memory() -> Result<Self, sqlx::Error> {
        Self::connect(SqliteConnectOptions::new().in_memory(true))
    }

    pub(crate) fn execute_statement(
        &self,
        sql: &str,
        params: AppSqliteParams,
    ) -> Result<u64, sqlx::Error> {
        let mut connection = self.lock()?;
        let query = bind_params(sqlx::query(sqlx::AssertSqlSafe(sql)), params);
        let result = futures_executor::block_on(query.execute(&mut *connection))?;
        Ok(result.rows_affected())
    }

    pub(crate) fn execute<P>(&self, sql: &str, params: P) -> Result<u64, sqlx::Error>
    where
        P: IntoAppSqliteParams,
    {
        self.execute_statement(sql, params.into_app_sqlite_params())
    }

    pub(crate) fn execute_script(&self, sql: &str) -> Result<(), sqlx::Error> {
        let mut connection = self.lock()?;
        futures_executor::block_on(
            sqlx::raw_sql(sqlx::AssertSqlSafe(sql)).execute(&mut *connection),
        )?;
        Ok(())
    }

    pub(crate) fn execute_batch(&self, sql: &str) -> Result<(), sqlx::Error> {
        self.execute_script(sql)
    }

    pub(crate) fn fetch_one<T, F>(
        &self,
        sql: &str,
        params: AppSqliteParams,
        map: F,
    ) -> Result<T, sqlx::Error>
    where
        F: FnOnce(&SqliteRow) -> Result<T, sqlx::Error>,
    {
        let mut connection = self.lock()?;
        let query = bind_params(sqlx::query(sqlx::AssertSqlSafe(sql)), params);
        let row = futures_executor::block_on(query.fetch_one(&mut *connection))?;
        map(&row)
    }

    pub(crate) fn query_row<T, P, F>(&self, sql: &str, params: P, map: F) -> Result<T, sqlx::Error>
    where
        P: IntoAppSqliteParams,
        F: FnOnce(&SqliteRow) -> Result<T, sqlx::Error>,
    {
        self.fetch_one(sql, params.into_app_sqlite_params(), map)
    }

    pub(crate) fn fetch_optional<T, F>(
        &self,
        sql: &str,
        params: AppSqliteParams,
        map: F,
    ) -> Result<Option<T>, sqlx::Error>
    where
        F: FnOnce(&SqliteRow) -> Result<T, sqlx::Error>,
    {
        let mut connection = self.lock()?;
        let query = bind_params(sqlx::query(sqlx::AssertSqlSafe(sql)), params);
        let Some(row) = futures_executor::block_on(query.fetch_optional(&mut *connection))? else {
            return Ok(None);
        };
        map(&row).map(Some)
    }

    pub(crate) fn fetch_mapped<T, F>(
        &self,
        sql: &str,
        params: AppSqliteParams,
        map: F,
    ) -> Result<std::vec::IntoIter<Result<T, sqlx::Error>>, sqlx::Error>
    where
        F: FnMut(&SqliteRow) -> Result<T, sqlx::Error>,
    {
        let rows = {
            let mut connection = self.lock()?;
            let query = bind_params(sqlx::query(sqlx::AssertSqlSafe(sql)), params);
            futures_executor::block_on(query.fetch_all(&mut *connection))?
        };
        Ok(rows.iter().map(map).collect::<Vec<_>>().into_iter())
    }

    pub(crate) fn prepare(&self, sql: &str) -> Result<AppSqliteStatement<'_>, sqlx::Error> {
        Ok(AppSqliteStatement {
            database: self,
            sql: sql.to_owned(),
        })
    }

    #[cfg(test)]
    pub(crate) fn query_rows<P>(&self, sql: &str, params: P) -> Result<AppSqliteRows, sqlx::Error>
    where
        P: IntoAppSqliteParams,
    {
        let rows = {
            let mut connection = self.lock()?;
            let query = bind_params(
                sqlx::query(sqlx::AssertSqlSafe(sql)),
                params.into_app_sqlite_params(),
            );
            futures_executor::block_on(query.fetch_all(&mut *connection))?
        };
        Ok(AppSqliteRows {
            rows: rows.into_iter(),
        })
    }

    fn connect(options: SqliteConnectOptions) -> Result<Self, sqlx::Error> {
        let connection = futures_executor::block_on(SqliteConnection::connect_with(&options))?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, SqliteConnection>, sqlx::Error> {
        self.connection
            .lock()
            .map_err(|_| sqlx::Error::Protocol("sqlite connection mutex poisoned".to_owned()))
    }
}

pub(crate) struct AppSqliteStatement<'a> {
    database: &'a AppSqliteDatabase,
    sql: String,
}

impl AppSqliteStatement<'_> {
    pub(crate) fn execute<P>(&mut self, params: P) -> Result<u64, sqlx::Error>
    where
        P: IntoAppSqliteParams,
    {
        self.database.execute(&self.sql, params)
    }

    pub(crate) fn query_map<T, P, F>(
        &mut self,
        params: P,
        map: F,
    ) -> Result<std::vec::IntoIter<Result<T, sqlx::Error>>, sqlx::Error>
    where
        P: IntoAppSqliteParams,
        F: FnMut(&SqliteRow) -> Result<T, sqlx::Error>,
    {
        self.database
            .fetch_mapped(&self.sql, params.into_app_sqlite_params(), map)
    }

    #[cfg(test)]
    pub(crate) fn query<P>(&mut self, params: P) -> Result<AppSqliteRows, sqlx::Error>
    where
        P: IntoAppSqliteParams,
    {
        self.database.query_rows(&self.sql, params)
    }
}

#[cfg(test)]
pub(crate) struct AppSqliteRows {
    rows: std::vec::IntoIter<SqliteRow>,
}

#[cfg(test)]
impl AppSqliteRows {
    pub(crate) fn next(&mut self) -> Result<Option<SqliteRow>, sqlx::Error> {
        Ok(self.rows.next())
    }
}

pub(crate) fn empty_params() -> AppSqliteParams {
    Vec::new()
}

fn bind_params<'q>(
    query: sqlx::query::Query<'q, sqlx::Sqlite, SqliteArguments>,
    params: AppSqliteParams,
) -> sqlx::query::Query<'q, sqlx::Sqlite, SqliteArguments> {
    let mut query = query;
    for param in params {
        query = match param {
            AppSqliteValue::Null => query.bind(Option::<String>::None),
            AppSqliteValue::Integer(value) => query.bind(value),
            AppSqliteValue::Real(value) => query.bind(value),
            AppSqliteValue::Text(value) => query.bind(value),
        };
    }
    query
}
