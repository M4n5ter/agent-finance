use chrono::{DateTime, SecondsFormat, Utc};
use chrono_tz::Tz;

pub const DEFAULT_TIMEZONE: &str = "Asia/Singapore";

pub fn utc_to_local(value: Option<&str>, timezone: &str) -> Option<String> {
    let value = value?;
    let datetime = DateTime::parse_from_rfc3339(value).ok()?;
    Some(format_local(datetime.with_timezone(&Utc), timezone))
}

pub fn now_local(timezone: &str) -> String {
    format_local(Utc::now(), timezone)
}

pub fn format_local(datetime: DateTime<Utc>, timezone: &str) -> String {
    let timezone = timezone.parse::<Tz>().unwrap_or(chrono_tz::Asia::Singapore);
    datetime
        .with_timezone(&timezone)
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}
