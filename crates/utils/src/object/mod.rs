#![forbid(unsafe_code)]

pub fn obj_en<K, V, F, I>(object: I, parse_function: F) -> Vec<(K, V)>
where
    I: IntoIterator<Item = (String, V)>,
    F: Fn(&str) -> K,
{
    object
        .into_iter()
        .map(|(key, value)| (parse_function(&key), value))
        .collect()
}

pub fn obj_truthy_fields<I, V>(values: I) -> bool
where
    I: IntoIterator<Item = V>,
    V: AsRef<str>,
{
    values.into_iter().all(|value| !value.as_ref().is_empty())
}

pub fn obj_result(value: &serde_json::Value) -> Option<String> {
    let obj = value.as_object()?;
    let result = obj.get("result")?.as_str()?;
    Some(result.to_string())
}

pub fn obj_results_str(value: &serde_json::Value) -> Option<Vec<String>> {
    let obj = value.as_object()?;
    let results = obj.get("results")?.as_array()?;
    Some(
        results
            .iter()
            .map(|entry| {
                entry
                    .as_str()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| entry.to_string())
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::{obj_en, obj_result, obj_results_str, obj_truthy_fields};
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn obj_en_maps_entries() {
        let mut map = BTreeMap::new();
        map.insert("one".to_string(), 1);
        let entries = obj_en(map.into_iter(), |key| format!("key:{key}"));
        assert_eq!(entries, vec![("key:one".to_string(), 1)]);
    }

    #[test]
    fn obj_truthy_fields_checks_values() {
        let values = vec!["one", "two"];
        assert!(obj_truthy_fields(values));
        let values = vec!["one", ""];
        assert!(!obj_truthy_fields(values));
    }

    #[test]
    fn obj_result_reads_result_field() {
        let value = json!({ "result": "ok" });
        assert_eq!(obj_result(&value), Some("ok".to_string()));
    }

    #[test]
    fn obj_results_str_reads_results_list() {
        let value = json!({ "results": ["a", "b"] });
        assert_eq!(
            obj_results_str(&value),
            Some(vec!["a".to_string(), "b".to_string()])
        );
    }
}
