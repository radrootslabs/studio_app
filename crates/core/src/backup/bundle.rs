use std::fmt;

use crate::crypto::{
    RadrootsClientCryptoError,
    RadrootsClientCryptoRegistryExport,
    RadrootsClientKeyMaterialProvider,
    RadrootsClientWebCryptoService,
};

use super::{
    backup_bundle_decode,
    backup_bundle_encode,
    RadrootsClientBackupBundle,
    RadrootsClientBackupBundleManifest,
    RadrootsClientBackupBundlePayload,
    RadrootsClientBackupDatastoreStore,
    RadrootsClientBackupError,
    RadrootsClientBackupKeystoreStore,
    RadrootsClientBackupSqlStore,
    RadrootsClientBackupStoreRef,
    RADROOTS_CLIENT_BACKUP_BUNDLE_VERSION,
};

#[derive(Debug)]
pub enum RadrootsClientBackupBundleError<SqlErr, KeystoreErr, DatastoreErr> {
    Backup(RadrootsClientBackupError),
    Crypto(RadrootsClientCryptoError),
    Sql(SqlErr),
    Keystore(KeystoreErr),
    Datastore(DatastoreErr),
}

impl<SqlErr: fmt::Display, KeystoreErr: fmt::Display, DatastoreErr: fmt::Display> fmt::Display
    for RadrootsClientBackupBundleError<SqlErr, KeystoreErr, DatastoreErr>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RadrootsClientBackupBundleError::Backup(err) => err.fmt(f),
            RadrootsClientBackupBundleError::Crypto(err) => err.fmt(f),
            RadrootsClientBackupBundleError::Sql(err) => err.fmt(f),
            RadrootsClientBackupBundleError::Keystore(err) => err.fmt(f),
            RadrootsClientBackupBundleError::Datastore(err) => err.fmt(f),
        }
    }
}

impl<SqlErr, KeystoreErr, DatastoreErr> std::error::Error
    for RadrootsClientBackupBundleError<SqlErr, KeystoreErr, DatastoreErr>
where
    SqlErr: std::error::Error + 'static,
    KeystoreErr: std::error::Error + 'static,
    DatastoreErr: std::error::Error + 'static,
{
}

pub type RadrootsClientBackupBundleResult<T, SqlErr, KeystoreErr, DatastoreErr> =
    Result<T, RadrootsClientBackupBundleError<SqlErr, KeystoreErr, DatastoreErr>>;

pub struct RadrootsClientBackupBundleBuildOpts<'a, SqlStore, KeystoreStore, DatastoreStore>
where
    SqlStore: RadrootsClientBackupSqlStore + ?Sized,
    KeystoreStore: RadrootsClientBackupKeystoreStore + ?Sized,
    DatastoreStore: RadrootsClientBackupDatastoreStore + ?Sized,
{
    pub sql_store: Option<&'a SqlStore>,
    pub keystore_store: Option<&'a KeystoreStore>,
    pub datastore_store: Option<&'a DatastoreStore>,
    pub app_version: Option<&'a str>,
    pub crypto_service: Option<&'a dyn RadrootsClientWebCryptoService>,
    pub key_material_provider: Option<&'a dyn RadrootsClientKeyMaterialProvider>,
}

pub struct RadrootsClientBackupBundleImportOpts<'a, SqlStore, KeystoreStore, DatastoreStore>
where
    SqlStore: RadrootsClientBackupSqlStore + ?Sized,
    KeystoreStore: RadrootsClientBackupKeystoreStore + ?Sized,
    DatastoreStore: RadrootsClientBackupDatastoreStore + ?Sized,
{
    pub sql_store: Option<&'a SqlStore>,
    pub keystore_store: Option<&'a KeystoreStore>,
    pub datastore_store: Option<&'a DatastoreStore>,
    pub crypto_service: Option<&'a dyn RadrootsClientWebCryptoService>,
    pub key_material_provider: Option<&'a dyn RadrootsClientKeyMaterialProvider>,
    pub import_registry: bool,
}

fn now_millis() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        return js_sys::Date::now() as u64;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        return SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0);
    }
}

async fn collect_payloads<SqlStore, KeystoreStore, DatastoreStore>(
    opts: &RadrootsClientBackupBundleBuildOpts<'_, SqlStore, KeystoreStore, DatastoreStore>,
) -> RadrootsClientBackupBundleResult<
    Vec<RadrootsClientBackupBundlePayload>,
    SqlStore::Error,
    KeystoreStore::Error,
    DatastoreStore::Error,
>
where
    SqlStore: RadrootsClientBackupSqlStore + ?Sized,
    KeystoreStore: RadrootsClientBackupKeystoreStore + ?Sized,
    DatastoreStore: RadrootsClientBackupDatastoreStore + ?Sized,
{
    let mut payloads = Vec::new();
    if let Some(store) = opts.sql_store {
        let data = store
            .export_backup()
            .await
            .map_err(RadrootsClientBackupBundleError::Sql)?;
        payloads.push(RadrootsClientBackupBundlePayload::Sql {
            store_id: store.store_id().to_string(),
            data,
        });
    }
    if let Some(store) = opts.keystore_store {
        let data = store
            .export_backup()
            .await
            .map_err(RadrootsClientBackupBundleError::Keystore)?;
        payloads.push(RadrootsClientBackupBundlePayload::Keystore {
            store_id: store.store_id().to_string(),
            data,
        });
    }
    if let Some(store) = opts.datastore_store {
        let data = store
            .export_backup()
            .await
            .map_err(RadrootsClientBackupBundleError::Datastore)?;
        payloads.push(RadrootsClientBackupBundlePayload::Datastore {
            store_id: store.store_id().to_string(),
            data,
        });
    }
    Ok(payloads)
}

pub async fn backup_bundle_build<SqlStore, KeystoreStore, DatastoreStore>(
    opts: &RadrootsClientBackupBundleBuildOpts<'_, SqlStore, KeystoreStore, DatastoreStore>,
) -> RadrootsClientBackupBundleResult<
    RadrootsClientBackupBundle,
    SqlStore::Error,
    KeystoreStore::Error,
    DatastoreStore::Error,
>
where
    SqlStore: RadrootsClientBackupSqlStore + ?Sized,
    KeystoreStore: RadrootsClientBackupKeystoreStore + ?Sized,
    DatastoreStore: RadrootsClientBackupDatastoreStore + ?Sized,
{
    let payloads = collect_payloads(opts).await?;
    let stores = payloads
        .iter()
        .map(|payload| RadrootsClientBackupStoreRef {
            store_id: payload.store_id().to_string(),
            store_type: payload.store_type(),
        })
        .collect();
    let crypto_registry = match opts.crypto_service {
        Some(crypto) => crypto
            .export_registry()
            .await
            .map_err(RadrootsClientBackupBundleError::Crypto)?,
        None => RadrootsClientCryptoRegistryExport {
            stores: Vec::new(),
            keys: Vec::new(),
        },
    };
    Ok(RadrootsClientBackupBundle {
        manifest: RadrootsClientBackupBundleManifest {
            version: RADROOTS_CLIENT_BACKUP_BUNDLE_VERSION,
            created_at: now_millis(),
            app_version: opts.app_version.map(str::to_string),
            stores,
            crypto_registry,
        },
        payloads,
    })
}

pub async fn backup_bundle_export<SqlStore, KeystoreStore, DatastoreStore>(
    opts: &RadrootsClientBackupBundleBuildOpts<'_, SqlStore, KeystoreStore, DatastoreStore>,
) -> RadrootsClientBackupBundleResult<
    Vec<u8>,
    SqlStore::Error,
    KeystoreStore::Error,
    DatastoreStore::Error,
>
where
    SqlStore: RadrootsClientBackupSqlStore + ?Sized,
    KeystoreStore: RadrootsClientBackupKeystoreStore + ?Sized,
    DatastoreStore: RadrootsClientBackupDatastoreStore + ?Sized,
{
    let provider = opts
        .key_material_provider
        .ok_or(RadrootsClientBackupBundleError::Backup(
            RadrootsClientBackupError::ProviderMissing,
        ))?;
    let bundle = backup_bundle_build(opts).await?;
    backup_bundle_encode(&bundle, provider)
        .await
        .map_err(RadrootsClientBackupBundleError::Backup)
}

pub async fn backup_bundle_import<SqlStore, KeystoreStore, DatastoreStore>(
    blob: &[u8],
    opts: &RadrootsClientBackupBundleImportOpts<'_, SqlStore, KeystoreStore, DatastoreStore>,
) -> RadrootsClientBackupBundleResult<
    RadrootsClientBackupBundle,
    SqlStore::Error,
    KeystoreStore::Error,
    DatastoreStore::Error,
>
where
    SqlStore: RadrootsClientBackupSqlStore + ?Sized,
    KeystoreStore: RadrootsClientBackupKeystoreStore + ?Sized,
    DatastoreStore: RadrootsClientBackupDatastoreStore + ?Sized,
{
    let provider = opts
        .key_material_provider
        .ok_or(RadrootsClientBackupBundleError::Backup(
            RadrootsClientBackupError::ProviderMissing,
        ))?;
    let bundle = backup_bundle_decode(blob, provider)
        .await
        .map_err(RadrootsClientBackupBundleError::Backup)?;
    if opts.import_registry {
        if let Some(crypto) = opts.crypto_service {
            crypto
                .import_registry(bundle.manifest.crypto_registry.clone())
                .await
                .map_err(RadrootsClientBackupBundleError::Crypto)?;
        }
    }
    for payload in &bundle.payloads {
        match payload {
            RadrootsClientBackupBundlePayload::Sql { store_id, data } => {
                if let Some(store) = opts.sql_store {
                    if store.store_id() == store_id {
                        store
                            .import_backup(data.clone())
                            .await
                            .map_err(RadrootsClientBackupBundleError::Sql)?;
                    }
                }
            }
            RadrootsClientBackupBundlePayload::Keystore { store_id, data } => {
                if let Some(store) = opts.keystore_store {
                    if store.store_id() == store_id {
                        store
                            .import_backup(data.clone())
                            .await
                            .map_err(RadrootsClientBackupBundleError::Keystore)?;
                    }
                }
            }
            RadrootsClientBackupBundlePayload::Datastore { store_id, data } => {
                if let Some(store) = opts.datastore_store {
                    if store.store_id() == store_id {
                        store
                            .import_backup(data.clone())
                            .await
                            .map_err(RadrootsClientBackupBundleError::Datastore)?;
                    }
                }
            }
        }
    }
    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::{
        backup_bundle_build,
        backup_bundle_export,
        RadrootsClientBackupBundleBuildOpts,
        RadrootsClientBackupBundleError,
    };
    use crate::backup::{
        RadrootsClientBackupBundlePayload,
        RadrootsClientBackupDatastorePayload,
        RadrootsClientBackupDatastoreStore,
        RadrootsClientBackupError,
        RadrootsClientBackupKeystorePayload,
        RadrootsClientBackupKeystoreStore,
        RadrootsClientBackupSqlPayload,
        RadrootsClientBackupSqlStore,
    };

    #[derive(Debug)]
    struct StubError;

    impl std::fmt::Display for StubError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("stub")
        }
    }

    impl std::error::Error for StubError {}

    struct StubSqlStore;
    struct StubKeystoreStore;
    struct StubDatastoreStore;

    #[async_trait(?Send)]
    impl RadrootsClientBackupSqlStore for StubSqlStore {
        type Error = StubError;

        async fn export_backup(&self) -> Result<RadrootsClientBackupSqlPayload, Self::Error> {
            Ok(RadrootsClientBackupSqlPayload {
                bytes_b64: "sql".to_string(),
            })
        }

        async fn import_backup(
            &self,
            _payload: RadrootsClientBackupSqlPayload,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn store_id(&self) -> &str {
            "sql-store"
        }
    }

    #[async_trait(?Send)]
    impl RadrootsClientBackupKeystoreStore for StubKeystoreStore {
        type Error = StubError;

        async fn export_backup(
            &self,
        ) -> Result<RadrootsClientBackupKeystorePayload, Self::Error> {
            Ok(RadrootsClientBackupKeystorePayload {
                entries: Vec::new(),
            })
        }

        async fn import_backup(
            &self,
            _payload: RadrootsClientBackupKeystorePayload,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn store_id(&self) -> &str {
            "keystore"
        }
    }

    #[async_trait(?Send)]
    impl RadrootsClientBackupDatastoreStore for StubDatastoreStore {
        type Error = StubError;

        async fn export_backup(
            &self,
        ) -> Result<RadrootsClientBackupDatastorePayload, Self::Error> {
            Ok(RadrootsClientBackupDatastorePayload {
                entries: Vec::new(),
            })
        }

        async fn import_backup(
            &self,
            _payload: RadrootsClientBackupDatastorePayload,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn store_id(&self) -> &str {
            "datastore"
        }
    }

    #[test]
    fn build_collects_payloads() {
        let sql = StubSqlStore;
        let keystore = StubKeystoreStore;
        let datastore = StubDatastoreStore;
        let opts = RadrootsClientBackupBundleBuildOpts {
            sql_store: Some(&sql),
            keystore_store: Some(&keystore),
            datastore_store: Some(&datastore),
            app_version: Some("1.2.3"),
            crypto_service: None,
            key_material_provider: None,
        };
        let bundle = futures::executor::block_on(backup_bundle_build(&opts))
            .expect("bundle");
        assert_eq!(bundle.payloads.len(), 3);
        assert_eq!(bundle.manifest.stores.len(), 3);
        assert_eq!(bundle.manifest.app_version.as_deref(), Some("1.2.3"));
        assert!(bundle.manifest.crypto_registry.stores.is_empty());
        assert!(bundle.manifest.crypto_registry.keys.is_empty());
        assert!(matches!(
            bundle.payloads[0],
            RadrootsClientBackupBundlePayload::Sql { .. }
        ));
    }

    #[test]
    fn export_requires_provider() {
        let sql = StubSqlStore;
        let opts: RadrootsClientBackupBundleBuildOpts<
            StubSqlStore,
            StubKeystoreStore,
            StubDatastoreStore,
        > = RadrootsClientBackupBundleBuildOpts {
            sql_store: Some(&sql),
            keystore_store: None,
            datastore_store: None,
            app_version: None,
            crypto_service: None,
            key_material_provider: None,
        };
        let err = futures::executor::block_on(backup_bundle_export(&opts))
            .expect_err("missing provider");
        match err {
            RadrootsClientBackupBundleError::Backup(
                RadrootsClientBackupError::ProviderMissing,
            ) => {}
            other => panic!("unexpected error: {other}"),
        }
    }
}
