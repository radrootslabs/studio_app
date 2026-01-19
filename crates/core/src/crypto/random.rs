use super::RadrootsClientCryptoError;

pub fn fill_random(bytes: &mut [u8]) -> Result<(), RadrootsClientCryptoError> {
    if bytes.is_empty() {
        return Ok(());
    }
    fill_random_inner(bytes)
}

#[cfg(target_arch = "wasm32")]
fn fill_random_inner(bytes: &mut [u8]) -> Result<(), RadrootsClientCryptoError> {
    let window = web_sys::window().ok_or(RadrootsClientCryptoError::CryptoUndefined)?;
    let crypto = window.crypto().map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    crypto
        .get_random_values_with_u8_array(bytes)
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn fill_random_inner(bytes: &mut [u8]) -> Result<(), RadrootsClientCryptoError> {
    getrandom::getrandom(bytes).map_err(|_| RadrootsClientCryptoError::CryptoUndefined)
}

#[cfg(test)]
mod tests {
    use super::fill_random;

    #[test]
    fn fill_random_noop_for_empty() {
        let mut bytes = [];
        assert!(fill_random(&mut bytes).is_ok());
    }
}
