#![forbid(unsafe_code)]

use crate::error::RadrootsAppUtilsError;
use std::future::Future;
use std::time::Duration;

pub async fn exe_iter<F, Fut>(
    callback: F,
    num: usize,
    delay_ms: u64,
) -> Result<(), RadrootsAppUtilsError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = ()>,
{
    if num == 0 {
        return Ok(());
    }
    for index in 0..num {
        callback().await;
        if index + 1 < num {
            sleep_ms(delay_ms).await?;
        }
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
async fn sleep_ms(delay_ms: u64) -> Result<(), RadrootsAppUtilsError> {
    gloo_timers::future::TimeoutFuture::new(delay_ms as u32).await;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
async fn sleep_ms(delay_ms: u64) -> Result<(), RadrootsAppUtilsError> {
    std::thread::sleep(Duration::from_millis(delay_ms));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::exe_iter;
    use std::sync::{Arc, Mutex};

    #[test]
    fn exe_iter_runs_callback() {
        let counter = Arc::new(Mutex::new(0usize));
        let counter_ref = Arc::clone(&counter);
        let task = exe_iter(
            move || {
                let counter_ref = Arc::clone(&counter_ref);
                async move {
                    let mut guard = counter_ref.lock().expect("lock");
                    *guard += 1;
                }
            },
            3,
            0,
        );
        futures::executor::block_on(task).expect("exe_iter");
        assert_eq!(*counter.lock().expect("lock"), 3);
    }
}
