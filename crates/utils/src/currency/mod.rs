#![forbid(unsafe_code)]

use crate::numbers::parse_float;
use crate::validation::regex::UtilRegex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiatCurrency {
    Usd,
    Eur,
}

impl FiatCurrency {
    pub const fn as_str(self) -> &'static str {
        match self {
            FiatCurrency::Usd => "usd",
            FiatCurrency::Eur => "eur",
        }
    }

    pub const fn as_upper(self) -> &'static str {
        match self {
            FiatCurrency::Usd => "USD",
            FiatCurrency::Eur => "EUR",
        }
    }
}

pub const FIAT_CURRENCIES: [FiatCurrency; 2] = [FiatCurrency::Usd, FiatCurrency::Eur];

pub fn price_to_formatted(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub fn parse_currency(val: Option<&str>) -> FiatCurrency {
    match val.map(|value| value.trim().to_lowercase()) {
        Some(value) if value == "eur" => FiatCurrency::Eur,
        _ => FiatCurrency::Usd,
    }
}

pub fn fmt_price(locale: &str, value: &str, currency: &str) -> String {
    let value = parse_float(value, 0.0);
    let currency = parse_currency(Some(currency));
    fmt_price_value(locale, value, currency)
}

pub fn parse_currency_marker(locale: &str, currency: &str) -> String {
    let currency = parse_currency(Some(currency));
    let formatted = fmt_price_value(locale, 1.0, currency);
    if let Some(match_value) = UtilRegex::currency_marker().find(&formatted) {
        return match_value.as_str().to_string();
    }
    if let Some(match_value) = UtilRegex::currency_symbol().find(&formatted) {
        return match_value.as_str().to_string();
    }
    if let Some(match_value) = formatted
        .find(currency.as_upper())
        .map(|start| &formatted[start..start + currency.as_upper().len()])
    {
        return match_value.to_string();
    }
    currency.as_upper().to_string()
}

#[cfg(target_arch = "wasm32")]
fn fmt_price_value(locale: &str, value: f64, currency: FiatCurrency) -> String {
    use js_sys::{Array, Object, Reflect};
    use wasm_bindgen::JsValue;

    let locales = Array::new();
    locales.push(&JsValue::from_str(locale));
    let options = Object::new();
    let currency_upper = currency.as_upper();
    let _ = Reflect::set(&options, &JsValue::from_str("style"), &JsValue::from_str("currency"));
    let _ = Reflect::set(
        &options,
        &JsValue::from_str("currency"),
        &JsValue::from_str(currency_upper),
    );
    let _ = Reflect::set(
        &options,
        &JsValue::from_str("minimumFractionDigits"),
        &JsValue::from_f64(2.0),
    );
    let _ = Reflect::set(
        &options,
        &JsValue::from_str("maximumFractionDigits"),
        &JsValue::from_f64(2.0),
    );
    let formatter = js_sys::Intl::NumberFormat::new(&locales, &options);
    formatter.format(value).into()
}

#[cfg(not(target_arch = "wasm32"))]
fn fmt_price_value(_locale: &str, value: f64, currency: FiatCurrency) -> String {
    format!("{} {:.2}", currency.as_upper(), value)
}

#[cfg(test)]
mod tests {
    use super::{fmt_price, parse_currency, parse_currency_marker, price_to_formatted, FiatCurrency};

    #[test]
    fn price_to_formatted_rounds() {
        assert_eq!(price_to_formatted(1.234), 1.23);
    }

    #[test]
    fn parse_currency_defaults() {
        assert_eq!(parse_currency(Some("usd")), FiatCurrency::Usd);
        assert_eq!(parse_currency(Some("eur")), FiatCurrency::Eur);
        assert_eq!(parse_currency(None), FiatCurrency::Usd);
    }

    #[test]
    fn fmt_price_formats_value() {
        let formatted = fmt_price("en-US", "1.25", "usd");
        assert!(formatted.contains("USD"));
    }

    #[test]
    fn parse_currency_marker_returns_token() {
        let marker = parse_currency_marker("en-US", "usd");
        assert!(!marker.is_empty());
    }
}
