pub fn radroots_studio_app_ui_compose_event_handlers<T, A, B, P>(
    mut first: Option<A>,
    mut second: Option<B>,
    mut is_prevented: P,
) -> impl FnMut(&T)
where
    A: FnMut(&T),
    B: FnMut(&T),
    P: FnMut(&T) -> bool,
{
    move |event| {
        if let Some(handler) = first.as_mut() {
            handler(event);
        }
        if is_prevented(event) {
            return;
        }
        if let Some(handler) = second.as_mut() {
            handler(event);
        }
    }
}

pub fn radroots_studio_app_ui_compose_event_handlers_unchecked<T, A, B>(
    first: Option<A>,
    second: Option<B>,
) -> impl FnMut(&T)
where
    A: FnMut(&T),
    B: FnMut(&T),
{
    radroots_studio_app_ui_compose_event_handlers(first, second, |_| false)
}

#[cfg(test)]
mod tests {
    use super::{
        radroots_studio_app_ui_compose_event_handlers,
        radroots_studio_app_ui_compose_event_handlers_unchecked,
    };

    #[derive(Default)]
    struct TestEvent {
        calls: core::cell::Cell<usize>,
        prevented: core::cell::Cell<bool>,
    }

    impl TestEvent {
        fn mark_called(&self) {
            self.calls.set(self.calls.get() + 1);
        }

        fn prevent(&self) {
            self.prevented.set(true);
        }
    }

    #[test]
    fn compose_calls_handlers_in_order() {
        let event = TestEvent::default();
        let mut handler = radroots_studio_app_ui_compose_event_handlers_unchecked(
            Some(|evt: &TestEvent| evt.mark_called()),
            Some(|evt: &TestEvent| evt.mark_called()),
        );
        handler(&event);
        assert_eq!(event.calls.get(), 2);
    }

    #[test]
    fn compose_skips_second_when_prevented() {
        let event = TestEvent::default();
        let mut handler = radroots_studio_app_ui_compose_event_handlers(
            Some(|evt: &TestEvent| {
                evt.mark_called();
                evt.prevent();
            }),
            Some(|evt: &TestEvent| evt.mark_called()),
            |evt: &TestEvent| evt.prevented.get(),
        );
        handler(&event);
        assert_eq!(event.calls.get(), 1);
    }
}
