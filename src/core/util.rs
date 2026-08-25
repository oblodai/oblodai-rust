//! Small pure helpers: constant-time comparison, case-insensitive header lookup and an
//! HTTP-date parser (the `Date` and `Retry-After` headers, without pulling in a date library).

use subtle::ConstantTimeEq;

/// Constant-time equality. Both sides are hex here, so byte length equals character length and
/// leaking the length leaks nothing.
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

/// Case-insensitive lookup over a header list.
pub fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// Parse an IMF-fixdate / RFC 850 / asctime HTTP date into unix seconds.
///
/// Only the forms an HTTP server may legally send are accepted; anything else returns `None`, and
/// the caller then ignores the header rather than acting on a misread time.
pub fn parse_http_date(value: &str) -> Option<i64> {
    let v = value.trim();
    // Drop the weekday: "Sun, " (IMF-fixdate / RFC 850) or "Sun " (asctime).
    let rest = match v.find(", ") {
        Some(i) => &v[i + 2..],
        None => v
            .split_once(' ')
            .map(|(_, tail)| tail.trim_start())
            .unwrap_or(v),
    };
    let mut tokens = rest.split_whitespace();
    let first = tokens.next()?;

    let (day, month, year, time) = if let Some(month) = month_index(first) {
        // asctime: "Nov  6 08:49:37 1994"
        let day = tokens.next()?;
        let time = tokens.next()?;
        let year: i64 = tokens.next()?.parse().ok()?;
        (day.to_string(), month, year, time.to_string())
    } else if first.len() == 9 && first.as_bytes()[2] == b'-' {
        // RFC 850: "06-Nov-94 08:49:37 GMT"
        let time = tokens.next()?;
        let mut d = first.split('-');
        let day = d.next()?.to_string();
        let month = month_index(d.next()?)?;
        let two_digit: i64 = d.next()?.parse().ok()?;
        let year = if two_digit < 70 {
            two_digit + 2000
        } else {
            two_digit + 1900
        };
        (day, month, year, time.to_string())
    } else {
        // IMF-fixdate: "06 Nov 1994 08:49:37 GMT"
        let month = month_index(tokens.next()?)?;
        let year: i64 = tokens.next()?.parse().ok()?;
        let time = tokens.next()?;
        (first.to_string(), month, year, time.to_string())
    };

    let day: i64 = day.parse().ok()?;
    let mut t = time.split(':');
    let hour: i64 = t.next()?.parse().ok()?;
    let minute: i64 = t.next()?.parse().ok()?;
    let second: i64 = t.next()?.parse().ok()?;
    if !(1..=31).contains(&day) || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second)
}

fn month_index(name: &str) -> Option<i64> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    MONTHS
        .iter()
        .position(|m| m.eq_ignore_ascii_case(name))
        .map(|i| i as i64 + 1)
}

/// Howard Hinnant's days-from-civil: days since 1970-01-01 for a proleptic Gregorian date.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Current unix time in seconds.
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
