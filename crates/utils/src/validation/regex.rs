#![forbid(unsafe_code)]

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

pub struct UtilRegex;

macro_rules! regex_lazy {
    ($name:ident, $pattern:expr) => {
        static $name: Lazy<Regex> = Lazy::new(|| {
            Regex::new($pattern).expect("regex")
        });
    };
}

regex_lazy!(EMAIL, r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$");
regex_lazy!(EMAIL_CH, r"^[A-Za-z0-9._%+@-]*$");
regex_lazy!(PRODUCT_KEY, r"^[A-Za-z_]+$");
regex_lazy!(PRODUCT_KEY_CH, r"^[A-Za-z_]$");
regex_lazy!(PRODUCT_TITLE, r"[A-Za-z0-9 ]+$");
regex_lazy!(PRODUCT_TITLE_CH, r"[A-Za-z0-9 ]$");
regex_lazy!(FLOAT, r"^[+-]?(\d+(\.\d*)?|\.\d+)$");
regex_lazy!(FLOAT_CH, r"^[0-9.+-]$");
regex_lazy!(FLOAT_POS, r"^\d+(\.\d+)?$");
regex_lazy!(FLOAT_POS_CH, r"^[0-9.]$");
regex_lazy!(DESCRIPTION, r"^(?:\S+(?:\s+\S+)*)$");
regex_lazy!(DESCRIPTION_CH, r#"[^a-zA-Z0-9.,!?;:'"(){}\[\]\s\x{0600}-\x{06FF}\x{0900}-\x{097F}\x{0400}-\x{04FF}\x{0500}-\x{052F}\x{1F00}-\x{1FFF}\x{4E00}-\x{9FFF}\x{AC00}-\x{D7AF}\x{3040}-\x{309F}\x{30A0}-\x{30FF} ]+"#);
regex_lazy!(NBSP, r"[\x{00A0}]");
regex_lazy!(NBSP_RP, r"[\x{00A0}]+");
regex_lazy!(RTLM, r"[\x{200F}]");
regex_lazy!(RTLM_RP, r"[\x{200F}]+");
regex_lazy!(COMMAS, r"[,]+");
regex_lazy!(PERIODS, r"[.]+");
regex_lazy!(WORD_ONLY, r"^[a-zA-Z]+$");
regex_lazy!(ALPHA, r"[a-zA-Z ]$");
regex_lazy!(ALPHA_CH, r"[a-zA-Z ]$");
regex_lazy!(NUM, r"^[0-9]+$");
regex_lazy!(LAT, r"^[-+]?([1-8]?[0-9](\.\d{1,6})?|90(\.0{1,6})?)$");
regex_lazy!(LAT_CH, r"^[0-9.+-]$");
regex_lazy!(LNG, r"^[-+]?((1[0-7]?[0-9]|180)(\.\d{1,6})?|(\d{1,2})(\.\d{1,6})?)$");
regex_lazy!(LNG_CH, r"^[0-9.+-]$");
regex_lazy!(ALPHANUM, r"[a-zA-Z0-9., ]$");
regex_lazy!(ALPHANUM_CH, r"[a-zA-Z0-9.,\s\x{0600}-\x{06FF}\x{0900}-\x{097F}\x{0400}-\x{04FF}\x{0500}-\x{052F}\x{1F00}-\x{1FFF}\x{4E00}-\x{9FFF}\x{AC00}-\x{D7AF}\x{3040}-\x{309F}\x{30A0}-\x{30FF} ]+");
regex_lazy!(PRICE, r"^\d+(\.\d+)?$");
regex_lazy!(PRICE_CH, r"[0-9.]$");
regex_lazy!(PRICE_CUR, r"^[A-Za-z]{3}$");
regex_lazy!(PRICE_CUR_CH, r"[A-Za-z]$");
regex_lazy!(PROFILE_NAME, r"^[a-zA-Z0-9._]{3,30}$");
regex_lazy!(PROFILE_NAME_CH, r"[a-zA-Z0-9._]");
regex_lazy!(TRADE_PRODUCT_KEY, r"^(?:[a-zA-Z0-9]+(?:\s+[a-zA-Z0-9]+){0,2})$");
regex_lazy!(TRADE_PRODUCT_CATEGORY, r"^(?:[a-zA-Z0-9]+(?:\s+[a-zA-Z0-9]+){0,2})$");
regex_lazy!(CURRENCY_SYMBOL, r"(?:[A-Za-z]{3,5}\$|\p{Sc})");
regex_lazy!(CURRENCY_MARKER, r"(?:[A-Za-z]{2,4}[^\d\s]+|[^\d\s]{1,3}[A-Za-z]{2,4})");
regex_lazy!(WS_PROTO, r"^(wss://|ws://)");
regex_lazy!(BIN_DISPLAY_UNIT, r"^(kg|lb|g)$");
regex_lazy!(BIN_DISPLAY_UNIT_CH, r"[A-Za-z]$");
regex_lazy!(URL_IMAGE_UPLOAD, r"^file://.*\.(png|jpg|jpeg|gif|webp|bmp|svg)$");
regex_lazy!(URL_IMAGE_UPLOAD_DEV, r"^file://.*\.(png|jpg|jpeg|gif|webp|bmp|svg)$");
regex_lazy!(COUNTRY_CODE_A2, r"^[A-Za-z]{2}$");
regex_lazy!(ADDR_PRIMARY, r"[a-zA-Z0-9., ]$");
regex_lazy!(ADDR_ADMIN, r"[a-zA-Z0-9., ]$");
regex_lazy!(NUM_INT, r"^[0-9]$");
regex_lazy!(AREA_UNIT, r"^(ac|ha|ft2|m2)$");
regex_lazy!(AREA_UNIT_CH, r"[A-Za-z2]$");

impl UtilRegex {
    pub fn email() -> &'static Regex {
        &EMAIL
    }

    pub fn email_ch() -> &'static Regex {
        &EMAIL_CH
    }

    pub fn product_key() -> &'static Regex {
        &PRODUCT_KEY
    }

    pub fn product_key_ch() -> &'static Regex {
        &PRODUCT_KEY_CH
    }

    pub fn product_title() -> &'static Regex {
        &PRODUCT_TITLE
    }

    pub fn product_title_ch() -> &'static Regex {
        &PRODUCT_TITLE_CH
    }

    pub fn float() -> &'static Regex {
        &FLOAT
    }

    pub fn float_ch() -> &'static Regex {
        &FLOAT_CH
    }

    pub fn float_pos() -> &'static Regex {
        &FLOAT_POS
    }

    pub fn float_pos_ch() -> &'static Regex {
        &FLOAT_POS_CH
    }

    pub fn description() -> &'static Regex {
        &DESCRIPTION
    }

    pub fn description_ch() -> &'static Regex {
        &DESCRIPTION_CH
    }

    pub fn nbsp() -> &'static Regex {
        &NBSP
    }

    pub fn nbsp_rp() -> &'static Regex {
        &NBSP_RP
    }

    pub fn rtlm() -> &'static Regex {
        &RTLM
    }

    pub fn rtlm_rp() -> &'static Regex {
        &RTLM_RP
    }

    pub fn commas() -> &'static Regex {
        &COMMAS
    }

    pub fn periods() -> &'static Regex {
        &PERIODS
    }

    pub fn word_only() -> &'static Regex {
        &WORD_ONLY
    }

    pub fn alpha() -> &'static Regex {
        &ALPHA
    }

    pub fn alpha_ch() -> &'static Regex {
        &ALPHA_CH
    }

    pub fn num() -> &'static Regex {
        &NUM
    }

    pub fn lat() -> &'static Regex {
        &LAT
    }

    pub fn lat_ch() -> &'static Regex {
        &LAT_CH
    }

    pub fn lng() -> &'static Regex {
        &LNG
    }

    pub fn lng_ch() -> &'static Regex {
        &LNG_CH
    }

    pub fn alphanum() -> &'static Regex {
        &ALPHANUM
    }

    pub fn alphanum_ch() -> &'static Regex {
        &ALPHANUM_CH
    }

    pub fn price() -> &'static Regex {
        &PRICE
    }

    pub fn price_ch() -> &'static Regex {
        &PRICE_CH
    }

    pub fn price_cur() -> &'static Regex {
        &PRICE_CUR
    }

    pub fn price_cur_ch() -> &'static Regex {
        &PRICE_CUR_CH
    }

    pub fn profile_name() -> &'static Regex {
        &PROFILE_NAME
    }

    pub fn profile_name_ch() -> &'static Regex {
        &PROFILE_NAME_CH
    }

    pub fn trade_product_key() -> &'static Regex {
        &TRADE_PRODUCT_KEY
    }

    pub fn trade_product_category() -> &'static Regex {
        &TRADE_PRODUCT_CATEGORY
    }

    pub fn currency_symbol() -> &'static Regex {
        &CURRENCY_SYMBOL
    }

    pub fn currency_marker() -> &'static Regex {
        &CURRENCY_MARKER
    }

    pub fn ws_proto() -> &'static Regex {
        &WS_PROTO
    }

    pub fn bin_display_unit() -> &'static Regex {
        &BIN_DISPLAY_UNIT
    }

    pub fn bin_display_unit_ch() -> &'static Regex {
        &BIN_DISPLAY_UNIT_CH
    }

    pub fn url_image_upload() -> &'static Regex {
        &URL_IMAGE_UPLOAD
    }

    pub fn url_image_upload_dev() -> &'static Regex {
        &URL_IMAGE_UPLOAD_DEV
    }

    pub fn country_code_a2() -> &'static Regex {
        &COUNTRY_CODE_A2
    }

    pub fn addr_primary() -> &'static Regex {
        &ADDR_PRIMARY
    }

    pub fn addr_admin() -> &'static Regex {
        &ADDR_ADMIN
    }

    pub fn num_int() -> &'static Regex {
        &NUM_INT
    }

    pub fn area_unit() -> &'static Regex {
        &AREA_UNIT
    }

    pub fn area_unit_ch() -> &'static Regex {
        &AREA_UNIT_CH
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormFieldsKey {
    NostrSecretKey,
    ProductTitle,
    ProductKey,
    ProductProcess,
    ProductDescription,
    Price,
    PriceCurrency,
    BinDisplayUnit,
    BinDisplayAmount,
    BinLabel,
    FarmName,
    FarmSize,
    Area,
    AreaUnit,
    ContactName,
    ProfileName,
}

impl FormFieldsKey {
    pub const fn as_str(self) -> &'static str {
        match self {
            FormFieldsKey::NostrSecretKey => "nostr_secret_key",
            FormFieldsKey::ProductTitle => "product_title",
            FormFieldsKey::ProductKey => "product_key",
            FormFieldsKey::ProductProcess => "product_process",
            FormFieldsKey::ProductDescription => "product_description",
            FormFieldsKey::Price => "price",
            FormFieldsKey::PriceCurrency => "price_currency",
            FormFieldsKey::BinDisplayUnit => "bin_display_unit",
            FormFieldsKey::BinDisplayAmount => "bin_display_amount",
            FormFieldsKey::BinLabel => "bin_label",
            FormFieldsKey::FarmName => "farm_name",
            FormFieldsKey::FarmSize => "farm_size",
            FormFieldsKey::Area => "area",
            FormFieldsKey::AreaUnit => "area_unit",
            FormFieldsKey::ContactName => "contact_name",
            FormFieldsKey::ProfileName => "profile_name",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FormField {
    pub validate: &'static Regex,
    pub charset: &'static Regex,
}

static FORM_FIELDS: Lazy<HashMap<FormFieldsKey, FormField>> = Lazy::new(|| {
    let mut fields = HashMap::new();
    fields.insert(
        FormFieldsKey::ProfileName,
        FormField {
            validate: UtilRegex::profile_name(),
            charset: UtilRegex::profile_name_ch(),
        },
    );
    fields.insert(
        FormFieldsKey::ProductDescription,
        FormField {
            validate: UtilRegex::alpha(),
            charset: UtilRegex::alpha_ch(),
        },
    );
    fields.insert(
        FormFieldsKey::ProductKey,
        FormField {
            validate: UtilRegex::product_key(),
            charset: UtilRegex::product_key_ch(),
        },
    );
    fields.insert(
        FormFieldsKey::ProductTitle,
        FormField {
            validate: UtilRegex::product_title(),
            charset: UtilRegex::product_title_ch(),
        },
    );
    fields.insert(
        FormFieldsKey::ProductProcess,
        FormField {
            validate: UtilRegex::alphanum(),
            charset: UtilRegex::alphanum_ch(),
        },
    );
    fields.insert(
        FormFieldsKey::Price,
        FormField {
            validate: UtilRegex::price(),
            charset: UtilRegex::price_ch(),
        },
    );
    fields.insert(
        FormFieldsKey::PriceCurrency,
        FormField {
            validate: UtilRegex::price_cur(),
            charset: UtilRegex::price_cur_ch(),
        },
    );
    fields.insert(
        FormFieldsKey::BinDisplayAmount,
        FormField {
            validate: UtilRegex::float_pos(),
            charset: UtilRegex::float_pos_ch(),
        },
    );
    fields.insert(
        FormFieldsKey::BinDisplayUnit,
        FormField {
            validate: UtilRegex::bin_display_unit(),
            charset: UtilRegex::bin_display_unit_ch(),
        },
    );
    fields.insert(
        FormFieldsKey::BinLabel,
        FormField {
            validate: UtilRegex::alphanum(),
            charset: UtilRegex::alphanum_ch(),
        },
    );
    fields.insert(
        FormFieldsKey::Area,
        FormField {
            validate: UtilRegex::float(),
            charset: UtilRegex::float_ch(),
        },
    );
    fields.insert(
        FormFieldsKey::AreaUnit,
        FormField {
            validate: UtilRegex::area_unit(),
            charset: UtilRegex::area_unit_ch(),
        },
    );
    fields.insert(
        FormFieldsKey::FarmName,
        FormField {
            validate: UtilRegex::alpha(),
            charset: UtilRegex::alpha_ch(),
        },
    );
    fields.insert(
        FormFieldsKey::FarmSize,
        FormField {
            validate: UtilRegex::num_int(),
            charset: UtilRegex::num_int(),
        },
    );
    fields.insert(
        FormFieldsKey::ContactName,
        FormField {
            validate: UtilRegex::alpha(),
            charset: UtilRegex::alpha_ch(),
        },
    );
    fields.insert(
        FormFieldsKey::NostrSecretKey,
        FormField {
            validate: UtilRegex::alpha(),
            charset: UtilRegex::alpha_ch(),
        },
    );
    fields
});

pub fn form_fields() -> &'static HashMap<FormFieldsKey, FormField> {
    &FORM_FIELDS
}

#[cfg(test)]
mod tests {
    use super::{form_fields, FormFieldsKey, UtilRegex};

    #[test]
    fn email_regex_accepts_valid_email() {
        assert!(UtilRegex::email().is_match("user@example.com"));
        assert!(!UtilRegex::email().is_match("invalid"));
    }

    #[test]
    fn form_fields_contains_profile_name() {
        let fields = form_fields();
        assert!(fields.contains_key(&FormFieldsKey::ProfileName));
    }
}
