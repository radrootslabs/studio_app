use ui_primitives_core::roving_focus::RovingFocus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppUiRovingFocusOrientation {
    Horizontal,
    Vertical,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppUiRovingFocusAction {
    Next,
    Prev,
    First,
    Last,
}

pub fn radroots_studio_app_ui_roving_focus_action_from_key(
    key: &str,
    orientation: RadrootsAppUiRovingFocusOrientation,
) -> Option<RadrootsAppUiRovingFocusAction> {
    match key {
        "Home" => Some(RadrootsAppUiRovingFocusAction::First),
        "End" => Some(RadrootsAppUiRovingFocusAction::Last),
        "ArrowLeft" => matches!(
            orientation,
            RadrootsAppUiRovingFocusOrientation::Horizontal | RadrootsAppUiRovingFocusOrientation::Both
        )
        .then_some(RadrootsAppUiRovingFocusAction::Prev),
        "ArrowRight" => matches!(
            orientation,
            RadrootsAppUiRovingFocusOrientation::Horizontal | RadrootsAppUiRovingFocusOrientation::Both
        )
        .then_some(RadrootsAppUiRovingFocusAction::Next),
        "ArrowUp" => matches!(
            orientation,
            RadrootsAppUiRovingFocusOrientation::Vertical | RadrootsAppUiRovingFocusOrientation::Both
        )
        .then_some(RadrootsAppUiRovingFocusAction::Prev),
        "ArrowDown" => matches!(
            orientation,
            RadrootsAppUiRovingFocusOrientation::Vertical | RadrootsAppUiRovingFocusOrientation::Both
        )
        .then_some(RadrootsAppUiRovingFocusAction::Next),
        _ => None,
    }
}

pub fn radroots_studio_app_ui_roving_focus_next_index(
    current: usize,
    count: usize,
    action: RadrootsAppUiRovingFocusAction,
    looped: bool,
) -> usize {
    if count == 0 {
        return 0;
    }

    let mut focus = RovingFocus::with_active(count, Some(current), looped);
    match action {
        RadrootsAppUiRovingFocusAction::First => focus.move_first().unwrap_or(0),
        RadrootsAppUiRovingFocusAction::Last => focus.move_last().unwrap_or(0),
        RadrootsAppUiRovingFocusAction::Next => focus.move_next().unwrap_or(current),
        RadrootsAppUiRovingFocusAction::Prev => focus.move_prev().unwrap_or(current),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        radroots_studio_app_ui_roving_focus_action_from_key,
        radroots_studio_app_ui_roving_focus_next_index,
        RadrootsAppUiRovingFocusAction,
        RadrootsAppUiRovingFocusOrientation,
    };

    #[test]
    fn roving_focus_action_maps_arrows() {
        assert_eq!(
            radroots_studio_app_ui_roving_focus_action_from_key(
                "ArrowLeft",
                RadrootsAppUiRovingFocusOrientation::Horizontal
            ),
            Some(RadrootsAppUiRovingFocusAction::Prev)
        );
        assert_eq!(
            radroots_studio_app_ui_roving_focus_action_from_key(
                "ArrowUp",
                RadrootsAppUiRovingFocusOrientation::Horizontal
            ),
            None
        );
        assert_eq!(
            radroots_studio_app_ui_roving_focus_action_from_key(
                "ArrowDown",
                RadrootsAppUiRovingFocusOrientation::Both
            ),
            Some(RadrootsAppUiRovingFocusAction::Next)
        );
    }

    #[test]
    fn roving_focus_next_index_respects_loop() {
        assert_eq!(
            radroots_studio_app_ui_roving_focus_next_index(
                0,
                3,
                RadrootsAppUiRovingFocusAction::Prev,
                false
            ),
            0
        );
        assert_eq!(
            radroots_studio_app_ui_roving_focus_next_index(
                0,
                3,
                RadrootsAppUiRovingFocusAction::Prev,
                true
            ),
            2
        );
        assert_eq!(
            radroots_studio_app_ui_roving_focus_next_index(
                2,
                3,
                RadrootsAppUiRovingFocusAction::Next,
                true
            ),
            0
        );
    }
}
