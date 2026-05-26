use std::collections::BTreeSet;

use radroots_studio_app_models::{
    FarmOrderMethod, FarmReadiness, FarmSetupDraft, FarmSetupProjection, FarmSummary,
};
use rusqlite::{Connection, OptionalExtension, params};

use crate::AppSqliteError;

pub struct AppFarmSetupRepository<'a> {
    connection: &'a Connection,
}

impl<'a> AppFarmSetupRepository<'a> {
    pub const fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub fn load_farm_setup(&self, account_id: &str) -> Result<FarmSetupProjection, AppSqliteError> {
        let row = self
            .connection
            .query_row(
                "SELECT
                    farm_name,
                    location_or_service_area,
                    pickup_enabled,
                    delivery_enabled,
                    shipping_enabled,
                    saved_farm_id,
                    saved_farm_display_name,
                    saved_farm_readiness
                 FROM account_farm_setups
                 WHERE account_id = ?1
                 LIMIT 1",
                [account_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load account farm setup",
                source,
            })?;

        let Some((
            farm_name,
            location_or_service_area,
            pickup_enabled,
            delivery_enabled,
            shipping_enabled,
            saved_farm_id,
            saved_farm_display_name,
            saved_farm_readiness,
        )) = row
        else {
            return Ok(FarmSetupProjection::not_started());
        };

        let mut order_methods = BTreeSet::new();
        if parse_sqlite_bool("account_farm_setups.pickup_enabled", pickup_enabled)? {
            order_methods.insert(FarmOrderMethod::Pickup);
        }
        if parse_sqlite_bool("account_farm_setups.delivery_enabled", delivery_enabled)? {
            order_methods.insert(FarmOrderMethod::Delivery);
        }
        if parse_sqlite_bool("account_farm_setups.shipping_enabled", shipping_enabled)? {
            order_methods.insert(FarmOrderMethod::Shipping);
        }

        let saved_farm =
            parse_saved_farm(saved_farm_id, saved_farm_display_name, saved_farm_readiness)?;

        Ok(FarmSetupProjection::new(
            FarmSetupDraft::new(farm_name, location_or_service_area, order_methods),
            saved_farm,
        ))
    }

    pub fn save_farm_setup(
        &self,
        account_id: &str,
        projection: &FarmSetupProjection,
    ) -> Result<(), AppSqliteError> {
        if !projection.has_saved_farm() && projection.draft.is_empty() {
            return self.clear_farm_setup(account_id);
        }

        self.connection
            .execute(
                "INSERT INTO account_farm_setups (
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
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                ON CONFLICT(account_id) DO UPDATE SET
                    farm_name = excluded.farm_name,
                    location_or_service_area = excluded.location_or_service_area,
                    pickup_enabled = excluded.pickup_enabled,
                    delivery_enabled = excluded.delivery_enabled,
                    shipping_enabled = excluded.shipping_enabled,
                    saved_farm_id = excluded.saved_farm_id,
                    saved_farm_display_name = excluded.saved_farm_display_name,
                    saved_farm_readiness = excluded.saved_farm_readiness,
                    updated_at = excluded.updated_at",
                params![
                    account_id,
                    projection.draft.farm_name,
                    projection.draft.location_or_service_area,
                    i64::from(
                        projection
                            .draft
                            .order_methods
                            .contains(&FarmOrderMethod::Pickup)
                    ),
                    i64::from(
                        projection
                            .draft
                            .order_methods
                            .contains(&FarmOrderMethod::Delivery)
                    ),
                    i64::from(
                        projection
                            .draft
                            .order_methods
                            .contains(&FarmOrderMethod::Shipping)
                    ),
                    projection
                        .saved_farm
                        .as_ref()
                        .map(|farm| farm.farm_id.to_string()),
                    projection
                        .saved_farm
                        .as_ref()
                        .map(|farm| farm.display_name.clone()),
                    projection
                        .saved_farm
                        .as_ref()
                        .map(|farm| farm_readiness_storage_key(farm.readiness)),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "save account farm setup",
                source,
            })?;

        Ok(())
    }

    pub fn clear_farm_setup(&self, account_id: &str) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "DELETE FROM account_farm_setups WHERE account_id = ?1",
                [account_id],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "clear account farm setup",
                source,
            })?;

        Ok(())
    }
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

fn parse_saved_farm(
    farm_id: Option<String>,
    display_name: Option<String>,
    readiness: Option<String>,
) -> Result<Option<FarmSummary>, AppSqliteError> {
    match (farm_id, display_name, readiness) {
        (Some(farm_id), Some(display_name), Some(readiness)) => Ok(Some(FarmSummary {
            farm_id: farm_id.parse().map_err(|_| AppSqliteError::DecodeId {
                field: "account_farm_setups.saved_farm_id",
                value: farm_id,
            })?,
            display_name,
            readiness: parse_farm_readiness("account_farm_setups.saved_farm_readiness", readiness)?,
        })),
        (None, None, None) => Ok(None),
        (Some(_), None, _) => Err(AppSqliteError::MissingColumn {
            field: "account_farm_setups.saved_farm_display_name",
        }),
        (Some(_), _, None) => Err(AppSqliteError::MissingColumn {
            field: "account_farm_setups.saved_farm_readiness",
        }),
        (None, Some(_), _) => Err(AppSqliteError::MissingColumn {
            field: "account_farm_setups.saved_farm_id",
        }),
        (None, _, Some(_)) => Err(AppSqliteError::MissingColumn {
            field: "account_farm_setups.saved_farm_id",
        }),
    }
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
    use std::{
        env, fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use radroots_studio_app_models::{
        FarmId, FarmOrderMethod, FarmReadiness, FarmSetupDraft, FarmSetupProjection, FarmSummary,
    };

    use crate::{AppSqliteStore, DatabaseTarget};

    #[test]
    fn load_farm_setup_returns_not_started_when_account_is_missing() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");

        let projection = store
            .load_farm_setup("acct_missing")
            .expect("missing setup should load");

        assert_eq!(projection, FarmSetupProjection::not_started());
    }

    #[test]
    fn farm_setup_round_trips_incomplete_draft() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let projection = FarmSetupProjection::from_draft(FarmSetupDraft::new(
            "North field farm",
            "",
            [FarmOrderMethod::Pickup, FarmOrderMethod::Shipping],
        ));

        store
            .save_farm_setup("acct_farm_draft", &projection)
            .expect("farm setup should save");

        let loaded = store
            .load_farm_setup("acct_farm_draft")
            .expect("farm setup should load");

        assert_eq!(loaded, projection);
    }

    #[test]
    fn farm_setup_round_trips_saved_farm_state() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let saved_farm = FarmSummary {
            farm_id: FarmId::new(),
            display_name: "North field farm".to_owned(),
            readiness: FarmReadiness::Ready,
        };
        let projection = FarmSetupProjection::new(
            FarmSetupDraft::new(
                "North field farm",
                "Asheville, NC",
                [FarmOrderMethod::Pickup],
            ),
            Some(saved_farm.clone()),
        );

        store
            .save_farm_setup("acct_saved_farm", &projection)
            .expect("saved farm setup should save");

        let loaded = store
            .load_farm_setup("acct_saved_farm")
            .expect("saved farm setup should load");

        assert_eq!(loaded.saved_farm, Some(saved_farm));
        assert_eq!(loaded.readiness, projection.readiness);
        assert_eq!(loaded.draft, projection.draft);
    }

    #[test]
    fn clearing_farm_setup_restores_not_started_state() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");

        store
            .save_farm_setup(
                "acct_clear",
                &FarmSetupProjection::from_draft(FarmSetupDraft::new(
                    "North field farm",
                    "Asheville, NC",
                    [FarmOrderMethod::Delivery],
                )),
            )
            .expect("farm setup should save");
        store
            .clear_farm_setup("acct_clear")
            .expect("farm setup should clear");

        assert_eq!(
            store
                .load_farm_setup("acct_clear")
                .expect("cleared setup should load"),
            FarmSetupProjection::not_started()
        );
    }

    #[test]
    fn file_backed_farm_setup_survives_reopen() {
        let path = temp_database_path("farm_setup_reopen");
        let projection = FarmSetupProjection::from_draft(FarmSetupDraft::new(
            "North field farm",
            "Asheville, NC",
            [FarmOrderMethod::Pickup, FarmOrderMethod::Delivery],
        ));

        let first = AppSqliteStore::open(DatabaseTarget::Path(path.clone())).expect("store");
        first
            .save_farm_setup("acct_file_backed", &projection)
            .expect("farm setup should save");
        drop(first);

        let reopened = AppSqliteStore::open(DatabaseTarget::Path(path.clone())).expect("reopen");
        let loaded = reopened
            .load_farm_setup("acct_file_backed")
            .expect("reloaded setup should load");

        assert_eq!(loaded, projection);

        drop(reopened);
        remove_database_artifacts(&path);
    }

    fn temp_database_path(test_name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();

        env::temp_dir()
            .join("radroots_studio_app_sqlite_tests")
            .join(format!("{test_name}-{nonce}"))
            .join("app.sqlite3")
    }

    fn remove_database_artifacts(database_path: &std::path::Path) {
        if let Some(parent) = database_path.parent() {
            let wal_path = database_path.with_extension("sqlite3-wal");
            let shm_path = database_path.with_extension("sqlite3-shm");

            let _ = fs::remove_file(&wal_path);
            let _ = fs::remove_file(&shm_path);
            let _ = fs::remove_file(database_path);
            let _ = fs::remove_dir_all(parent);
        }
    }
}
