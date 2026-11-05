//! Exact calendar-day searches share the timeline's retained local date.
use chrono::{Datelike, NaiveDate};

pub(super) fn day(value: &str) -> Option<String> {
    let value = value.trim();
    let bytes = value.as_bytes();
    if bytes.len() != 10 {
        return None;
    }
    let format = if bytes[2] == b'-' && bytes[5] == b'-' {
        "%d-%m-%Y"
    } else if bytes[4] == b'-' && bytes[7] == b'-' {
        "%Y-%m-%d"
    } else {
        return None;
    };
    if !bytes
        .iter()
        .all(|byte| byte.is_ascii_digit() || *byte == b'-')
    {
        return None;
    }
    let date = NaiveDate::parse_from_str(value, format).ok()?;
    (date.year() > 0).then(|| date.format("%Y-%m-%d").to_string())
}

#[cfg(test)]
mod tests;
