use std::{fmt, str::FromStr};

use radroots_studio_app_models::{
    BlackoutPeriodRecord, FarmId, FarmOperatingRulesRecord, FarmProfileRecord,
    FarmReadinessBlocker, FarmRulesProjection, FarmRulesReadiness, FarmTimingConflict,
    FarmTimingConflictKind, FulfillmentWindowRecord, PickupLocationRecord,
};
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};

use crate::AppSqliteError;

pub struct AppFarmRulesRepository<'a> {
    connection: &'a Connection,
}

impl<'a> AppFarmRulesRepository<'a> {
    pub const fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub fn load_farm_rules(&self, farm_id: FarmId) -> Result<FarmRulesProjection, AppSqliteError> {
        let farm_profile = self.load_farm_profile(farm_id)?;

        if farm_profile.is_none() {
            return Ok(FarmRulesProjection::default());
        }

        let pickup_locations = self.load_pickup_locations(farm_id)?;
        let operating_rules = self.load_operating_rules(farm_id)?;
        let fulfillment_windows = self.load_fulfillment_windows(farm_id)?;
        let blackout_periods = self.load_blackout_periods(farm_id)?;
        let readiness = derive_farm_rules_readiness_parts(
            farm_profile.as_ref(),
            &pickup_locations,
            operating_rules.as_ref(),
            &fulfillment_windows,
            &blackout_periods,
        );

        Ok(FarmRulesProjection {
            farm_profile,
            pickup_locations,
            operating_rules,
            fulfillment_windows,
            blackout_periods,
            readiness,
        })
    }

    pub fn save_farm_rules(&self, projection: &FarmRulesProjection) -> Result<(), AppSqliteError> {
        let farm_id = validate_projection(projection)?;
        let readiness = derive_farm_rules_readiness(projection);
        let farm_profile =
            projection
                .farm_profile
                .as_ref()
                .ok_or(AppSqliteError::InvalidProjection {
                    reason: "farm rules projection must include a farm profile",
                })?;

        self.connection
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|source| AppSqliteError::Query {
                operation: "begin save farm rules transaction",
                source,
            })?;

        let result = (|| {
            self.upsert_farm_profile(farm_profile, readiness.is_ready())?;

            match projection.operating_rules.as_ref() {
                Some(rules) => self.upsert_operating_rules(rules)?,
                None => self.delete_operating_rules(farm_id)?,
            }

            for pickup_location in &projection.pickup_locations {
                self.upsert_pickup_location(pickup_location)?;
            }

            for fulfillment_window in &projection.fulfillment_windows {
                self.upsert_fulfillment_window(fulfillment_window)?;
            }

            for blackout_period in &projection.blackout_periods {
                self.upsert_blackout_period(blackout_period)?;
            }

            self.delete_missing_blackout_periods(farm_id, &projection.blackout_periods)?;
            self.delete_missing_fulfillment_windows(farm_id, &projection.fulfillment_windows)?;
            self.delete_missing_pickup_locations(farm_id, &projection.pickup_locations)?;

            Ok(())
        })();

        match result {
            Ok(()) => {
                self.connection.execute_batch("COMMIT").map_err(|source| {
                    AppSqliteError::Query {
                        operation: "commit save farm rules transaction",
                        source,
                    }
                })?;
                Ok(())
            }
            Err(error) => {
                let _ = self.connection.execute_batch("ROLLBACK");
                Err(error)
            }
        }
    }

    fn load_farm_profile(
        &self,
        farm_id: FarmId,
    ) -> Result<Option<FarmProfileRecord>, AppSqliteError> {
        let row = self
            .connection
            .query_row(
                "select id, display_name, timezone, currency_code
                 from farms
                 where id = ?1
                 limit 1",
                [farm_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load farm rules profile",
                source,
            })?;

        row.map(|(farm_id, display_name, timezone, currency_code)| {
            Ok(FarmProfileRecord {
                farm_id: parse_typed_id("farms.id", farm_id)?,
                display_name,
                timezone,
                currency_code,
            })
        })
        .transpose()
    }

    fn load_pickup_locations(
        &self,
        farm_id: FarmId,
    ) -> Result<Vec<PickupLocationRecord>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select id, farm_id, label, address_line, directions, is_default
                 from pickup_locations
                 where farm_id = ?1
                 order by is_default desc, updated_at desc, id desc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare load pickup locations",
                source,
            })?;
        let rows = statement
            .query_map([farm_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query load pickup locations",
                source,
            })?;
        let rows = collect_rows("read pickup locations", rows)?;
        let mut pickup_locations = Vec::with_capacity(rows.len());

        for (pickup_location_id, farm_id, label, address_line, directions, is_default) in rows {
            pickup_locations.push(PickupLocationRecord {
                pickup_location_id: parse_typed_id("pickup_locations.id", pickup_location_id)?,
                farm_id: parse_typed_id("pickup_locations.farm_id", farm_id)?,
                label,
                address_line,
                directions,
                is_default: parse_sqlite_bool("pickup_locations.is_default", is_default)?,
            });
        }

        Ok(pickup_locations)
    }

    fn load_operating_rules(
        &self,
        farm_id: FarmId,
    ) -> Result<Option<FarmOperatingRulesRecord>, AppSqliteError> {
        let row = self
            .connection
            .query_row(
                "select farm_id, promise_lead_hours, substitution_policy, missed_pickup_policy
                 from farm_operating_rules
                 where farm_id = ?1
                 limit 1",
                [farm_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load farm operating rules",
                source,
            })?;

        row.map(
            |(farm_id, promise_lead_hours, substitution_policy, missed_pickup_policy)| {
                Ok(FarmOperatingRulesRecord {
                    farm_id: parse_typed_id("farm_operating_rules.farm_id", farm_id)?,
                    promise_lead_hours: parse_u16(
                        "farm_operating_rules.promise_lead_hours",
                        promise_lead_hours,
                    )?,
                    substitution_policy,
                    missed_pickup_policy,
                })
            },
        )
        .transpose()
    }

    fn load_fulfillment_windows(
        &self,
        farm_id: FarmId,
    ) -> Result<Vec<FulfillmentWindowRecord>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select
                    fw.id,
                    fw.farm_id,
                    fw.pickup_location_id,
                    fw.label,
                    fw.starts_at,
                    fw.ends_at,
                    fw.order_cutoff_at
                 from fulfillment_windows fw
                 inner join pickup_locations pl
                    on pl.id = fw.pickup_location_id and pl.farm_id = fw.farm_id
                 where fw.farm_id = ?1
                   and trim(fw.label) <> ''
                   and fw.order_cutoff_at is not null
                   and trim(fw.order_cutoff_at) <> ''
                 order by fw.starts_at asc, fw.id asc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare load fulfillment windows",
                source,
            })?;
        let rows = statement
            .query_map([farm_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query load fulfillment windows",
                source,
            })?;
        let rows = collect_rows("read fulfillment windows", rows)?;
        let mut fulfillment_windows = Vec::with_capacity(rows.len());

        for (
            fulfillment_window_id,
            farm_id,
            pickup_location_id,
            label,
            starts_at,
            ends_at,
            order_cutoff_at,
        ) in rows
        {
            fulfillment_windows.push(FulfillmentWindowRecord {
                fulfillment_window_id: parse_typed_id(
                    "fulfillment_windows.id",
                    fulfillment_window_id,
                )?,
                farm_id: parse_typed_id("fulfillment_windows.farm_id", farm_id)?,
                pickup_location_id: parse_typed_id(
                    "fulfillment_windows.pickup_location_id",
                    pickup_location_id,
                )?,
                label,
                starts_at,
                ends_at,
                order_cutoff_at,
            });
        }

        Ok(fulfillment_windows)
    }

    fn load_blackout_periods(
        &self,
        farm_id: FarmId,
    ) -> Result<Vec<BlackoutPeriodRecord>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "select id, farm_id, label, starts_at, ends_at
                 from blackout_periods
                 where farm_id = ?1
                 order by starts_at asc, id asc",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare load blackout periods",
                source,
            })?;
        let rows = statement
            .query_map([farm_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query load blackout periods",
                source,
            })?;
        let rows = collect_rows("read blackout periods", rows)?;
        let mut blackout_periods = Vec::with_capacity(rows.len());

        for (blackout_period_id, farm_id, label, starts_at, ends_at) in rows {
            blackout_periods.push(BlackoutPeriodRecord {
                blackout_period_id: parse_typed_id("blackout_periods.id", blackout_period_id)?,
                farm_id: parse_typed_id("blackout_periods.farm_id", farm_id)?,
                label,
                starts_at,
                ends_at,
            });
        }

        Ok(blackout_periods)
    }

    fn upsert_farm_profile(
        &self,
        farm_profile: &FarmProfileRecord,
        ready: bool,
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "insert into farms (
                    id,
                    display_name,
                    readiness,
                    timezone,
                    currency_code,
                    created_at,
                    updated_at
                 ) values (
                    ?1,
                    ?2,
                    ?3,
                    ?4,
                    ?5,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 )
                 on conflict(id) do update set
                    display_name = excluded.display_name,
                    readiness = excluded.readiness,
                    timezone = excluded.timezone,
                    currency_code = excluded.currency_code,
                    updated_at = excluded.updated_at",
                params![
                    farm_profile.farm_id.to_string(),
                    farm_profile.display_name,
                    farm_readiness_storage_key(ready),
                    farm_profile.timezone,
                    farm_profile.currency_code,
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "save farm profile",
                source,
            })?;

        Ok(())
    }

    fn upsert_operating_rules(
        &self,
        operating_rules: &FarmOperatingRulesRecord,
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "insert into farm_operating_rules (
                    farm_id,
                    promise_lead_hours,
                    substitution_policy,
                    missed_pickup_policy,
                    created_at,
                    updated_at
                 ) values (
                    ?1,
                    ?2,
                    ?3,
                    ?4,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 )
                 on conflict(farm_id) do update set
                    promise_lead_hours = excluded.promise_lead_hours,
                    substitution_policy = excluded.substitution_policy,
                    missed_pickup_policy = excluded.missed_pickup_policy,
                    updated_at = excluded.updated_at",
                params![
                    operating_rules.farm_id.to_string(),
                    i64::from(operating_rules.promise_lead_hours),
                    operating_rules.substitution_policy,
                    operating_rules.missed_pickup_policy,
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "save farm operating rules",
                source,
            })?;

        Ok(())
    }

    fn delete_operating_rules(&self, farm_id: FarmId) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "delete from farm_operating_rules where farm_id = ?1",
                [farm_id.to_string()],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "delete farm operating rules",
                source,
            })?;

        Ok(())
    }

    fn upsert_pickup_location(
        &self,
        pickup_location: &PickupLocationRecord,
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "insert into pickup_locations (
                    id,
                    farm_id,
                    label,
                    address_line,
                    directions,
                    is_default,
                    created_at,
                    updated_at
                 ) values (
                    ?1,
                    ?2,
                    ?3,
                    ?4,
                    ?5,
                    ?6,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 )
                 on conflict(id) do update set
                    farm_id = excluded.farm_id,
                    label = excluded.label,
                    address_line = excluded.address_line,
                    directions = excluded.directions,
                    is_default = excluded.is_default,
                    updated_at = excluded.updated_at",
                params![
                    pickup_location.pickup_location_id.to_string(),
                    pickup_location.farm_id.to_string(),
                    pickup_location.label,
                    pickup_location.address_line,
                    pickup_location.directions,
                    i64::from(pickup_location.is_default),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "save pickup location",
                source,
            })?;

        Ok(())
    }

    fn upsert_fulfillment_window(
        &self,
        fulfillment_window: &FulfillmentWindowRecord,
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "insert into fulfillment_windows (
                    id,
                    farm_id,
                    starts_at,
                    ends_at,
                    capacity_limit,
                    created_at,
                    updated_at,
                    pickup_location_id,
                    label,
                    order_cutoff_at
                 ) values (
                    ?1,
                    ?2,
                    ?3,
                    ?4,
                    null,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    ?5,
                    ?6,
                    ?7
                 )
                 on conflict(id) do update set
                    farm_id = excluded.farm_id,
                    starts_at = excluded.starts_at,
                    ends_at = excluded.ends_at,
                    pickup_location_id = excluded.pickup_location_id,
                    label = excluded.label,
                    order_cutoff_at = excluded.order_cutoff_at,
                    updated_at = excluded.updated_at",
                params![
                    fulfillment_window.fulfillment_window_id.to_string(),
                    fulfillment_window.farm_id.to_string(),
                    fulfillment_window.starts_at,
                    fulfillment_window.ends_at,
                    fulfillment_window.pickup_location_id.to_string(),
                    fulfillment_window.label,
                    fulfillment_window.order_cutoff_at,
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "save fulfillment window",
                source,
            })?;

        Ok(())
    }

    fn upsert_blackout_period(
        &self,
        blackout_period: &BlackoutPeriodRecord,
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "insert into blackout_periods (
                    id,
                    farm_id,
                    label,
                    starts_at,
                    ends_at,
                    created_at,
                    updated_at
                 ) values (
                    ?1,
                    ?2,
                    ?3,
                    ?4,
                    ?5,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 )
                 on conflict(id) do update set
                    farm_id = excluded.farm_id,
                    label = excluded.label,
                    starts_at = excluded.starts_at,
                    ends_at = excluded.ends_at,
                    updated_at = excluded.updated_at",
                params![
                    blackout_period.blackout_period_id.to_string(),
                    blackout_period.farm_id.to_string(),
                    blackout_period.label,
                    blackout_period.starts_at,
                    blackout_period.ends_at,
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "save blackout period",
                source,
            })?;

        Ok(())
    }

    fn delete_missing_pickup_locations(
        &self,
        farm_id: FarmId,
        pickup_locations: &[PickupLocationRecord],
    ) -> Result<(), AppSqliteError> {
        delete_missing_rows(
            self.connection,
            "pickup_locations",
            "id",
            farm_id,
            pickup_locations
                .iter()
                .map(|pickup_location| pickup_location.pickup_location_id)
                .collect::<Vec<_>>()
                .as_slice(),
            "delete missing pickup locations",
        )
    }

    fn delete_missing_fulfillment_windows(
        &self,
        farm_id: FarmId,
        fulfillment_windows: &[FulfillmentWindowRecord],
    ) -> Result<(), AppSqliteError> {
        delete_missing_rows(
            self.connection,
            "fulfillment_windows",
            "id",
            farm_id,
            fulfillment_windows
                .iter()
                .map(|fulfillment_window| fulfillment_window.fulfillment_window_id)
                .collect::<Vec<_>>()
                .as_slice(),
            "delete missing fulfillment windows",
        )
    }

    fn delete_missing_blackout_periods(
        &self,
        farm_id: FarmId,
        blackout_periods: &[BlackoutPeriodRecord],
    ) -> Result<(), AppSqliteError> {
        delete_missing_rows(
            self.connection,
            "blackout_periods",
            "id",
            farm_id,
            blackout_periods
                .iter()
                .map(|blackout_period| blackout_period.blackout_period_id)
                .collect::<Vec<_>>()
                .as_slice(),
            "delete missing blackout periods",
        )
    }
}

fn validate_projection(projection: &FarmRulesProjection) -> Result<FarmId, AppSqliteError> {
    let farm_profile =
        projection
            .farm_profile
            .as_ref()
            .ok_or(AppSqliteError::InvalidProjection {
                reason: "farm rules projection must include a farm profile",
            })?;
    let farm_id = farm_profile.farm_id;

    if projection
        .pickup_locations
        .iter()
        .any(|pickup_location| pickup_location.farm_id != farm_id)
    {
        return Err(AppSqliteError::InvalidProjection {
            reason: "pickup locations must belong to the farm profile",
        });
    }

    if projection
        .operating_rules
        .as_ref()
        .is_some_and(|operating_rules| operating_rules.farm_id != farm_id)
    {
        return Err(AppSqliteError::InvalidProjection {
            reason: "operating rules must belong to the farm profile",
        });
    }

    let pickup_location_ids = projection
        .pickup_locations
        .iter()
        .map(|pickup_location| pickup_location.pickup_location_id)
        .collect::<std::collections::BTreeSet<_>>();

    if projection
        .fulfillment_windows
        .iter()
        .any(|fulfillment_window| fulfillment_window.farm_id != farm_id)
    {
        return Err(AppSqliteError::InvalidProjection {
            reason: "fulfillment windows must belong to the farm profile",
        });
    }

    if projection
        .fulfillment_windows
        .iter()
        .any(|fulfillment_window| {
            !pickup_location_ids.contains(&fulfillment_window.pickup_location_id)
        })
    {
        return Err(AppSqliteError::InvalidProjection {
            reason: "fulfillment windows must reference a saved pickup location",
        });
    }

    if projection
        .blackout_periods
        .iter()
        .any(|blackout_period| blackout_period.farm_id != farm_id)
    {
        return Err(AppSqliteError::InvalidProjection {
            reason: "blackout periods must belong to the farm profile",
        });
    }

    Ok(farm_id)
}

pub fn derive_farm_rules_readiness(projection: &FarmRulesProjection) -> FarmRulesReadiness {
    derive_farm_rules_readiness_parts(
        projection.farm_profile.as_ref(),
        &projection.pickup_locations,
        projection.operating_rules.as_ref(),
        &projection.fulfillment_windows,
        &projection.blackout_periods,
    )
}

fn derive_farm_rules_readiness_parts(
    farm_profile: Option<&FarmProfileRecord>,
    pickup_locations: &[PickupLocationRecord],
    operating_rules: Option<&FarmOperatingRulesRecord>,
    fulfillment_windows: &[FulfillmentWindowRecord],
    blackout_periods: &[BlackoutPeriodRecord],
) -> FarmRulesReadiness {
    let mut blockers = Vec::new();
    let mut timing_conflicts = Vec::new();

    if farm_profile.is_none_or(|farm_profile| {
        farm_profile.display_name.trim().is_empty()
            || farm_profile.timezone.trim().is_empty()
            || farm_profile.currency_code.trim().is_empty()
    }) {
        blockers.push(FarmReadinessBlocker::MissingProfileBasics);
    }

    if !pickup_locations
        .iter()
        .any(|pickup_location| pickup_location_is_present(pickup_location))
    {
        blockers.push(FarmReadinessBlocker::MissingPickupLocation);
    }

    if operating_rules.is_none_or(|operating_rules| {
        operating_rules.substitution_policy.trim().is_empty()
            || operating_rules.missed_pickup_policy.trim().is_empty()
    }) {
        blockers.push(FarmReadinessBlocker::MissingOperatingRules);
    }

    if fulfillment_windows.is_empty() {
        blockers.push(FarmReadinessBlocker::MissingFulfillmentWindow);
    }

    for fulfillment_window in fulfillment_windows {
        if fulfillment_window.starts_at.trim().is_empty()
            || fulfillment_window.ends_at.trim().is_empty()
            || fulfillment_window.ends_at <= fulfillment_window.starts_at
        {
            timing_conflicts.push(FarmTimingConflict {
                kind: FarmTimingConflictKind::FulfillmentWindowEndsBeforeStart,
                fulfillment_window_id: Some(fulfillment_window.fulfillment_window_id),
                blackout_period_id: None,
            });
        }

        if fulfillment_window.order_cutoff_at.trim().is_empty()
            || fulfillment_window.order_cutoff_at >= fulfillment_window.starts_at
        {
            timing_conflicts.push(FarmTimingConflict {
                kind: FarmTimingConflictKind::FulfillmentWindowCutoffAfterStart,
                fulfillment_window_id: Some(fulfillment_window.fulfillment_window_id),
                blackout_period_id: None,
            });
        }
    }

    for blackout_period in blackout_periods {
        if blackout_period.starts_at.trim().is_empty()
            || blackout_period.ends_at.trim().is_empty()
            || blackout_period.ends_at <= blackout_period.starts_at
        {
            timing_conflicts.push(FarmTimingConflict {
                kind: FarmTimingConflictKind::BlackoutPeriodEndsBeforeStart,
                fulfillment_window_id: None,
                blackout_period_id: Some(blackout_period.blackout_period_id),
            });
        }

        for fulfillment_window in fulfillment_windows {
            if blackout_period.starts_at < fulfillment_window.ends_at
                && blackout_period.ends_at > fulfillment_window.starts_at
            {
                timing_conflicts.push(FarmTimingConflict {
                    kind: FarmTimingConflictKind::BlackoutOverlapsFulfillmentWindow,
                    fulfillment_window_id: Some(fulfillment_window.fulfillment_window_id),
                    blackout_period_id: Some(blackout_period.blackout_period_id),
                });
            }
        }
    }

    FarmRulesReadiness {
        blockers,
        timing_conflicts,
    }
}

fn pickup_location_is_present(pickup_location: &PickupLocationRecord) -> bool {
    !pickup_location.label.trim().is_empty() && !pickup_location.address_line.trim().is_empty()
}

fn delete_missing_rows<T>(
    connection: &Connection,
    table_name: &str,
    id_column: &str,
    farm_id: FarmId,
    keep_ids: &[T],
    operation: &'static str,
) -> Result<(), AppSqliteError>
where
    T: fmt::Display,
{
    if keep_ids.is_empty() {
        let sql = format!("delete from {table_name} where farm_id = ?");
        connection
            .execute(&sql, [farm_id.to_string()])
            .map_err(|source| AppSqliteError::Query { operation, source })?;
        return Ok(());
    }

    let placeholders = std::iter::repeat_n("?", keep_ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "delete from {table_name} where farm_id = ? and {id_column} not in ({placeholders})"
    );
    let mut values = Vec::with_capacity(keep_ids.len() + 1);
    values.push(farm_id.to_string());
    values.extend(keep_ids.iter().map(ToString::to_string));

    connection
        .execute(&sql, params_from_iter(values.iter()))
        .map_err(|source| AppSqliteError::Query { operation, source })?;

    Ok(())
}

fn collect_rows<T, F>(
    operation: &'static str,
    rows: rusqlite::MappedRows<'_, F>,
) -> Result<Vec<T>, AppSqliteError>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    let mut values = Vec::new();

    for row in rows {
        values.push(row.map_err(|source| AppSqliteError::Query { operation, source })?);
    }

    Ok(values)
}

fn parse_typed_id<T>(field: &'static str, value: String) -> Result<T, AppSqliteError>
where
    T: FromStr,
{
    value
        .parse()
        .map_err(|_| AppSqliteError::DecodeId { field, value })
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

fn parse_u16(field: &'static str, value: i64) -> Result<u16, AppSqliteError> {
    value.try_into().map_err(|_| AppSqliteError::DecodeEnum {
        field,
        value: value.to_string(),
    })
}

fn farm_readiness_storage_key(ready: bool) -> &'static str {
    match ready {
        true => "ready",
        false => "incomplete",
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
        BlackoutPeriodId, BlackoutPeriodRecord, FarmId, FarmOperatingRulesRecord,
        FarmProfileRecord, FarmReadinessBlocker, FarmRulesProjection, FarmRulesReadiness,
        FarmTimingConflictKind, FulfillmentWindowId, FulfillmentWindowRecord, PickupLocationId,
        PickupLocationRecord,
    };

    use crate::{AppSqliteStore, DatabaseTarget};

    use super::{AppFarmRulesRepository, derive_farm_rules_readiness};

    #[test]
    fn load_farm_rules_returns_default_when_farm_is_missing() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = AppFarmRulesRepository::new(store.connection());

        let projection = repository
            .load_farm_rules(FarmId::new())
            .expect("missing farm rules should load");

        assert_eq!(projection, FarmRulesProjection::default());
    }

    #[test]
    fn save_farm_rules_round_trips_across_restart() {
        let path = temp_database_path("farm-rules-roundtrip");
        let farm_id = FarmId::new();
        let pickup_location_id = PickupLocationId::new();
        let fulfillment_window_id = FulfillmentWindowId::new();
        let blackout_period_id = BlackoutPeriodId::new();
        let projection = FarmRulesProjection {
            farm_profile: Some(FarmProfileRecord {
                farm_id,
                display_name: "North field farm".to_owned(),
                timezone: "UTC".to_owned(),
                currency_code: "USD".to_owned(),
            }),
            pickup_locations: vec![PickupLocationRecord {
                pickup_location_id,
                farm_id,
                label: "Barn pickup".to_owned(),
                address_line: "14 Orchard Lane".to_owned(),
                directions: Some("Drive to the red barn.".to_owned()),
                is_default: true,
            }],
            operating_rules: Some(FarmOperatingRulesRecord {
                farm_id,
                promise_lead_hours: 24,
                substitution_policy: "ask_customer".to_owned(),
                missed_pickup_policy: "hold_next_window".to_owned(),
            }),
            fulfillment_windows: vec![FulfillmentWindowRecord {
                fulfillment_window_id,
                farm_id,
                pickup_location_id,
                label: "Friday pickup".to_owned(),
                starts_at: "2026-04-25T14:00:00Z".to_owned(),
                ends_at: "2026-04-25T18:00:00Z".to_owned(),
                order_cutoff_at: "2026-04-24T18:00:00Z".to_owned(),
            }],
            blackout_periods: vec![BlackoutPeriodRecord {
                blackout_period_id,
                farm_id,
                label: "Spring break".to_owned(),
                starts_at: "2026-05-01T00:00:00Z".to_owned(),
                ends_at: "2026-05-03T23:59:59Z".to_owned(),
            }],
            readiness: FarmRulesReadiness::ready(),
        };

        {
            let store = AppSqliteStore::open(DatabaseTarget::Path(path.clone()))
                .expect("store should open");
            let repository = AppFarmRulesRepository::new(store.connection());
            repository
                .save_farm_rules(&projection)
                .expect("farm rules should save");
        }

        let reopened =
            AppSqliteStore::open(DatabaseTarget::Path(path.clone())).expect("store should reopen");
        let loaded = reopened
            .load_farm_rules(farm_id)
            .expect("farm rules should load after restart");

        assert_eq!(loaded, projection);

        drop(reopened);
        remove_database_artifacts(&path);
    }

    #[test]
    fn load_farm_rules_derives_missing_and_conflict_readiness() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = AppFarmRulesRepository::new(store.connection());
        let farm_id = FarmId::new();
        let pickup_location_id = PickupLocationId::new();
        let fulfillment_window_id = FulfillmentWindowId::new();
        let blackout_period_id = BlackoutPeriodId::new();

        repository
            .save_farm_rules(&FarmRulesProjection {
                farm_profile: Some(FarmProfileRecord {
                    farm_id,
                    display_name: "North field farm".to_owned(),
                    timezone: "UTC".to_owned(),
                    currency_code: "USD".to_owned(),
                }),
                pickup_locations: vec![PickupLocationRecord {
                    pickup_location_id,
                    farm_id,
                    label: "Barn pickup".to_owned(),
                    address_line: "14 Orchard Lane".to_owned(),
                    directions: None,
                    is_default: true,
                }],
                operating_rules: None,
                fulfillment_windows: vec![FulfillmentWindowRecord {
                    fulfillment_window_id,
                    farm_id,
                    pickup_location_id,
                    label: "Friday pickup".to_owned(),
                    starts_at: "2026-04-25T14:00:00Z".to_owned(),
                    ends_at: "2026-04-25T13:00:00Z".to_owned(),
                    order_cutoff_at: "2026-04-25T15:00:00Z".to_owned(),
                }],
                blackout_periods: vec![BlackoutPeriodRecord {
                    blackout_period_id,
                    farm_id,
                    label: "Spring break".to_owned(),
                    starts_at: "2026-04-25T12:00:00Z".to_owned(),
                    ends_at: "2026-04-25T16:00:00Z".to_owned(),
                }],
                readiness: FarmRulesReadiness::ready(),
            })
            .expect("farm rules should save");

        let projection = repository
            .load_farm_rules(farm_id)
            .expect("farm rules should load");

        assert_eq!(
            projection.readiness.blockers,
            vec![FarmReadinessBlocker::MissingOperatingRules]
        );
        assert_eq!(projection.readiness.timing_conflicts.len(), 3);
        assert_eq!(
            projection.readiness.timing_conflicts[0].kind,
            FarmTimingConflictKind::FulfillmentWindowEndsBeforeStart
        );
        assert_eq!(
            projection.readiness.timing_conflicts[1].kind,
            FarmTimingConflictKind::FulfillmentWindowCutoffAfterStart
        );
        assert_eq!(
            projection.readiness.timing_conflicts[2].kind,
            FarmTimingConflictKind::BlackoutOverlapsFulfillmentWindow
        );
    }

    #[test]
    fn blank_pickup_location_rows_do_not_count_as_present_for_readiness() {
        let farm_id = FarmId::new();
        let readiness = derive_farm_rules_readiness(&FarmRulesProjection {
            farm_profile: Some(FarmProfileRecord {
                farm_id,
                display_name: "North field farm".to_owned(),
                timezone: "UTC".to_owned(),
                currency_code: "USD".to_owned(),
            }),
            pickup_locations: vec![PickupLocationRecord {
                pickup_location_id: PickupLocationId::new(),
                farm_id,
                label: "   ".to_owned(),
                address_line: String::new(),
                directions: None,
                is_default: true,
            }],
            operating_rules: Some(FarmOperatingRulesRecord {
                farm_id,
                promise_lead_hours: 24,
                substitution_policy: "ask_customer".to_owned(),
                missed_pickup_policy: "hold_next_window".to_owned(),
            }),
            fulfillment_windows: Vec::new(),
            blackout_periods: Vec::new(),
            readiness: FarmRulesReadiness::ready(),
        });

        assert!(
            readiness
                .blockers
                .contains(&FarmReadinessBlocker::MissingPickupLocation)
        );
        assert!(
            readiness
                .blockers
                .contains(&FarmReadinessBlocker::MissingFulfillmentWindow)
        );
    }

    #[test]
    fn complete_pickup_location_row_counts_as_present_for_readiness() {
        let farm_id = FarmId::new();
        let pickup_location_id = PickupLocationId::new();
        let readiness = derive_farm_rules_readiness(&FarmRulesProjection {
            farm_profile: Some(FarmProfileRecord {
                farm_id,
                display_name: "North field farm".to_owned(),
                timezone: "UTC".to_owned(),
                currency_code: "USD".to_owned(),
            }),
            pickup_locations: vec![PickupLocationRecord {
                pickup_location_id,
                farm_id,
                label: "Barn pickup".to_owned(),
                address_line: "14 Orchard Lane".to_owned(),
                directions: None,
                is_default: true,
            }],
            operating_rules: Some(FarmOperatingRulesRecord {
                farm_id,
                promise_lead_hours: 24,
                substitution_policy: "ask_customer".to_owned(),
                missed_pickup_policy: "hold_next_window".to_owned(),
            }),
            fulfillment_windows: vec![FulfillmentWindowRecord {
                fulfillment_window_id: FulfillmentWindowId::new(),
                farm_id,
                pickup_location_id,
                label: "Friday pickup".to_owned(),
                starts_at: "2026-04-25T14:00:00Z".to_owned(),
                ends_at: "2026-04-25T18:00:00Z".to_owned(),
                order_cutoff_at: "2026-04-24T18:00:00Z".to_owned(),
            }],
            blackout_periods: Vec::new(),
            readiness: FarmRulesReadiness::ready(),
        });

        assert!(
            !readiness
                .blockers
                .contains(&FarmReadinessBlocker::MissingPickupLocation)
        );
        assert!(readiness.blockers.is_empty());
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
