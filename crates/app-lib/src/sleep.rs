#![forbid(unsafe_code)]

pub async fn sleep(ms: u64) {
    #[cfg(target_arch = "wasm32")]
    {
        let delay = ms.min(u32::MAX as u64) as u32;
        gloo_timers::future::TimeoutFuture::new(delay).await;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
}

#[cfg(test)]
mod tests {
    use super::sleep;

    #[test]
    fn sleep_returns() {
        futures::executor::block_on(sleep(0));
    }
}
