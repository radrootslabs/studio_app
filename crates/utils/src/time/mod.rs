#![forbid(unsafe_code)]

use std::time::{SystemTime, UNIX_EPOCH};

pub fn time_now_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub fn time_now_ms() -> u64 {
    time_now_s()
}

#[cfg(test)]
mod tests {
    use super::{time_now_ms, time_now_s};

    #[test]
    fn time_now_returns_seconds() {
        let now_s = time_now_s();
        let now_ms = time_now_ms();
        let delta = if now_s > now_ms {
            now_s - now_ms
        } else {
            now_ms - now_s
        };
        assert!(delta <= 1);
    }
}
