#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppTextKey {
    Brand,
}

pub fn app_text(key: AppTextKey) -> &'static str {
    match key {
        AppTextKey::Brand => "radroots",
    }
}
