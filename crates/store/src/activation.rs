use radroots_studio_app_view::{
    AccountSurfaceActivationProjection, ActiveSurface, FarmId, FarmerActivationProjection,
    SelectedSurfaceProjection,
};
use rusqlite::{Connection, OptionalExtension, params};

use crate::AppSqliteError;

pub struct AppActivationRepository<'a> {
    connection: &'a Connection,
}

impl<'a> AppActivationRepository<'a> {
    pub const fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub fn load_surface_activation(
        &self,
        account_id: &str,
    ) -> Result<Option<AccountSurfaceActivationProjection>, AppSqliteError> {
        let row = self
            .connection
            .query_row(
                "SELECT account_id, selected_surface, farmer_farm_id
                 FROM account_surface_activations
                 WHERE account_id = ?1
                 LIMIT 1",
                [account_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load account surface activation",
                source,
            })?;

        row.map(|(account_id, selected_surface, farmer_farm_id)| {
            Ok(AccountSurfaceActivationProjection::new(
                account_id,
                SelectedSurfaceProjection::new(parse_active_surface(
                    "account_surface_activations.selected_surface",
                    selected_surface,
                )?),
                FarmerActivationProjection {
                    farm_id: parse_optional_farm_id(
                        "account_surface_activations.farmer_farm_id",
                        farmer_farm_id,
                    )?,
                },
            ))
        })
        .transpose()
    }

    pub fn save_surface_activation(
        &self,
        projection: &AccountSurfaceActivationProjection,
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "INSERT INTO account_surface_activations (
                    account_id,
                    selected_surface,
                    farmer_farm_id,
                    updated_at
                ) VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                ON CONFLICT(account_id) DO UPDATE SET
                    selected_surface = excluded.selected_surface,
                    farmer_farm_id = excluded.farmer_farm_id,
                    updated_at = excluded.updated_at",
                params![
                    projection.account_id,
                    projection.active_surface().storage_key(),
                    projection
                        .farmer_activation
                        .farm_id
                        .map(|farm_id| farm_id.to_string()),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "save account surface activation",
                source,
            })?;

        Ok(())
    }

    pub fn clear_surface_activation(&self, account_id: &str) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "DELETE FROM account_surface_activations WHERE account_id = ?1",
                [account_id],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "clear account surface activation",
                source,
            })?;

        Ok(())
    }
}

fn parse_active_surface(
    field: &'static str,
    value: String,
) -> Result<ActiveSurface, AppSqliteError> {
    match value.as_str() {
        "personal" => Ok(ActiveSurface::Personal),
        "farmer" => Ok(ActiveSurface::Farmer),
        _ => Err(AppSqliteError::DecodeEnum { field, value }),
    }
}

fn parse_optional_farm_id(
    field: &'static str,
    value: Option<String>,
) -> Result<Option<FarmId>, AppSqliteError> {
    value
        .map(|value| {
            value
                .parse()
                .map_err(|_| AppSqliteError::DecodeId { field, value })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use radroots_studio_app_view::{
        AccountSurfaceActivationProjection, ActiveSurface, FarmId, FarmerActivationProjection,
        SelectedSurfaceProjection,
    };

    use crate::{AppSqliteStore, DatabaseTarget};

    #[test]
    fn load_surface_activation_returns_none_for_unknown_account() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");

        let projection = store
            .load_surface_activation("acct_missing")
            .expect("missing activation should load");

        assert_eq!(projection, None);
    }

    #[test]
    fn surface_activation_round_trips_farmer_binding() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let projection = AccountSurfaceActivationProjection::new(
            "acct_farmer",
            SelectedSurfaceProjection::new(ActiveSurface::Farmer),
            FarmerActivationProjection::active(FarmId::new()),
        );

        store
            .save_surface_activation(&projection)
            .expect("surface activation should save");

        let loaded = store
            .load_surface_activation("acct_farmer")
            .expect("surface activation should load")
            .expect("surface activation should exist");

        assert_eq!(loaded, projection);
    }

    #[test]
    fn surface_activation_upsert_and_clear_are_explicit() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let first = AccountSurfaceActivationProjection::new(
            "acct_surface",
            SelectedSurfaceProjection::new(ActiveSurface::Farmer),
            FarmerActivationProjection::active(FarmId::new()),
        );
        let second = AccountSurfaceActivationProjection::new(
            "acct_surface",
            SelectedSurfaceProjection::new(ActiveSurface::Farmer),
            FarmerActivationProjection::inactive(),
        );

        store
            .save_surface_activation(&first)
            .expect("initial surface activation should save");
        store
            .save_surface_activation(&second)
            .expect("updated surface activation should save");

        let loaded = store
            .load_surface_activation("acct_surface")
            .expect("updated surface activation should load")
            .expect("updated surface activation should exist");
        assert_eq!(loaded.active_surface(), ActiveSurface::Personal);
        assert_eq!(loaded, second);

        store
            .clear_surface_activation("acct_surface")
            .expect("surface activation should clear");
        assert_eq!(
            store
                .load_surface_activation("acct_surface")
                .expect("cleared surface activation should load"),
            None
        );
    }
}
