use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use radroots_nostr::prelude::{
    radroots_event_from_nostr,
    radroots_nostr_build_event,
    RadrootsNostrEvent,
    RadrootsNostrKeys,
    RadrootsNostrSecretKey,
};
use radroots_sql_core::error::SqlError;
use radroots_sql_core::{ExecOutcome, SqlExecutor};
use radroots_sql_core::sqlite_util;
use radroots_tangle_db::{export_manifest, TangleSql};
use radroots_tangle_events::{
    radroots_tangle_ingest_event,
    radroots_tangle_sync_all,
    RadrootsTangleEventDraft,
    RadrootsTangleFarmSelector,
    RadrootsTangleSyncRequest,
};
use rusqlite::{params_from_iter, Connection};
use sha2::{Digest, Sha256};

use crate::idb::{IDB_CONFIG_TANGLE, RadrootsClientIdbConfig};
use crate::sql::{
    RadrootsClientSqlCipherConfig,
    RadrootsClientSqlEngine,
    RadrootsClientSqlEngineConfig,
    RadrootsClientSqlError,
    RadrootsClientSqlMigrationState,
    RadrootsClientSqlParams,
    RadrootsClientSqlResultRow,
    RadrootsClientWebSqlEngine,
};

use super::error::RadrootsClientTangleError;
use super::types::{
    IFarmCreate,
    IFarmCreateResolve,
    IFarmDelete,
    IFarmDeleteResolve,
    IFarmFindMany,
    IFarmFindManyResolve,
    IFarmFindOne,
    IFarmFindOneResolve,
    IFarmUpdate,
    IFarmUpdateResolve,
    IFarmGcsLocationCreate,
    IFarmGcsLocationCreateResolve,
    IFarmGcsLocationDelete,
    IFarmGcsLocationDeleteResolve,
    IFarmGcsLocationFindMany,
    IFarmGcsLocationFindManyResolve,
    IFarmGcsLocationFindOne,
    IFarmGcsLocationFindOneResolve,
    IFarmGcsLocationUpdate,
    IFarmGcsLocationUpdateResolve,
    IFarmMemberClaimCreate,
    IFarmMemberClaimCreateResolve,
    IFarmMemberClaimDelete,
    IFarmMemberClaimDeleteResolve,
    IFarmMemberClaimFindMany,
    IFarmMemberClaimFindManyResolve,
    IFarmMemberClaimFindOne,
    IFarmMemberClaimFindOneResolve,
    IFarmMemberClaimUpdate,
    IFarmMemberClaimUpdateResolve,
    IFarmMemberCreate,
    IFarmMemberCreateResolve,
    IFarmMemberDelete,
    IFarmMemberDeleteResolve,
    IFarmMemberFindMany,
    IFarmMemberFindManyResolve,
    IFarmMemberFindOne,
    IFarmMemberFindOneResolve,
    IFarmMemberUpdate,
    IFarmMemberUpdateResolve,
    IFarmTagCreate,
    IFarmTagCreateResolve,
    IFarmTagDelete,
    IFarmTagDeleteResolve,
    IFarmTagFindMany,
    IFarmTagFindManyResolve,
    IFarmTagFindOne,
    IFarmTagFindOneResolve,
    IFarmTagUpdate,
    IFarmTagUpdateResolve,
    IGcsLocationCreate,
    IGcsLocationCreateResolve,
    IGcsLocationDelete,
    IGcsLocationDeleteResolve,
    IGcsLocationFindMany,
    IGcsLocationFindManyResolve,
    IGcsLocationFindOne,
    IGcsLocationFindOneResolve,
    IGcsLocationUpdate,
    IGcsLocationUpdateResolve,
    ILogErrorCreate,
    ILogErrorCreateResolve,
    ILogErrorDelete,
    ILogErrorDeleteResolve,
    ILogErrorFindMany,
    ILogErrorFindManyResolve,
    ILogErrorFindOne,
    ILogErrorFindOneResolve,
    ILogErrorUpdate,
    ILogErrorUpdateResolve,
    IMediaImageCreate,
    IMediaImageCreateResolve,
    IMediaImageDelete,
    IMediaImageDeleteResolve,
    IMediaImageFindMany,
    IMediaImageFindManyResolve,
    IMediaImageFindOne,
    IMediaImageFindOneResolve,
    IMediaImageUpdate,
    IMediaImageUpdateResolve,
    INostrEventStateCreate,
    INostrEventStateCreateResolve,
    INostrEventStateDelete,
    INostrEventStateDeleteResolve,
    INostrEventStateFindMany,
    INostrEventStateFindManyResolve,
    INostrEventStateFindOne,
    INostrEventStateFindOneResolve,
    INostrEventStateUpdate,
    INostrEventStateUpdateResolve,
    INostrProfileCreate,
    INostrProfileCreateResolve,
    INostrProfileDelete,
    INostrProfileDeleteResolve,
    INostrProfileFindMany,
    INostrProfileFindManyResolve,
    INostrProfileFindOne,
    INostrProfileFindOneResolve,
    INostrProfileUpdate,
    INostrProfileUpdateResolve,
    INostrRelayCreate,
    INostrRelayCreateResolve,
    INostrRelayDelete,
    INostrRelayDeleteResolve,
    INostrRelayFindMany,
    INostrRelayFindManyResolve,
    INostrRelayFindOne,
    INostrRelayFindOneResolve,
    INostrRelayUpdate,
    INostrRelayUpdateResolve,
    IPlotCreate,
    IPlotCreateResolve,
    IPlotDelete,
    IPlotDeleteResolve,
    IPlotFindMany,
    IPlotFindManyResolve,
    IPlotFindOne,
    IPlotFindOneResolve,
    IPlotUpdate,
    IPlotUpdateResolve,
    IPlotGcsLocationCreate,
    IPlotGcsLocationCreateResolve,
    IPlotGcsLocationDelete,
    IPlotGcsLocationDeleteResolve,
    IPlotGcsLocationFindMany,
    IPlotGcsLocationFindManyResolve,
    IPlotGcsLocationFindOne,
    IPlotGcsLocationFindOneResolve,
    IPlotGcsLocationUpdate,
    IPlotGcsLocationUpdateResolve,
    IPlotTagCreate,
    IPlotTagCreateResolve,
    IPlotTagDelete,
    IPlotTagDeleteResolve,
    IPlotTagFindMany,
    IPlotTagFindManyResolve,
    IPlotTagFindOne,
    IPlotTagFindOneResolve,
    IPlotTagUpdate,
    IPlotTagUpdateResolve,
    ITradeProductCreate,
    ITradeProductCreateResolve,
    ITradeProductDelete,
    ITradeProductDeleteResolve,
    ITradeProductFindMany,
    ITradeProductFindManyResolve,
    ITradeProductFindOne,
    ITradeProductFindOneResolve,
    ITradeProductUpdate,
    ITradeProductUpdateResolve,
    INostrProfileRelayRelation,
    INostrProfileRelayResolve,
    ITradeProductLocationRelation,
    ITradeProductLocationResolve,
    ITradeProductMediaRelation,
    ITradeProductMediaResolve,
    RadrootsClientTangle,
    RadrootsClientTangleConfig,
    RadrootsClientTangleDatabaseExportManifest,
    RadrootsClientTangleDatabaseExportManifestClient,
    RadrootsClientTangleDatabaseExportOptions,
    RadrootsClientTangleDatabaseExportSnapshot,
    RadrootsClientTangleDatabaseJsonExport,
    RadrootsClientTangleNostrSyncOptions,
    RadrootsClientTangleNostrSyncSigner,
    RadrootsClientTangleNostrSyncSummary,
    RadrootsClientTangleResult,
};

const DEFAULT_TANGLE_STORE_KEY: &str = "radroots-pwa-v1-tangle-db";

pub struct RadrootsClientWebTangle {
    store_key: String,
    idb_config: RadrootsClientIdbConfig,
    cipher_config: RadrootsClientSqlCipherConfig,
    sql_wasm_path: Option<String>,
    engine: RefCell<Option<Rc<RadrootsClientWebSqlEngine>>>,
    init_in_progress: RefCell<bool>,
}

impl RadrootsClientWebTangle {
    pub fn new(config: Option<RadrootsClientTangleConfig>) -> Self {
        let config = config.unwrap_or(RadrootsClientTangleConfig {
            store_key: None,
            idb_config: None,
            cipher_config: None,
            sql_wasm_path: None,
        });
        let store_key = config
            .store_key
            .unwrap_or_else(|| DEFAULT_TANGLE_STORE_KEY.to_string());
        let idb_config = config.idb_config.unwrap_or(IDB_CONFIG_TANGLE);
        let cipher_config =
            config
                .cipher_config
                .unwrap_or(RadrootsClientSqlCipherConfig::Disabled);
        let sql_wasm_path = config.sql_wasm_path;
        Self {
            store_key,
            idb_config,
            cipher_config,
            sql_wasm_path,
            engine: RefCell::new(None),
            init_in_progress: RefCell::new(false),
        }
    }

    fn engine_config(&self) -> RadrootsClientSqlEngineConfig {
        RadrootsClientSqlEngineConfig {
            store_key: self.store_key.clone(),
            idb_config: self.idb_config,
            cipher_config: self.cipher_config.clone(),
            sql_wasm_path: self.sql_wasm_path.clone(),
        }
    }

    async fn init_engine(&self) -> RadrootsClientTangleResult<Rc<RadrootsClientWebSqlEngine>> {
        let engine = RadrootsClientWebSqlEngine::create(self.engine_config())
            .await
            .map_err(map_engine_error)?;
        let tangle = self.tangle(&engine);
        tangle.migrate_up().map_err(|_| RadrootsClientTangleError::InitFailure)?;
        let engine = Rc::new(engine);
        self.engine.borrow_mut().replace(engine.clone());
        Ok(engine)
    }

    async fn ensure_ready(&self) -> RadrootsClientTangleResult<Rc<RadrootsClientWebSqlEngine>> {
        if let Some(engine) = self.engine.borrow().as_ref() {
            return Ok(Rc::clone(engine));
        }
        if *self.init_in_progress.borrow() {
            return Err(RadrootsClientTangleError::InitFailure);
        }
        *self.init_in_progress.borrow_mut() = true;
        let result = self.init_engine().await;
        *self.init_in_progress.borrow_mut() = false;
        result
    }

    fn tangle(&self, engine: &RadrootsClientWebSqlEngine) -> TangleSql<TangleSqlExecutor> {
        TangleSql::new(TangleSqlExecutor::new(engine.shared_connection()))
    }
}

#[async_trait(?Send)]
impl RadrootsClientTangle for RadrootsClientWebTangle {
    async fn init(&self) -> RadrootsClientTangleResult<()> {
        let _ = self.ensure_ready().await?;
        Ok(())
    }

    async fn close(&self) -> RadrootsClientTangleResult<()> {
        if let Some(engine) = self.engine.borrow_mut().take() {
            engine
                .close()
                .await
                .map_err(map_engine_error)?;
        }
        *self.init_in_progress.borrow_mut() = false;
        Ok(())
    }

    async fn migration_state(&self) -> RadrootsClientTangleResult<RadrootsClientSqlMigrationState> {
        let engine = self.ensure_ready().await?;
        let rows = engine
            .query(
                "select id, name, applied_at from __migrations order by id asc",
                RadrootsClientSqlParams::Positional(Vec::new()),
            )
            .map_err(map_engine_error)?;
        migration_state_from_rows(rows)
    }

    async fn reset(&self) -> RadrootsClientTangleResult<RadrootsClientSqlMigrationState> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        tangle.migrate_down().map_err(|_| RadrootsClientTangleError::InvalidResponse)?;
        tangle.migrate_up().map_err(|_| RadrootsClientTangleError::InvalidResponse)?;
        self.migration_state().await
    }

    async fn reinit(&self) -> RadrootsClientTangleResult<RadrootsClientSqlMigrationState> {
        if let Some(engine) = self.engine.borrow_mut().take() {
            engine
                .purge_storage()
                .await
                .map_err(map_engine_error)?;
            engine
                .close()
                .await
                .map_err(map_engine_error)?;
        }
        self.migration_state().await
    }

    fn get_store_key(&self) -> &str {
        &self.store_key
    }

    async fn export_json(
        &self,
    ) -> RadrootsClientTangleResult<RadrootsClientTangleDatabaseJsonExport> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        tangle
            .backup_database()
            .map_err(|_| RadrootsClientTangleError::InvalidResponse)
    }

    async fn import_json(
        &self,
        backup: RadrootsClientTangleDatabaseJsonExport,
    ) -> RadrootsClientTangleResult<()> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        tangle
            .restore_database(&backup)
            .map_err(|_| RadrootsClientTangleError::InvalidResponse)
    }

    async fn export_database(
        &self,
        opts: RadrootsClientTangleDatabaseExportOptions,
    ) -> RadrootsClientTangleResult<RadrootsClientTangleDatabaseExportSnapshot> {
        if let Some(store_key) = opts.store_key.clone() {
            if store_key != self.store_key {
                let alt = RadrootsClientWebTangle::new(Some(RadrootsClientTangleConfig {
                    store_key: Some(store_key),
                    idb_config: Some(self.idb_config),
                    cipher_config: Some(self.cipher_config.clone()),
                    sql_wasm_path: self.sql_wasm_path.clone(),
                }));
                let mut opts = opts.clone();
                opts.store_key = None;
                let snapshot = alt.export_database(opts).await?;
                let _ = alt.close().await;
                return Ok(snapshot);
            }
        }
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        let manifest_rs = export_manifest(tangle.executor())
            .map_err(|_| RadrootsClientTangleError::InvalidResponse)?;
        let db_bytes = engine.export_bytes().map_err(map_engine_error)?;
        let db_sha256 = sha256_hex(&db_bytes);
        let exported_at = export_timestamp();
        let manifest_client = RadrootsClientTangleDatabaseExportManifestClient {
            app_name: opts.app_name,
            app_version: opts.app_version,
            exported_at,
            db_sha256,
            db_size_bytes: db_bytes.len() as u64,
            store_key: self.store_key.clone(),
            nostr_event: None,
        };
        let manifest = RadrootsClientTangleDatabaseExportManifest {
            rust: manifest_rs,
            client: manifest_client,
        };
        Ok(RadrootsClientTangleDatabaseExportSnapshot { manifest, db_bytes })
    }

    async fn nostr_sync_all(
        &self,
        opts: RadrootsClientTangleNostrSyncOptions,
    ) -> RadrootsClientTangleResult<RadrootsClientTangleNostrSyncSummary> {
        let engine = self.ensure_ready().await?;
        let relays = normalize_relays(&opts.relays);
        if relays.is_empty() || opts.signers.is_empty() {
            return Err(RadrootsClientTangleError::InvalidResponse);
        }
        let signer_map = build_signer_map(&opts.signers);
        if signer_map.is_empty() {
            return Err(RadrootsClientTangleError::InvalidResponse);
        }
        let tangle = self.tangle(&engine);
        let farms = map_db_result(tangle.farm_find_many(&IFarmFindMany { filter: None }))?;
        let mut event_map: BTreeMap<String, RadrootsTangleEventDraft> = BTreeMap::new();
        for farm in farms.results {
            let request = RadrootsTangleSyncRequest {
                farm: RadrootsTangleFarmSelector {
                    id: Some(farm.id),
                    d_tag: None,
                    pubkey: None,
                },
                options: None,
            };
            let bundle = radroots_tangle_sync_all(tangle.executor(), &request)
                .map_err(|_| RadrootsClientTangleError::InvalidResponse)?;
            for draft in bundle.events {
                let key = tangle_sync_event_key(&draft);
                event_map.entry(key).or_insert(draft);
            }
        }
        if event_map.is_empty() {
            return Ok(RadrootsClientTangleNostrSyncSummary {
                events_total: 0,
                events_published: 0,
                events_failed: 0,
                events_skipped: 0,
                missing_signers: Vec::new(),
            });
        }
        let mut events_published = 0;
        let mut events_failed = 0;
        let mut events_skipped = 0;
        let mut missing_signers = BTreeSet::new();

        for draft in event_map.values() {
            let Some(secret_key) = signer_map.get(&draft.author) else {
                missing_signers.insert(draft.author.clone());
                events_skipped += 1;
                continue;
            };
            let event = sign_draft_event(draft, secret_key)?;
            let event = radroots_event_from_nostr(&event);
            match radroots_tangle_ingest_event(tangle.executor(), &event) {
                Ok(_) => events_published += 1,
                Err(_) => events_failed += 1,
            }
        }
        let summary = RadrootsClientTangleNostrSyncSummary {
            events_total: event_map.len(),
            events_published,
            events_failed,
            events_skipped,
            missing_signers: missing_signers.into_iter().collect(),
        };
        if !summary.missing_signers.is_empty() || summary.events_failed > 0 {
            return Err(RadrootsClientTangleError::InvalidResponse);
        }
        let _ = relays;
        Ok(summary)
    }

    async fn farm_create(&self, opts: IFarmCreate) -> RadrootsClientTangleResult<IFarmCreateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_create(&opts))
    }

    async fn farm_find_one(&self, opts: IFarmFindOne) -> RadrootsClientTangleResult<IFarmFindOneResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_find_one(&opts))
    }

    async fn farm_find_many(&self, opts: IFarmFindMany) -> RadrootsClientTangleResult<IFarmFindManyResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_find_many(&opts))
    }

    async fn farm_delete(&self, opts: IFarmDelete) -> RadrootsClientTangleResult<IFarmDeleteResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_delete(&opts))
    }

    async fn farm_update(&self, opts: IFarmUpdate) -> RadrootsClientTangleResult<IFarmUpdateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_update(&opts))
    }

    async fn plot_create(&self, opts: IPlotCreate) -> RadrootsClientTangleResult<IPlotCreateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.plot_create(&opts))
    }

    async fn plot_find_one(&self, opts: IPlotFindOne) -> RadrootsClientTangleResult<IPlotFindOneResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.plot_find_one(&opts))
    }

    async fn plot_find_many(&self, opts: IPlotFindMany) -> RadrootsClientTangleResult<IPlotFindManyResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.plot_find_many(&opts))
    }

    async fn plot_delete(&self, opts: IPlotDelete) -> RadrootsClientTangleResult<IPlotDeleteResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.plot_delete(&opts))
    }

    async fn plot_update(&self, opts: IPlotUpdate) -> RadrootsClientTangleResult<IPlotUpdateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.plot_update(&opts))
    }

    async fn gcs_location_create(&self, opts: IGcsLocationCreate) -> RadrootsClientTangleResult<IGcsLocationCreateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.gcs_location_create(&opts))
    }

    async fn gcs_location_find_one(&self, opts: IGcsLocationFindOne) -> RadrootsClientTangleResult<IGcsLocationFindOneResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.gcs_location_find_one(&opts))
    }

    async fn gcs_location_find_many(&self, opts: IGcsLocationFindMany) -> RadrootsClientTangleResult<IGcsLocationFindManyResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.gcs_location_find_many(&opts))
    }

    async fn gcs_location_delete(&self, opts: IGcsLocationDelete) -> RadrootsClientTangleResult<IGcsLocationDeleteResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.gcs_location_delete(&opts))
    }

    async fn gcs_location_update(&self, opts: IGcsLocationUpdate) -> RadrootsClientTangleResult<IGcsLocationUpdateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.gcs_location_update(&opts))
    }

    async fn farm_gcs_location_create(&self, opts: IFarmGcsLocationCreate) -> RadrootsClientTangleResult<IFarmGcsLocationCreateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_gcs_location_create(&opts))
    }

    async fn farm_gcs_location_find_one(&self, opts: IFarmGcsLocationFindOne) -> RadrootsClientTangleResult<IFarmGcsLocationFindOneResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_gcs_location_find_one(&opts))
    }

    async fn farm_gcs_location_find_many(&self, opts: IFarmGcsLocationFindMany) -> RadrootsClientTangleResult<IFarmGcsLocationFindManyResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_gcs_location_find_many(&opts))
    }

    async fn farm_gcs_location_delete(&self, opts: IFarmGcsLocationDelete) -> RadrootsClientTangleResult<IFarmGcsLocationDeleteResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_gcs_location_delete(&opts))
    }

    async fn farm_gcs_location_update(&self, opts: IFarmGcsLocationUpdate) -> RadrootsClientTangleResult<IFarmGcsLocationUpdateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_gcs_location_update(&opts))
    }

    async fn plot_gcs_location_create(&self, opts: IPlotGcsLocationCreate) -> RadrootsClientTangleResult<IPlotGcsLocationCreateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.plot_gcs_location_create(&opts))
    }

    async fn plot_gcs_location_find_one(&self, opts: IPlotGcsLocationFindOne) -> RadrootsClientTangleResult<IPlotGcsLocationFindOneResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.plot_gcs_location_find_one(&opts))
    }

    async fn plot_gcs_location_find_many(&self, opts: IPlotGcsLocationFindMany) -> RadrootsClientTangleResult<IPlotGcsLocationFindManyResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.plot_gcs_location_find_many(&opts))
    }

    async fn plot_gcs_location_delete(&self, opts: IPlotGcsLocationDelete) -> RadrootsClientTangleResult<IPlotGcsLocationDeleteResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.plot_gcs_location_delete(&opts))
    }

    async fn plot_gcs_location_update(&self, opts: IPlotGcsLocationUpdate) -> RadrootsClientTangleResult<IPlotGcsLocationUpdateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.plot_gcs_location_update(&opts))
    }

    async fn farm_tag_create(&self, opts: IFarmTagCreate) -> RadrootsClientTangleResult<IFarmTagCreateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_tag_create(&opts))
    }

    async fn farm_tag_find_one(&self, opts: IFarmTagFindOne) -> RadrootsClientTangleResult<IFarmTagFindOneResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_tag_find_one(&opts))
    }

    async fn farm_tag_find_many(&self, opts: IFarmTagFindMany) -> RadrootsClientTangleResult<IFarmTagFindManyResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_tag_find_many(&opts))
    }

    async fn farm_tag_delete(&self, opts: IFarmTagDelete) -> RadrootsClientTangleResult<IFarmTagDeleteResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_tag_delete(&opts))
    }

    async fn farm_tag_update(&self, opts: IFarmTagUpdate) -> RadrootsClientTangleResult<IFarmTagUpdateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_tag_update(&opts))
    }

    async fn plot_tag_create(&self, opts: IPlotTagCreate) -> RadrootsClientTangleResult<IPlotTagCreateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.plot_tag_create(&opts))
    }

    async fn plot_tag_find_one(&self, opts: IPlotTagFindOne) -> RadrootsClientTangleResult<IPlotTagFindOneResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.plot_tag_find_one(&opts))
    }

    async fn plot_tag_find_many(&self, opts: IPlotTagFindMany) -> RadrootsClientTangleResult<IPlotTagFindManyResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.plot_tag_find_many(&opts))
    }

    async fn plot_tag_delete(&self, opts: IPlotTagDelete) -> RadrootsClientTangleResult<IPlotTagDeleteResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.plot_tag_delete(&opts))
    }

    async fn plot_tag_update(&self, opts: IPlotTagUpdate) -> RadrootsClientTangleResult<IPlotTagUpdateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.plot_tag_update(&opts))
    }

    async fn farm_member_create(&self, opts: IFarmMemberCreate) -> RadrootsClientTangleResult<IFarmMemberCreateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_member_create(&opts))
    }

    async fn farm_member_find_one(&self, opts: IFarmMemberFindOne) -> RadrootsClientTangleResult<IFarmMemberFindOneResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_member_find_one(&opts))
    }

    async fn farm_member_find_many(&self, opts: IFarmMemberFindMany) -> RadrootsClientTangleResult<IFarmMemberFindManyResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_member_find_many(&opts))
    }

    async fn farm_member_delete(&self, opts: IFarmMemberDelete) -> RadrootsClientTangleResult<IFarmMemberDeleteResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_member_delete(&opts))
    }

    async fn farm_member_update(&self, opts: IFarmMemberUpdate) -> RadrootsClientTangleResult<IFarmMemberUpdateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_member_update(&opts))
    }

    async fn farm_member_claim_create(&self, opts: IFarmMemberClaimCreate) -> RadrootsClientTangleResult<IFarmMemberClaimCreateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_member_claim_create(&opts))
    }

    async fn farm_member_claim_find_one(&self, opts: IFarmMemberClaimFindOne) -> RadrootsClientTangleResult<IFarmMemberClaimFindOneResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_member_claim_find_one(&opts))
    }

    async fn farm_member_claim_find_many(&self, opts: IFarmMemberClaimFindMany) -> RadrootsClientTangleResult<IFarmMemberClaimFindManyResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_member_claim_find_many(&opts))
    }

    async fn farm_member_claim_delete(&self, opts: IFarmMemberClaimDelete) -> RadrootsClientTangleResult<IFarmMemberClaimDeleteResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_member_claim_delete(&opts))
    }

    async fn farm_member_claim_update(&self, opts: IFarmMemberClaimUpdate) -> RadrootsClientTangleResult<IFarmMemberClaimUpdateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.farm_member_claim_update(&opts))
    }

    async fn nostr_event_state_create(&self, opts: INostrEventStateCreate) -> RadrootsClientTangleResult<INostrEventStateCreateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_event_state_create(&opts))
    }

    async fn nostr_event_state_find_one(&self, opts: INostrEventStateFindOne) -> RadrootsClientTangleResult<INostrEventStateFindOneResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_event_state_find_one(&opts))
    }

    async fn nostr_event_state_find_many(&self, opts: INostrEventStateFindMany) -> RadrootsClientTangleResult<INostrEventStateFindManyResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_event_state_find_many(&opts))
    }

    async fn nostr_event_state_delete(&self, opts: INostrEventStateDelete) -> RadrootsClientTangleResult<INostrEventStateDeleteResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_event_state_delete(&opts))
    }

    async fn nostr_event_state_update(&self, opts: INostrEventStateUpdate) -> RadrootsClientTangleResult<INostrEventStateUpdateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_event_state_update(&opts))
    }

    async fn log_error_create(&self, opts: ILogErrorCreate) -> RadrootsClientTangleResult<ILogErrorCreateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.log_error_create(&opts))
    }

    async fn log_error_find_one(&self, opts: ILogErrorFindOne) -> RadrootsClientTangleResult<ILogErrorFindOneResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.log_error_find_one(&opts))
    }

    async fn log_error_find_many(&self, opts: ILogErrorFindMany) -> RadrootsClientTangleResult<ILogErrorFindManyResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.log_error_find_many(&opts))
    }

    async fn log_error_delete(&self, opts: ILogErrorDelete) -> RadrootsClientTangleResult<ILogErrorDeleteResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.log_error_delete(&opts))
    }

    async fn log_error_update(&self, opts: ILogErrorUpdate) -> RadrootsClientTangleResult<ILogErrorUpdateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.log_error_update(&opts))
    }

    async fn media_image_create(&self, opts: IMediaImageCreate) -> RadrootsClientTangleResult<IMediaImageCreateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.media_image_create(&opts))
    }

    async fn media_image_find_one(&self, opts: IMediaImageFindOne) -> RadrootsClientTangleResult<IMediaImageFindOneResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.media_image_find_one(&opts))
    }

    async fn media_image_find_many(&self, opts: IMediaImageFindMany) -> RadrootsClientTangleResult<IMediaImageFindManyResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.media_image_find_many(&opts))
    }

    async fn media_image_delete(&self, opts: IMediaImageDelete) -> RadrootsClientTangleResult<IMediaImageDeleteResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.media_image_delete(&opts))
    }

    async fn media_image_update(&self, opts: IMediaImageUpdate) -> RadrootsClientTangleResult<IMediaImageUpdateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.media_image_update(&opts))
    }

    async fn nostr_profile_create(&self, opts: INostrProfileCreate) -> RadrootsClientTangleResult<INostrProfileCreateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_profile_create(&opts))
    }

    async fn nostr_profile_find_one(&self, opts: INostrProfileFindOne) -> RadrootsClientTangleResult<INostrProfileFindOneResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_profile_find_one(&opts))
    }

    async fn nostr_profile_find_many(&self, opts: INostrProfileFindMany) -> RadrootsClientTangleResult<INostrProfileFindManyResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_profile_find_many(&opts))
    }

    async fn nostr_profile_delete(&self, opts: INostrProfileDelete) -> RadrootsClientTangleResult<INostrProfileDeleteResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_profile_delete(&opts))
    }

    async fn nostr_profile_update(&self, opts: INostrProfileUpdate) -> RadrootsClientTangleResult<INostrProfileUpdateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_profile_update(&opts))
    }

    async fn nostr_relay_create(&self, opts: INostrRelayCreate) -> RadrootsClientTangleResult<INostrRelayCreateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_relay_create(&opts))
    }

    async fn nostr_relay_find_one(&self, opts: INostrRelayFindOne) -> RadrootsClientTangleResult<INostrRelayFindOneResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_relay_find_one(&opts))
    }

    async fn nostr_relay_find_many(&self, opts: INostrRelayFindMany) -> RadrootsClientTangleResult<INostrRelayFindManyResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_relay_find_many(&opts))
    }

    async fn nostr_relay_delete(&self, opts: INostrRelayDelete) -> RadrootsClientTangleResult<INostrRelayDeleteResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_relay_delete(&opts))
    }

    async fn nostr_relay_update(&self, opts: INostrRelayUpdate) -> RadrootsClientTangleResult<INostrRelayUpdateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_relay_update(&opts))
    }

    async fn trade_product_create(&self, opts: ITradeProductCreate) -> RadrootsClientTangleResult<ITradeProductCreateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.trade_product_create(&opts))
    }

    async fn trade_product_find_one(&self, opts: ITradeProductFindOne) -> RadrootsClientTangleResult<ITradeProductFindOneResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.trade_product_find_one(&opts))
    }

    async fn trade_product_find_many(&self, opts: ITradeProductFindMany) -> RadrootsClientTangleResult<ITradeProductFindManyResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.trade_product_find_many(&opts))
    }

    async fn trade_product_delete(&self, opts: ITradeProductDelete) -> RadrootsClientTangleResult<ITradeProductDeleteResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.trade_product_delete(&opts))
    }

    async fn trade_product_update(&self, opts: ITradeProductUpdate) -> RadrootsClientTangleResult<ITradeProductUpdateResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.trade_product_update(&opts))
    }

    async fn nostr_profile_relay_set(&self, opts: INostrProfileRelayRelation) -> RadrootsClientTangleResult<INostrProfileRelayResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_profile_relay_set(&opts))
    }

    async fn nostr_profile_relay_unset(&self, opts: INostrProfileRelayRelation) -> RadrootsClientTangleResult<INostrProfileRelayResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.nostr_profile_relay_unset(&opts))
    }

    async fn trade_product_location_set(&self, opts: ITradeProductLocationRelation) -> RadrootsClientTangleResult<ITradeProductLocationResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.trade_product_location_set(&opts))
    }

    async fn trade_product_location_unset(&self, opts: ITradeProductLocationRelation) -> RadrootsClientTangleResult<ITradeProductLocationResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.trade_product_location_unset(&opts))
    }

    async fn trade_product_media_set(&self, opts: ITradeProductMediaRelation) -> RadrootsClientTangleResult<ITradeProductMediaResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.trade_product_media_set(&opts))
    }

    async fn trade_product_media_unset(&self, opts: ITradeProductMediaRelation) -> RadrootsClientTangleResult<ITradeProductMediaResolve> {
        let engine = self.ensure_ready().await?;
        let tangle = self.tangle(&engine);
        map_db_result(tangle.trade_product_media_unset(&opts))
    }

}

struct TangleSqlExecutor {
    conn: Arc<Mutex<Connection>>,
}

impl TangleSqlExecutor {
    fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

impl SqlExecutor for TangleSqlExecutor {
    fn exec(&self, sql: &str, params_json: &str) -> Result<ExecOutcome, SqlError> {
        let binds = sqlite_util::parse_params(params_json)?;
        let conn = self.conn.lock().map_err(|_| SqlError::Internal)?;
        let changes = conn
            .execute(sql, params_from_iter(binds.into_iter()))
            .map_err(SqlError::from)?;
        let last_insert_id = conn.last_insert_rowid();
        Ok(ExecOutcome {
            changes: changes as i64,
            last_insert_id,
        })
    }

    fn query_raw(&self, sql: &str, params_json: &str) -> Result<String, SqlError> {
        let binds = sqlite_util::parse_params(params_json)?;
        let rows = {
            let conn = self.conn.lock().map_err(|_| SqlError::Internal)?;
            let mut stmt = conn.prepare(sql).map_err(SqlError::from)?;
            let mapped = stmt.query_map(
                params_from_iter(binds.into_iter()),
                sqlite_util::row_to_json,
            )?;
            mapped
                .collect::<Result<Vec<_>, _>>()
                .map_err(SqlError::from)?
        };
        serde_json::to_string(&rows).map_err(SqlError::from)
    }

    fn begin(&self) -> Result<(), SqlError> {
        let conn = self.conn.lock().map_err(|_| SqlError::Internal)?;
        conn.execute("BEGIN", []).map_err(SqlError::from)?;
        Ok(())
    }

    fn commit(&self) -> Result<(), SqlError> {
        let conn = self.conn.lock().map_err(|_| SqlError::Internal)?;
        conn.execute("COMMIT", []).map_err(SqlError::from)?;
        Ok(())
    }

    fn rollback(&self) -> Result<(), SqlError> {
        let conn = self.conn.lock().map_err(|_| SqlError::Internal)?;
        conn.execute("ROLLBACK", []).map_err(SqlError::from)?;
        Ok(())
    }
}

fn map_engine_error(err: RadrootsClientSqlError) -> RadrootsClientTangleError {
    match err {
        RadrootsClientSqlError::EngineUnavailable => RadrootsClientTangleError::RuntimeUnavailable,
        RadrootsClientSqlError::IdbUndefined => RadrootsClientTangleError::RuntimeUnavailable,
        RadrootsClientSqlError::ImportFailure => RadrootsClientTangleError::InvalidResponse,
        RadrootsClientSqlError::ExportFailure => RadrootsClientTangleError::InvalidResponse,
        RadrootsClientSqlError::BackupFailure => RadrootsClientTangleError::InvalidResponse,
        RadrootsClientSqlError::InvalidParams => RadrootsClientTangleError::ParseFailure,
        RadrootsClientSqlError::QueryFailure => RadrootsClientTangleError::InvalidResponse,
    }
}

fn migration_state_from_rows(
    rows: Vec<RadrootsClientSqlResultRow>,
) -> RadrootsClientTangleResult<RadrootsClientSqlMigrationState> {
    let mut names = Vec::with_capacity(rows.len());
    for row in rows {
        let name = row
            .get("name")
            .and_then(|value| value.as_str())
            .ok_or(RadrootsClientTangleError::ParseFailure)?;
        names.push(name.to_string());
    }
    Ok(RadrootsClientSqlMigrationState {
        applied_names: names.clone(),
        applied_count: names.len(),
    })
}

fn map_db_result<T, E>(result: Result<T, E>) -> RadrootsClientTangleResult<T> {
    result.map_err(|_| RadrootsClientTangleError::InvalidResponse)
}

fn normalize_relays(relays: &[String]) -> Vec<String> {
    let mut unique = BTreeSet::new();
    for relay in relays {
        let relay = relay.trim();
        if relay.is_empty() {
            continue;
        }
        unique.insert(relay.to_string());
    }
    unique.into_iter().collect()
}

fn tangle_sync_event_key(draft: &RadrootsTangleEventDraft) -> String {
    let d_tag = draft_d_tag(&draft.tags);
    format!("{}:{}:{}", draft.kind, draft.author, d_tag.unwrap_or_default())
}

fn draft_d_tag(tags: &[Vec<String>]) -> Option<String> {
    for tag in tags {
        if tag.first().map(|value| value.as_str()) == Some("d") {
            if let Some(value) = tag.get(1) {
                return Some(value.clone());
            }
        }
    }
    None
}

fn build_signer_map(
    signers: &[RadrootsClientTangleNostrSyncSigner],
) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for signer in signers {
        let secret_key = match RadrootsNostrSecretKey::from_str(&signer.secret_key) {
            Ok(secret_key) => secret_key,
            Err(_) => continue,
        };
        let keys = RadrootsNostrKeys::new(secret_key);
        map.insert(keys.public_key().to_hex(), signer.secret_key.clone());
    }
    map
}

fn sign_draft_event(
    draft: &RadrootsTangleEventDraft,
    secret_key: &str,
) -> RadrootsClientTangleResult<RadrootsNostrEvent> {
    let secret_key = RadrootsNostrSecretKey::from_str(secret_key)
        .map_err(|_| RadrootsClientTangleError::CryptoUnavailable)?;
    let keys = RadrootsNostrKeys::new(secret_key);
    let builder = radroots_nostr_build_event(draft.kind, draft.content.clone(), draft.tags.clone())
        .map_err(|_| RadrootsClientTangleError::CryptoUnavailable)?;
    builder
        .sign_with_keys(&keys)
        .map_err(|_| RadrootsClientTangleError::CryptoUnavailable)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(target_arch = "wasm32")]
fn export_timestamp() -> String {
    js_sys::Date::new_0().to_iso_string().into()
}

#[cfg(not(target_arch = "wasm32"))]
fn export_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::{
        RadrootsClientWebTangle,
        DEFAULT_TANGLE_STORE_KEY,
    };
    use crate::tangle::{
        RadrootsClientTangle,
        RadrootsClientTangleError,
        RadrootsClientTangleNostrSyncOptions,
        RadrootsClientTangleNostrSyncSigner,
    };

    #[test]
    fn default_store_key_is_set() {
        let tangle = RadrootsClientWebTangle::new(None);
        assert_eq!(tangle.get_store_key(), DEFAULT_TANGLE_STORE_KEY);
    }

    #[test]
    fn nostr_sync_requires_relays() {
        let tangle = RadrootsClientWebTangle::new(None);
        let opts = RadrootsClientTangleNostrSyncOptions {
            relays: Vec::new(),
            signers: vec![RadrootsClientTangleNostrSyncSigner {
                secret_key: "deadbeef".to_string(),
            }],
            publish_timeout_ms: None,
        };
        let err = futures::executor::block_on(tangle.nostr_sync_all(opts))
            .expect_err("invalid response");
        assert_eq!(err, RadrootsClientTangleError::InvalidResponse);
    }
}
