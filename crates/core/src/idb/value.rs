#[cfg(target_arch = "wasm32")]
pub type RadrootsClientIdbValue = wasm_bindgen::JsValue;
#[cfg(not(target_arch = "wasm32"))]
pub type RadrootsClientIdbValue = ();

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
pub fn idb_value_as_bytes(value: &RadrootsClientIdbValue) -> Option<Vec<u8>> {
    if value.is_instance_of::<js_sys::Uint8Array>()
        || value.is_instance_of::<js_sys::ArrayBuffer>()
        || js_sys::ArrayBuffer::is_view(value)
    {
        let array = js_sys::Uint8Array::new(value);
        let mut out = vec![0u8; array.length() as usize];
        array.copy_to(&mut out);
        return Some(out);
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn idb_value_as_bytes(_value: &RadrootsClientIdbValue) -> Option<Vec<u8>> {
    None
}

#[cfg(test)]
mod tests {
    use super::{idb_value_as_bytes, RadrootsClientIdbValue};

    #[test]
    fn non_wasm_returns_none() {
        let value: RadrootsClientIdbValue = ();
        assert!(idb_value_as_bytes(&value).is_none());
    }
}
