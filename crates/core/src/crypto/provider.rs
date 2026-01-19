use async_trait::async_trait;

#[cfg(target_arch = "wasm32")]
use crate::crypto::random::fill_random;
#[cfg(target_arch = "wasm32")]
use crate::crypto::registry::{
    crypto_registry_get_device_material,
    crypto_registry_set_device_material,
};

use super::{RadrootsClientCryptoError, RadrootsClientKeyMaterialProvider};

const DEVICE_PROVIDER_ID: &str = "device";
#[cfg(target_arch = "wasm32")]
const DEVICE_MATERIAL_BYTES: usize = 32;

pub struct RadrootsClientDeviceKeyMaterialProvider;

#[async_trait(?Send)]
impl RadrootsClientKeyMaterialProvider for RadrootsClientDeviceKeyMaterialProvider {
    async fn get_key_material(&self) -> Result<Vec<u8>, RadrootsClientCryptoError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            return Err(RadrootsClientCryptoError::CryptoUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(existing) = crypto_registry_get_device_material().await? {
                return Ok(existing);
            }
            let mut material = vec![0u8; DEVICE_MATERIAL_BYTES];
            fill_random(&mut material)?;
            crypto_registry_set_device_material(&material).await?;
            Ok(material)
        }
    }

    async fn get_provider_id(&self) -> Result<String, RadrootsClientCryptoError> {
        Ok(String::from(DEVICE_PROVIDER_ID))
    }
}

#[cfg(test)]
mod tests {
    use super::RadrootsClientDeviceKeyMaterialProvider;
    use crate::crypto::{RadrootsClientCryptoError, RadrootsClientKeyMaterialProvider};

    #[test]
    fn provider_id_is_device() {
        let provider = RadrootsClientDeviceKeyMaterialProvider;
        let id = futures::executor::block_on(provider.get_provider_id())
            .expect("provider id");
        assert_eq!(id, "device");
    }

    #[test]
    fn non_wasm_material_errors() {
        let provider = RadrootsClientDeviceKeyMaterialProvider;
        let err = futures::executor::block_on(provider.get_key_material())
            .expect_err("missing crypto");
        assert_eq!(err, RadrootsClientCryptoError::CryptoUndefined);
    }
}
