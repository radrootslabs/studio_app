use super::{
    farm::{farm_query_bind_values, FarmQueryBindValues},
    location_gcs::{location_gcs_query_bind_values, LocationGcsQueryBindValues},
};
use crate::{error::ModelError, types::DatabaseConnection};
use tangle_core::types::IResultPass;

#[derive(serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct FarmLocation {
    tb_farm: String,
    tb_lg: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct IFarmLocationTables {
    pub farm: FarmQueryBindValues,
    pub location_gcs: LocationGcsQueryBindValues,
}

pub type IFarmLocationRelation = IFarmLocationTables;
pub type IFarmLocationResolve = IResultPass;

pub async fn lib_model_farm_location_set(
    db: &DatabaseConnection,
    opts: IFarmLocationRelation,
) -> Result<IFarmLocationResolve, ModelError> {
    let (bv_farm_k, bv_farm) = farm_query_bind_values(opts.farm);
    let (bv_lg_k, bv_lg) = location_gcs_query_bind_values(opts.location_gcs);
    let query_vals = vec![bv_farm, bv_lg];
    let query = format!("INSERT INTO farm_location (tb_farm, tb_lg) VALUES ((SELECT id FROM farm WHERE {} = ?), (SELECT id FROM location_gcs WHERE {} = ?));", bv_farm_k, bv_lg_k);
    let mut query_builder = sqlx::query(&query);
    for value in query_vals.iter() {
        query_builder = query_builder.bind(value);
    }
    query_builder
        .execute(db)
        .await
        .map_err(|e| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IResultPass { pass: true })
}
pub async fn lib_model_farm_location_unset(
    db: &DatabaseConnection,
    opts: IFarmLocationRelation,
) -> Result<IFarmLocationResolve, ModelError> {
    let (bv_farm_k, bv_farm) = farm_query_bind_values(opts.farm);
    let (bv_lg_k, bv_lg) = location_gcs_query_bind_values(opts.location_gcs);
    let query_vals = vec![bv_farm, bv_lg];
    let query = format!("DELETE FROM farm_location WHERE tb_farm = (SELECT id FROM farm WHERE {} = ?) AND tb_lg = (SELECT id FROM location_gcs WHERE {} = ?);", bv_farm_k, bv_lg_k);
    let mut query_builder = sqlx::query(&query);
    for value in query_vals.iter() {
        query_builder = query_builder.bind(value);
    }
    query_builder
        .execute(db)
        .await
        .map_err(|e| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IResultPass { pass: true })
}
