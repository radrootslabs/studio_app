#![forbid(unsafe_code)]

#[cfg(target_arch = "wasm32")]
pub type RadrootsAppArrayBuffer = js_sys::ArrayBuffer;

#[cfg(not(target_arch = "wasm32"))]
pub type RadrootsAppArrayBuffer = Vec<u8>;

pub fn as_array_buffer(bytes: &[u8]) -> RadrootsAppArrayBuffer {
    #[cfg(target_arch = "wasm32")]
    {
        let array = js_sys::Uint8Array::from(bytes);
        array.buffer()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        bytes.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::as_array_buffer;

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn as_array_buffer_clones_bytes() {
        let bytes = vec![1u8, 2u8, 3u8];
        let buffer = as_array_buffer(&bytes);
        assert_eq!(buffer, bytes);
    }
}
