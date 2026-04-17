macro_rules! define_app_text_keys {
    ($($variant:ident => $id:literal,)+) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum AppTextKey {
            $($variant,)+
        }

        impl AppTextKey {
            pub const ALL: &'static [Self] = &[
                $(Self::$variant,)+
            ];

            pub const fn id(self) -> &'static str {
                match self {
                    $(Self::$variant => $id,)+
                }
            }
        }
    };
}

define_app_text_keys! {
    AppName => "app.name",
    HomeBrand => "home.brand",
    HomeTitle => "home.title",
    SettingsTitle => "settings.title",
}
