use alloc::string::{String, ToString};
use core::sync::atomic::{AtomicUsize, Ordering};

static RADROOTS_APP_UI_ID_SEQ: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RadrootsAppUiId {
    value: usize,
}

impl RadrootsAppUiId {
    pub fn next() -> Self {
        let value = RADROOTS_APP_UI_ID_SEQ.fetch_add(1, Ordering::Relaxed);
        Self { value }
    }

    pub const fn value(self) -> usize {
        self.value
    }

    pub fn prefixed(self, prefix: &str) -> String {
        let mut out = String::with_capacity(prefix.len() + 1 + 20);
        out.push_str(prefix);
        out.push('-');
        out.push_str(self.value.to_string().as_str());
        out
    }
}

#[derive(Debug, Default, Clone)]
pub struct RadrootsAppUiIdSequence {
    next: usize,
}

impl RadrootsAppUiIdSequence {
    pub const fn new() -> Self {
        Self { next: 0 }
    }

    pub fn next(&mut self) -> RadrootsAppUiId {
        let value = self.next;
        self.next = self.next.saturating_add(1);
        RadrootsAppUiId { value }
    }

    pub const fn peek(&self) -> usize {
        self.next
    }
}

#[cfg(test)]
mod tests {
    use super::{RadrootsAppUiId, RadrootsAppUiIdSequence};

    #[test]
    fn id_sequence_increments() {
        let first = RadrootsAppUiId::next().value();
        let second = RadrootsAppUiId::next().value();
        assert!(second > first);
    }

    #[test]
    fn id_prefix_builds_value() {
        let id = RadrootsAppUiId { value: 7 };
        assert_eq!(id.prefixed("radroots"), "radroots-7");
    }

    #[test]
    fn id_sequence_local_increments() {
        let mut seq = RadrootsAppUiIdSequence::new();
        let first = seq.next();
        let second = seq.next();
        assert_eq!(first.value(), 0);
        assert_eq!(second.value(), 1);
    }

    #[test]
    fn id_sequence_peek_is_next_value() {
        let seq = RadrootsAppUiIdSequence::new();
        assert_eq!(seq.peek(), 0);
    }
}
