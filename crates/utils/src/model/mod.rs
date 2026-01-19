#![forbid(unsafe_code)]

use crate::types::ValidationRegex;

#[derive(Debug, Clone, PartialEq)]
pub enum ModelQueryValue {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModelQueryBindValue {
    String(String),
    Number(f64),
    Null,
}

pub type ModelQueryBindValueTuple = (String, ModelQueryValue);
pub type ModelQueryBindValueOpt = Option<ModelQueryBindValue>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelQueryFilterOption {
    Equals,
    StartsWith,
    EndsWith,
    Contains,
    NotEquals,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelQueryFilterOptionList {
    Between,
    In,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelQueryFilterCondition {
    And,
    Or,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSortCreatedAt {
    Newest,
    Oldest,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelQueryParam {
    pub query: String,
    pub bind_values: Vec<ModelQueryBindValue>,
}

pub type ModelFormErrorTuple = (bool, String);
pub type ModelFormValidationTuple = (String, String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSchemaErrors {
    pub err_s: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ModelForm {
    pub label: Option<String>,
    pub placeholder: Option<String>,
    pub validate_keypress: Option<bool>,
    pub prevent_focus_rest: Option<bool>,
    pub hidden: Option<bool>,
    pub optional: Option<bool>,
    pub default: Option<ModelQueryValue>,
    pub rxpv: ValidationRegex,
}

pub fn parse_model_query_value(value: &ModelQueryValue) -> ModelQueryBindValue {
    match value {
        ModelQueryValue::Bool(true) => ModelQueryBindValue::String("1".to_string()),
        ModelQueryValue::Bool(false) => ModelQueryBindValue::String("0".to_string()),
        ModelQueryValue::Number(value) => ModelQueryBindValue::Number(*value),
        ModelQueryValue::String(value) => {
            if value.is_empty() {
                ModelQueryBindValue::Null
            } else {
                ModelQueryBindValue::String(value.clone())
            }
        }
        ModelQueryValue::Null => ModelQueryBindValue::Null,
    }
}

pub fn is_model_query_filter_option(value: &str) -> bool {
    matches!(
        value,
        "equals" | "starts-with" | "ends-with" | "contains" | "ne"
    )
}

pub fn is_model_query_filter_option_list(value: &str) -> bool {
    matches!(value, "between" | "in")
}

pub fn is_model_query_values(value: &ModelQueryValue) -> bool {
    !matches!(value, ModelQueryValue::Null)
}

pub fn list_model_query_values_assert(values: &[Option<ModelQueryValue>]) -> Vec<ModelQueryValue> {
    values.iter().filter_map(|value| value.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::{
        is_model_query_filter_option, is_model_query_filter_option_list, is_model_query_values,
        list_model_query_values_assert, parse_model_query_value, ModelQueryBindValue,
        ModelQueryValue,
    };

    #[test]
    fn parse_model_query_value_handles_bool() {
        assert_eq!(
            parse_model_query_value(&ModelQueryValue::Bool(true)),
            ModelQueryBindValue::String("1".to_string())
        );
        assert_eq!(
            parse_model_query_value(&ModelQueryValue::Bool(false)),
            ModelQueryBindValue::String("0".to_string())
        );
    }

    #[test]
    fn parse_model_query_value_handles_string() {
        assert_eq!(
            parse_model_query_value(&ModelQueryValue::String("ok".to_string())),
            ModelQueryBindValue::String("ok".to_string())
        );
        assert_eq!(
            parse_model_query_value(&ModelQueryValue::String(String::new())),
            ModelQueryBindValue::Null
        );
    }

    #[test]
    fn filter_option_checks() {
        assert!(is_model_query_filter_option("equals"));
        assert!(!is_model_query_filter_option("other"));
        assert!(is_model_query_filter_option_list("between"));
        assert!(!is_model_query_filter_option_list("other"));
    }

    #[test]
    fn query_value_checks() {
        assert!(is_model_query_values(&ModelQueryValue::String("ok".to_string())));
        assert!(!is_model_query_values(&ModelQueryValue::Null));
    }

    #[test]
    fn list_model_query_values_filters_none() {
        let values = vec![
            Some(ModelQueryValue::String("a".to_string())),
            None,
            Some(ModelQueryValue::Number(1.0)),
        ];
        let filtered = list_model_query_values_assert(&values);
        assert_eq!(
            filtered,
            vec![
                ModelQueryValue::String("a".to_string()),
                ModelQueryValue::Number(1.0)
            ]
        );
    }
}
