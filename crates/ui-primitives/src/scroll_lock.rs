use std::sync::{Mutex, OnceLock};

#[cfg(target_arch = "wasm32")]
use web_sys::{window, CssStyleDeclaration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadrootsAppUiScrollLockError {
    WindowUnavailable,
    DocumentUnavailable,
    BodyUnavailable,
    StyleUnavailable,
}

pub type RadrootsAppUiScrollLockResult<T> = Result<T, RadrootsAppUiScrollLockError>;

#[derive(Debug)]
pub struct RadrootsAppUiScrollLockGuard {
    active: bool,
}

impl Drop for RadrootsAppUiScrollLockGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = radroots_studio_app_ui_scroll_lock_release();
            self.active = false;
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Debug, Default, Clone)]
struct ScrollLockSnapshot {
    scroll_y: f64,
    overflow: String,
    position: String,
    top: String,
    width: String,
}

#[derive(Debug, Default)]
struct ScrollLockState {
    count: usize,
    snapshot: Option<ScrollLockSnapshot>,
}

static SCROLL_LOCK_STATE: OnceLock<Mutex<ScrollLockState>> = OnceLock::new();

fn scroll_lock_state() -> &'static Mutex<ScrollLockState> {
    SCROLL_LOCK_STATE.get_or_init(|| Mutex::new(ScrollLockState::default()))
}

pub fn radroots_studio_app_ui_scroll_lock_acquire() -> RadrootsAppUiScrollLockResult<RadrootsAppUiScrollLockGuard> {
    let mut state = scroll_lock_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if state.count == 0 {
        scroll_lock_apply(&mut state)?;
    }
    state.count += 1;
    Ok(RadrootsAppUiScrollLockGuard { active: true })
}

pub fn radroots_studio_app_ui_scroll_lock_release() -> RadrootsAppUiScrollLockResult<()> {
    let mut state = scroll_lock_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if state.count == 0 {
        return Ok(());
    }
    state.count = state.count.saturating_sub(1);
    if state.count == 0 {
        scroll_lock_restore(&mut state)?;
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn scroll_lock_apply(state: &mut ScrollLockState) -> RadrootsAppUiScrollLockResult<()> {
    let window = window().ok_or(RadrootsAppUiScrollLockError::WindowUnavailable)?;
    let document = window
        .document()
        .ok_or(RadrootsAppUiScrollLockError::DocumentUnavailable)?;
    let body = document
        .body()
        .ok_or(RadrootsAppUiScrollLockError::BodyUnavailable)?;
    let style = body.style();
    let scroll_y = window
        .scroll_y()
        .map_err(|_| RadrootsAppUiScrollLockError::StyleUnavailable)?;
    let snapshot = ScrollLockSnapshot {
        scroll_y,
        overflow: style_value(&style, "overflow")?,
        position: style_value(&style, "position")?,
        top: style_value(&style, "top")?,
        width: style_value(&style, "width")?,
    };
    style_set(&style, "overflow", "hidden")?;
    style_set(&style, "position", "fixed")?;
    style_set(&style, "width", "100%")?;
    style_set(&style, "top", &format!("-{}px", snapshot.scroll_y))?;
    state.snapshot = Some(snapshot);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn scroll_lock_apply(state: &mut ScrollLockState) -> RadrootsAppUiScrollLockResult<()> {
    state.snapshot = None;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn scroll_lock_restore(state: &mut ScrollLockState) -> RadrootsAppUiScrollLockResult<()> {
    let Some(snapshot) = state.snapshot.take() else {
        return Ok(());
    };
    let window = window().ok_or(RadrootsAppUiScrollLockError::WindowUnavailable)?;
    let document = window
        .document()
        .ok_or(RadrootsAppUiScrollLockError::DocumentUnavailable)?;
    let body = document
        .body()
        .ok_or(RadrootsAppUiScrollLockError::BodyUnavailable)?;
    let style = body.style();
    style_set(&style, "overflow", &snapshot.overflow)?;
    style_set(&style, "position", &snapshot.position)?;
    style_set(&style, "top", &snapshot.top)?;
    style_set(&style, "width", &snapshot.width)?;
    window.scroll_to_with_x_and_y(0.0, snapshot.scroll_y);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn scroll_lock_restore(state: &mut ScrollLockState) -> RadrootsAppUiScrollLockResult<()> {
    state.snapshot = None;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn style_set(
    style: &CssStyleDeclaration,
    name: &str,
    value: &str,
) -> RadrootsAppUiScrollLockResult<()> {
    style
        .set_property(name, value)
        .map_err(|_| RadrootsAppUiScrollLockError::StyleUnavailable)
}

#[cfg(target_arch = "wasm32")]
fn style_value(style: &CssStyleDeclaration, name: &str) -> RadrootsAppUiScrollLockResult<String> {
    style
        .get_property_value(name)
        .map_err(|_| RadrootsAppUiScrollLockError::StyleUnavailable)
}

#[cfg(test)]
fn scroll_lock_count_for_test() -> usize {
    let state = scroll_lock_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.count
}

#[cfg(test)]
mod tests {
    use super::{
        radroots_studio_app_ui_scroll_lock_acquire,
        radroots_studio_app_ui_scroll_lock_release,
        scroll_lock_count_for_test,
    };

    #[test]
    fn scroll_lock_guard_updates_count() {
        assert_eq!(scroll_lock_count_for_test(), 0);
        let guard = radroots_studio_app_ui_scroll_lock_acquire().expect("lock");
        assert_eq!(scroll_lock_count_for_test(), 1);
        drop(guard);
        assert_eq!(scroll_lock_count_for_test(), 0);
    }

    #[test]
    fn scroll_lock_release_is_idempotent() {
        let _ = radroots_studio_app_ui_scroll_lock_release().expect("release");
        assert_eq!(scroll_lock_count_for_test(), 0);
    }
}
