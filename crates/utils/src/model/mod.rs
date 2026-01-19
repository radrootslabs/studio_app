#![forbid(unsafe_code)]

use crate::types::ValidationRegex;
use crate::error::RadrootsAppUtilsError;
use std::collections::BTreeMap;

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

#[derive(Debug, Clone, PartialEq)]
pub enum ModelQueryFilter {
    Value(ModelQueryValue),
    Single {
        value: ModelQueryValue,
        option: ModelQueryFilterOption,
        condition: Option<ModelQueryFilterCondition>,
    },
    List {
        values: Vec<ModelQueryValue>,
        option: ModelQueryFilterOptionList,
        condition: Option<ModelQueryFilterCondition>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelQueryFilterMapParsed {
    pub query_values: Vec<String>,
    pub bind_values: Vec<ModelQueryBindValue>,
}

pub type ModelQueryFilterMap = BTreeMap<String, ModelQueryFilter>;

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

pub fn parse_model_filter_map(
    filters: &ModelQueryFilterMap,
) -> Result<ModelQueryFilterMapParsed, RadrootsAppUtilsError> {
    let mut bind_values = Vec::new();
    let mut query_values = Vec::new();
    for (index, (field, filter)) in filters.iter().enumerate() {
        let filter_condition = if index == 0 {
            String::new()
        } else if let Some(condition) = filter_condition_for(filter) {
            format!("{} ", condition.as_str())
        } else {
            "AND ".to_string()
        };
        match filter {
            ModelQueryFilter::Value(value) => {
                query_values.push(format!("{filter_condition}{field} = ?"));
                bind_values.push(parse_model_query_value(value));
            }
            ModelQueryFilter::Single {
                value,
                option,
                ..
            } => match option {
                ModelQueryFilterOption::StartsWith => {
                    query_values.push(format!("{filter_condition}{field} LIKE ?"));
                    bind_values.push(ModelQueryBindValue::String(format!(
                        "{}%",
                        value_to_string(value)
                    )));
                }
                ModelQueryFilterOption::EndsWith => {
                    query_values.push(format!("{filter_condition}{field} LIKE ?"));
                    bind_values.push(ModelQueryBindValue::String(format!(
                        "%{}",
                        value_to_string(value)
                    )));
                }
                ModelQueryFilterOption::Contains => {
                    query_values.push(format!("{filter_condition}{field} LIKE ?"));
                    bind_values.push(ModelQueryBindValue::String(format!(
                        "%{}%",
                        value_to_string(value)
                    )));
                }
                ModelQueryFilterOption::NotEquals => {
                    query_values.push(format!("{filter_condition}{field} != ?"));
                    bind_values.push(parse_model_query_value(value));
                }
                ModelQueryFilterOption::Equals => {
                    query_values.push(format!("{filter_condition}{field} = ?"));
                    bind_values.push(parse_model_query_value(value));
                }
            },
            ModelQueryFilter::List {
                values,
                option,
                ..
            } => match option {
                ModelQueryFilterOptionList::Between => {
                    if values.len() < 2 {
                        return Err(RadrootsAppUtilsError::InvalidInput);
                    }
                    query_values.push(format!("{filter_condition}{field} BETWEEN ? AND ?"));
                    bind_values.push(parse_model_query_value(&values[0]));
                    bind_values.push(parse_model_query_value(&values[1]));
                }
                ModelQueryFilterOptionList::In => {
                    if values.is_empty() {
                        return Err(RadrootsAppUtilsError::InvalidInput);
                    }
                    let placeholders = std::iter::repeat("?")
                        .take(values.len())
                        .collect::<Vec<_>>()
                        .join(", ");
                    query_values.push(format!(
                        "{filter_condition}{field} IN ({placeholders})"
                    ));
                    for value in values {
                        bind_values.push(parse_model_query_value(value));
                    }
                }
            },
        }
    }
    if query_values.is_empty() || bind_values.is_empty() {
        return Err(RadrootsAppUtilsError::InvalidInput);
    }
    Ok(ModelQueryFilterMapParsed {
        query_values,
        bind_values,
    })
}

fn value_to_string(value: &ModelQueryValue) -> String {
    match value {
        ModelQueryValue::String(value) => value.clone(),
        ModelQueryValue::Number(value) => value.to_string(),
        ModelQueryValue::Bool(true) => "1".to_string(),
        ModelQueryValue::Bool(false) => "0".to_string(),
        ModelQueryValue::Null => String::new(),
    }
}

fn filter_condition_for(filter: &ModelQueryFilter) -> Option<ModelQueryFilterCondition> {
    match filter {
        ModelQueryFilter::Value(_) => None,
        ModelQueryFilter::Single { condition, .. } => *condition,
        ModelQueryFilter::List { condition, .. } => *condition,
    }
}

impl ModelQueryFilterCondition {
    pub const fn as_str(self) -> &'static str {
        match self {
            ModelQueryFilterCondition::And => "and",
            ModelQueryFilterCondition::Or => "or",
            ModelQueryFilterCondition::Not => "not",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        is_model_query_filter_option, is_model_query_filter_option_list, is_model_query_values,
        list_model_query_values_assert, parse_model_filter_map, parse_model_query_value,
        ModelQueryBindValue, ModelQueryFilter, ModelQueryFilterMap, ModelQueryFilterOption,
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

    #[test]
    fn parse_model_filter_map_builds_query() {
        let mut filters = ModelQueryFilterMap::new();
        filters.insert(
            "name".to_string(),
            ModelQueryFilter::Single {
                value: ModelQueryValue::String("rad".to_string()),
                option: ModelQueryFilterOption::Contains,
                condition: None,
            },
        );
        filters.insert(
            "status".to_string(),
            ModelQueryFilter::Value(ModelQueryValue::String("ok".to_string())),
        );
        let parsed = parse_model_filter_map(&filters).expect("parsed");
        assert_eq!(parsed.query_values.len(), 2);
        assert_eq!(parsed.bind_values.len(), 2);
    }
}
