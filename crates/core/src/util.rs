use chrono::Utc;
use uuid::Uuid;

pub fn time_created_on() -> String {
    let now: chrono::DateTime<Utc> = Utc::now();
    now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn uuidv4() -> String {
    Uuid::new_v4().to_string()
}

pub fn json_values(obj: &serde_json::Value) -> Vec<serde_json::Value> {
    if let serde_json::Value::Object(map) = obj {
        map.values().cloned().collect()
    } else {
        Vec::new()
    }
}

pub fn json_values_str(obj: &serde_json::Value) -> Vec<String> {
    if let serde_json::Value::Object(map) = obj {
        map.values()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    } else {
        Vec::new()
    }
}

pub fn json_keys(obj: &serde_json::Value) -> Vec<String> {
    if let serde_json::Value::Object(map) = obj {
        map.keys().cloned().collect()
    } else {
        Vec::new()
    }
}
