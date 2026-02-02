#![forbid(unsafe_code)]

use leptos::prelude::{use_context, LocalStorage, RwSignal, StoredValue, WithUntracked, WithValue};

use mf2_i18n_core::Args;
use mf2_i18n_embedded::EmbeddedRuntime;
use radroots_studio_app_lib::get_locale;

#[derive(Clone, Copy)]
pub struct RadrootsAppI18nContext {
    pub locale: RwSignal<String, LocalStorage>,
    pub runtime: StoredValue<Option<EmbeddedRuntime>>,
}

pub fn app_i18n_init() -> RadrootsAppI18nContext {
    let locale = get_locale(&["en"]);
    let locale = RwSignal::new_local(locale);
    let runtime = StoredValue::new(None::<EmbeddedRuntime>);
    RadrootsAppI18nContext { locale, runtime }
}

pub fn app_i18n() -> Option<RadrootsAppI18nContext> {
    use_context::<RadrootsAppI18nContext>()
}

pub fn translate(key: &str) -> String {
    let Some(ctx) = app_i18n() else {
        return key.to_string();
    };
    let locale = ctx.locale.with_untracked(|value| value.clone());
    ctx.runtime.with_value(|runtime: &Option<EmbeddedRuntime>| {
        if let Some(runtime) = runtime.as_ref() {
            let args = Args::new();
            runtime
                .format(&locale, key, &args)
                .unwrap_or_else(|_| key.to_string())
        } else {
            key.to_string()
        }
    })
}

#[macro_export]
macro_rules! t {
    ($key:literal) => {
        $crate::i18n::translate($key)
    };
}

#[cfg(test)]
mod tests {
    use super::{app_i18n, app_i18n_init, translate};
    use leptos::prelude::{provide_context, Owner};

    #[test]
    fn translate_falls_back_without_context() {
        assert_eq!(translate("hello"), "hello");
    }

    #[test]
    fn translate_reads_context() {
        let owner = Owner::new();
        owner.set();
        provide_context(app_i18n_init());
        assert!(app_i18n().is_some());
        assert_eq!(translate("hello"), "hello");
    }
}
