use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use radroots_tangle_db::backup::DatabaseBackup;
use radroots_tangle_db::TangleDbExportManifestRs;
pub use radroots_tangle_db_schema::{
    farm::*,
    farm_gcs_location::*,
    farm_member::*,
    farm_member_claim::*,
    farm_tag::*,
    gcs_location::*,
    log_error::*,
    media_image::*,
    nostr_event_state::*,
    nostr_profile::*,
    nostr_profile_relay::*,
    nostr_relay::*,
    plot::*,
    plot_gcs_location::*,
    plot_tag::*,
    trade_product::*,
    trade_product_location::*,
    trade_product_media::*,
};
use radroots_tangle_events::{RadrootsTangleEventDraft, RadrootsTangleSyncBundle};

use crate::idb::RadrootsClientIdbConfig;
use crate::sql::{RadrootsClientSqlCipherConfig, RadrootsClientSqlMigrationState};

use super::RadrootsClientTangleError;

pub type RadrootsClientTangleResult<T> = Result<T, RadrootsClientTangleError>;
pub type RadrootsClientTangleDatabaseJsonExport = DatabaseBackup;
pub type RadrootsClientTangleDatabaseExportManifestRs = TangleDbExportManifestRs;
pub type RadrootsClientTangleNostrEventDraft = RadrootsTangleEventDraft;
pub type RadrootsClientTangleNostrSyncBundle = RadrootsTangleSyncBundle;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadrootsClientTangleDatabaseExportManifestClient {
    pub app_name: String,
    pub app_version: String,
    pub exported_at: String,
    pub db_sha256: String,
    pub db_size_bytes: u64,
    pub store_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nostr_event: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadrootsClientTangleDatabaseExportManifest {
    pub rust: RadrootsClientTangleDatabaseExportManifestRs,
    pub client: RadrootsClientTangleDatabaseExportManifestClient,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadrootsClientTangleDatabaseExportSnapshot {
    pub manifest: RadrootsClientTangleDatabaseExportManifest,
    pub db_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientTangleDatabaseExportOptions {
    pub app_name: String,
    pub app_version: String,
    pub store_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientTangleNostrSyncSigner {
    pub secret_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientTangleNostrSyncOptions {
    pub relays: Vec<String>,
    pub signers: Vec<RadrootsClientTangleNostrSyncSigner>,
    pub publish_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientTangleNostrSyncSummary {
    pub events_total: usize,
    pub events_published: usize,
    pub events_failed: usize,
    pub events_skipped: usize,
    pub missing_signers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientTangleConfig {
    pub store_key: Option<String>,
    pub idb_config: Option<RadrootsClientIdbConfig>,
    pub cipher_config: Option<RadrootsClientSqlCipherConfig>,
    pub sql_wasm_path: Option<String>,
}

#[async_trait(?Send)]
pub trait RadrootsClientTangle {
    async fn init(&self) -> RadrootsClientTangleResult<()>;
    async fn close(&self) -> RadrootsClientTangleResult<()>;
    async fn migration_state(
        &self,
    ) -> RadrootsClientTangleResult<RadrootsClientSqlMigrationState>;
    async fn reset(&self) -> RadrootsClientTangleResult<RadrootsClientSqlMigrationState>;
    async fn reinit(&self) -> RadrootsClientTangleResult<RadrootsClientSqlMigrationState>;
    fn get_store_key(&self) -> &str;
    async fn export_json(
        &self,
    ) -> RadrootsClientTangleResult<RadrootsClientTangleDatabaseJsonExport>;
    async fn import_json(
        &self,
        backup: RadrootsClientTangleDatabaseJsonExport,
    ) -> RadrootsClientTangleResult<()>;
    async fn export_database(
        &self,
        opts: RadrootsClientTangleDatabaseExportOptions,
    ) -> RadrootsClientTangleResult<RadrootsClientTangleDatabaseExportSnapshot>;
    async fn nostr_sync_all(
        &self,
        opts: RadrootsClientTangleNostrSyncOptions,
    ) -> RadrootsClientTangleResult<RadrootsClientTangleNostrSyncSummary>;
    async fn farm_create(&self, opts: IFarmCreate) -> RadrootsClientTangleResult<IFarmCreateResolve>;
    async fn farm_find_one(&self, opts: IFarmFindOne) -> RadrootsClientTangleResult<IFarmFindOneResolve>;
    async fn farm_find_many(&self, opts: IFarmFindMany) -> RadrootsClientTangleResult<IFarmFindManyResolve>;
    async fn farm_delete(&self, opts: IFarmDelete) -> RadrootsClientTangleResult<IFarmDeleteResolve>;
    async fn farm_update(&self, opts: IFarmUpdate) -> RadrootsClientTangleResult<IFarmUpdateResolve>;
    async fn plot_create(&self, opts: IPlotCreate) -> RadrootsClientTangleResult<IPlotCreateResolve>;
    async fn plot_find_one(&self, opts: IPlotFindOne) -> RadrootsClientTangleResult<IPlotFindOneResolve>;
    async fn plot_find_many(&self, opts: IPlotFindMany) -> RadrootsClientTangleResult<IPlotFindManyResolve>;
    async fn plot_delete(&self, opts: IPlotDelete) -> RadrootsClientTangleResult<IPlotDeleteResolve>;
    async fn plot_update(&self, opts: IPlotUpdate) -> RadrootsClientTangleResult<IPlotUpdateResolve>;
    async fn gcs_location_create(
        &self,
        opts: IGcsLocationCreate,
    ) -> RadrootsClientTangleResult<IGcsLocationCreateResolve>;
    async fn gcs_location_find_one(
        &self,
        opts: IGcsLocationFindOne,
    ) -> RadrootsClientTangleResult<IGcsLocationFindOneResolve>;
    async fn gcs_location_find_many(
        &self,
        opts: IGcsLocationFindMany,
    ) -> RadrootsClientTangleResult<IGcsLocationFindManyResolve>;
    async fn gcs_location_delete(
        &self,
        opts: IGcsLocationDelete,
    ) -> RadrootsClientTangleResult<IGcsLocationDeleteResolve>;
    async fn gcs_location_update(
        &self,
        opts: IGcsLocationUpdate,
    ) -> RadrootsClientTangleResult<IGcsLocationUpdateResolve>;
    async fn farm_gcs_location_create(
        &self,
        opts: IFarmGcsLocationCreate,
    ) -> RadrootsClientTangleResult<IFarmGcsLocationCreateResolve>;
    async fn farm_gcs_location_find_one(
        &self,
        opts: IFarmGcsLocationFindOne,
    ) -> RadrootsClientTangleResult<IFarmGcsLocationFindOneResolve>;
    async fn farm_gcs_location_find_many(
        &self,
        opts: IFarmGcsLocationFindMany,
    ) -> RadrootsClientTangleResult<IFarmGcsLocationFindManyResolve>;
    async fn farm_gcs_location_delete(
        &self,
        opts: IFarmGcsLocationDelete,
    ) -> RadrootsClientTangleResult<IFarmGcsLocationDeleteResolve>;
    async fn farm_gcs_location_update(
        &self,
        opts: IFarmGcsLocationUpdate,
    ) -> RadrootsClientTangleResult<IFarmGcsLocationUpdateResolve>;
    async fn plot_gcs_location_create(
        &self,
        opts: IPlotGcsLocationCreate,
    ) -> RadrootsClientTangleResult<IPlotGcsLocationCreateResolve>;
    async fn plot_gcs_location_find_one(
        &self,
        opts: IPlotGcsLocationFindOne,
    ) -> RadrootsClientTangleResult<IPlotGcsLocationFindOneResolve>;
    async fn plot_gcs_location_find_many(
        &self,
        opts: IPlotGcsLocationFindMany,
    ) -> RadrootsClientTangleResult<IPlotGcsLocationFindManyResolve>;
    async fn plot_gcs_location_delete(
        &self,
        opts: IPlotGcsLocationDelete,
    ) -> RadrootsClientTangleResult<IPlotGcsLocationDeleteResolve>;
    async fn plot_gcs_location_update(
        &self,
        opts: IPlotGcsLocationUpdate,
    ) -> RadrootsClientTangleResult<IPlotGcsLocationUpdateResolve>;
    async fn farm_tag_create(
        &self,
        opts: IFarmTagCreate,
    ) -> RadrootsClientTangleResult<IFarmTagCreateResolve>;
    async fn farm_tag_find_one(
        &self,
        opts: IFarmTagFindOne,
    ) -> RadrootsClientTangleResult<IFarmTagFindOneResolve>;
    async fn farm_tag_find_many(
        &self,
        opts: IFarmTagFindMany,
    ) -> RadrootsClientTangleResult<IFarmTagFindManyResolve>;
    async fn farm_tag_delete(
        &self,
        opts: IFarmTagDelete,
    ) -> RadrootsClientTangleResult<IFarmTagDeleteResolve>;
    async fn farm_tag_update(
        &self,
        opts: IFarmTagUpdate,
    ) -> RadrootsClientTangleResult<IFarmTagUpdateResolve>;
    async fn plot_tag_create(
        &self,
        opts: IPlotTagCreate,
    ) -> RadrootsClientTangleResult<IPlotTagCreateResolve>;
    async fn plot_tag_find_one(
        &self,
        opts: IPlotTagFindOne,
    ) -> RadrootsClientTangleResult<IPlotTagFindOneResolve>;
    async fn plot_tag_find_many(
        &self,
        opts: IPlotTagFindMany,
    ) -> RadrootsClientTangleResult<IPlotTagFindManyResolve>;
    async fn plot_tag_delete(
        &self,
        opts: IPlotTagDelete,
    ) -> RadrootsClientTangleResult<IPlotTagDeleteResolve>;
    async fn plot_tag_update(
        &self,
        opts: IPlotTagUpdate,
    ) -> RadrootsClientTangleResult<IPlotTagUpdateResolve>;
    async fn farm_member_create(
        &self,
        opts: IFarmMemberCreate,
    ) -> RadrootsClientTangleResult<IFarmMemberCreateResolve>;
    async fn farm_member_find_one(
        &self,
        opts: IFarmMemberFindOne,
    ) -> RadrootsClientTangleResult<IFarmMemberFindOneResolve>;
    async fn farm_member_find_many(
        &self,
        opts: IFarmMemberFindMany,
    ) -> RadrootsClientTangleResult<IFarmMemberFindManyResolve>;
    async fn farm_member_delete(
        &self,
        opts: IFarmMemberDelete,
    ) -> RadrootsClientTangleResult<IFarmMemberDeleteResolve>;
    async fn farm_member_update(
        &self,
        opts: IFarmMemberUpdate,
    ) -> RadrootsClientTangleResult<IFarmMemberUpdateResolve>;
    async fn farm_member_claim_create(
        &self,
        opts: IFarmMemberClaimCreate,
    ) -> RadrootsClientTangleResult<IFarmMemberClaimCreateResolve>;
    async fn farm_member_claim_find_one(
        &self,
        opts: IFarmMemberClaimFindOne,
    ) -> RadrootsClientTangleResult<IFarmMemberClaimFindOneResolve>;
    async fn farm_member_claim_find_many(
        &self,
        opts: IFarmMemberClaimFindMany,
    ) -> RadrootsClientTangleResult<IFarmMemberClaimFindManyResolve>;
    async fn farm_member_claim_delete(
        &self,
        opts: IFarmMemberClaimDelete,
    ) -> RadrootsClientTangleResult<IFarmMemberClaimDeleteResolve>;
    async fn farm_member_claim_update(
        &self,
        opts: IFarmMemberClaimUpdate,
    ) -> RadrootsClientTangleResult<IFarmMemberClaimUpdateResolve>;
    async fn nostr_event_state_create(
        &self,
        opts: INostrEventStateCreate,
    ) -> RadrootsClientTangleResult<INostrEventStateCreateResolve>;
    async fn nostr_event_state_find_one(
        &self,
        opts: INostrEventStateFindOne,
    ) -> RadrootsClientTangleResult<INostrEventStateFindOneResolve>;
    async fn nostr_event_state_find_many(
        &self,
        opts: INostrEventStateFindMany,
    ) -> RadrootsClientTangleResult<INostrEventStateFindManyResolve>;
    async fn nostr_event_state_delete(
        &self,
        opts: INostrEventStateDelete,
    ) -> RadrootsClientTangleResult<INostrEventStateDeleteResolve>;
    async fn nostr_event_state_update(
        &self,
        opts: INostrEventStateUpdate,
    ) -> RadrootsClientTangleResult<INostrEventStateUpdateResolve>;
    async fn log_error_create(
        &self,
        opts: ILogErrorCreate,
    ) -> RadrootsClientTangleResult<ILogErrorCreateResolve>;
    async fn log_error_find_one(
        &self,
        opts: ILogErrorFindOne,
    ) -> RadrootsClientTangleResult<ILogErrorFindOneResolve>;
    async fn log_error_find_many(
        &self,
        opts: ILogErrorFindMany,
    ) -> RadrootsClientTangleResult<ILogErrorFindManyResolve>;
    async fn log_error_delete(
        &self,
        opts: ILogErrorDelete,
    ) -> RadrootsClientTangleResult<ILogErrorDeleteResolve>;
    async fn log_error_update(
        &self,
        opts: ILogErrorUpdate,
    ) -> RadrootsClientTangleResult<ILogErrorUpdateResolve>;
    async fn media_image_create(
        &self,
        opts: IMediaImageCreate,
    ) -> RadrootsClientTangleResult<IMediaImageCreateResolve>;
    async fn media_image_find_one(
        &self,
        opts: IMediaImageFindOne,
    ) -> RadrootsClientTangleResult<IMediaImageFindOneResolve>;
    async fn media_image_find_many(
        &self,
        opts: IMediaImageFindMany,
    ) -> RadrootsClientTangleResult<IMediaImageFindManyResolve>;
    async fn media_image_delete(
        &self,
        opts: IMediaImageDelete,
    ) -> RadrootsClientTangleResult<IMediaImageDeleteResolve>;
    async fn media_image_update(
        &self,
        opts: IMediaImageUpdate,
    ) -> RadrootsClientTangleResult<IMediaImageUpdateResolve>;
    async fn nostr_profile_create(
        &self,
        opts: INostrProfileCreate,
    ) -> RadrootsClientTangleResult<INostrProfileCreateResolve>;
    async fn nostr_profile_find_one(
        &self,
        opts: INostrProfileFindOne,
    ) -> RadrootsClientTangleResult<INostrProfileFindOneResolve>;
    async fn nostr_profile_find_many(
        &self,
        opts: INostrProfileFindMany,
    ) -> RadrootsClientTangleResult<INostrProfileFindManyResolve>;
    async fn nostr_profile_delete(
        &self,
        opts: INostrProfileDelete,
    ) -> RadrootsClientTangleResult<INostrProfileDeleteResolve>;
    async fn nostr_profile_update(
        &self,
        opts: INostrProfileUpdate,
    ) -> RadrootsClientTangleResult<INostrProfileUpdateResolve>;
    async fn nostr_relay_create(
        &self,
        opts: INostrRelayCreate,
    ) -> RadrootsClientTangleResult<INostrRelayCreateResolve>;
    async fn nostr_relay_find_one(
        &self,
        opts: INostrRelayFindOne,
    ) -> RadrootsClientTangleResult<INostrRelayFindOneResolve>;
    async fn nostr_relay_find_many(
        &self,
        opts: INostrRelayFindMany,
    ) -> RadrootsClientTangleResult<INostrRelayFindManyResolve>;
    async fn nostr_relay_delete(
        &self,
        opts: INostrRelayDelete,
    ) -> RadrootsClientTangleResult<INostrRelayDeleteResolve>;
    async fn nostr_relay_update(
        &self,
        opts: INostrRelayUpdate,
    ) -> RadrootsClientTangleResult<INostrRelayUpdateResolve>;
    async fn trade_product_create(
        &self,
        opts: ITradeProductCreate,
    ) -> RadrootsClientTangleResult<ITradeProductCreateResolve>;
    async fn trade_product_find_one(
        &self,
        opts: ITradeProductFindOne,
    ) -> RadrootsClientTangleResult<ITradeProductFindOneResolve>;
    async fn trade_product_find_many(
        &self,
        opts: ITradeProductFindMany,
    ) -> RadrootsClientTangleResult<ITradeProductFindManyResolve>;
    async fn trade_product_delete(
        &self,
        opts: ITradeProductDelete,
    ) -> RadrootsClientTangleResult<ITradeProductDeleteResolve>;
    async fn trade_product_update(
        &self,
        opts: ITradeProductUpdate,
    ) -> RadrootsClientTangleResult<ITradeProductUpdateResolve>;
    async fn nostr_profile_relay_set(
        &self,
        opts: INostrProfileRelayRelation,
    ) -> RadrootsClientTangleResult<INostrProfileRelayResolve>;
    async fn nostr_profile_relay_unset(
        &self,
        opts: INostrProfileRelayRelation,
    ) -> RadrootsClientTangleResult<INostrProfileRelayResolve>;
    async fn trade_product_location_set(
        &self,
        opts: ITradeProductLocationRelation,
    ) -> RadrootsClientTangleResult<ITradeProductLocationResolve>;
    async fn trade_product_location_unset(
        &self,
        opts: ITradeProductLocationRelation,
    ) -> RadrootsClientTangleResult<ITradeProductLocationResolve>;
    async fn trade_product_media_set(
        &self,
        opts: ITradeProductMediaRelation,
    ) -> RadrootsClientTangleResult<ITradeProductMediaResolve>;
    async fn trade_product_media_unset(
        &self,
        opts: ITradeProductMediaRelation,
    ) -> RadrootsClientTangleResult<ITradeProductMediaResolve>;
}
