use chrono::Utc;
use uuid::Uuid;

pub fn time_created_on() -> String {
    let now: chrono::DateTime<Utc> = Utc::now();
    now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn uuidv4() -> String {
    Uuid::new_v4().to_string()
}
