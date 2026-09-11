//! The two timestamp formats the packet forwarder protocol prescribes.
//!
//! An `rxpk` object times a packet's reception in the ISO 8601 compact form with microsecond
//! precision, `2013-03-31T16:21:17.528002Z`, and a `stat` object times the gateway's own
//! clock in the expanded form `2014-01-12 08:59:28 GMT`. Both are UTC, and the protocol
//! carries both as strings, so these turn a count from the Unix epoch into one and read one
//! back. The calendar arithmetic is Howard Hinnant's `days_from_civil` and `civil_from_days`,
//! which are exact for every year the protocol can carry.
//!
//! # Examples
//!
//! ```
//! use pamoja_gateway::time;
//!
//! // The reception time of the protocol's own rxpk example.
//! assert_eq!(time::compact(1_364_746_877_528_002), "2013-03-31T16:21:17.528002Z");
//! assert_eq!(time::from_compact("2013-03-31T16:21:17.528002Z"), Some(1_364_746_877_528_002));
//!
//! // The gateway clock of its stat example.
//! assert_eq!(time::expanded(1_389_517_168), "2014-01-12 08:59:28 GMT");
//! ```

/// Seconds in a day, which no leap second reaches this arithmetic.
const DAY: u64 = 86_400;

/// Microseconds in a second.
const MICROS: u64 = 1_000_000;

/// Formats a reception time as an `rxpk` carries it: ISO 8601, compact, to the microsecond.
///
/// # Arguments
///
/// * `micros_since_epoch` - the time in microseconds since 1970-01-01 UTC.
///
/// # Returns
///
/// The timestamp, such as `2013-03-31T16:21:17.528002Z`.
pub fn compact(micros_since_epoch: u64) -> String {
    let seconds = micros_since_epoch / MICROS;
    let fraction = micros_since_epoch % MICROS;
    let (year, month, day, hour, minute, second) = civil(seconds);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{fraction:06}Z")
}

/// Formats a gateway's clock as a `stat` carries it: the expanded form, to the second.
///
/// # Arguments
///
/// * `seconds_since_epoch` - the time in seconds since 1970-01-01 UTC.
///
/// # Returns
///
/// The timestamp, such as `2014-01-12 08:59:28 GMT`.
pub fn expanded(seconds_since_epoch: u64) -> String {
    let (year, month, day, hour, minute, second) = civil(seconds_since_epoch);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} GMT")
}

/// Reads an expanded timestamp back, as a `stat` object carries one.
///
/// # Arguments
///
/// * `text` - the timestamp, such as `2014-01-12 08:59:28 GMT`.
///
/// # Returns
///
/// The time in seconds since the epoch, or `None` when the text is not that shape or names
/// no date on the calendar.
pub fn from_expanded(text: &str) -> Option<u64> {
    let body = text.strip_suffix(" GMT")?;
    let (date, clock) = body.split_once(' ')?;
    from_compact(&format!("{date}T{clock}Z")).map(|micros| micros / MICROS)
}

/// Reads a compact timestamp back.
///
/// The fraction may be absent or one to six digits, since a gateway that times to the
/// millisecond writes three.
///
/// # Arguments
///
/// * `text` - the timestamp, such as `2013-03-31T16:21:17.528002Z`.
///
/// # Returns
///
/// The time in microseconds since the epoch, or `None` when the text is not that shape or
/// names no date on the calendar.
pub fn from_compact(text: &str) -> Option<u64> {
    let body = text.strip_suffix('Z')?;
    let (date, rest) = body.split_once('T')?;
    let (clock, fraction) = match rest.split_once('.') {
        Some((clock, digits)) => (clock, micros_of(digits)?),
        None => (rest, 0),
    };

    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let mut clock = clock.split(':');
    let hour: u64 = clock.next()?.parse().ok()?;
    let minute: u64 = clock.next()?.parse().ok()?;
    let second: u64 = clock.next()?.parse().ok()?;
    if clock.next().is_some() || hour > 23 || minute > 59 || second > 60 {
        return None;
    }

    let days = days_from_civil(year, month, day);
    if days < 0 {
        return None;
    }
    let seconds = u64::try_from(days).ok()? * DAY + hour * 3_600 + minute * 60 + second;
    Some(seconds * MICROS + fraction)
}

/// Reads a fraction of a second, which the protocol writes with up to six digits.
fn micros_of(digits: &str) -> Option<u64> {
    if digits.is_empty() || digits.len() > 6 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let value: u64 = digits.parse().ok()?;
    Some(value * 10u64.pow(6 - digits.len() as u32))
}

/// Splits a count of seconds into its civil date and time of day.
fn civil(seconds_since_epoch: u64) -> (i64, u32, u32, u64, u64, u64) {
    let days = (seconds_since_epoch / DAY) as i64;
    let rest = seconds_since_epoch % DAY;
    let (year, month, day) = civil_from_days(days);
    (year, month, day, rest / 3_600, (rest / 60) % 60, rest % 60)
}

/// Returns the days from 1970-01-01 to a civil date, by Howard Hinnant's `days_from_civil`.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year =
        (153 * (i64::from(month) + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Returns the civil date a day count names, by Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted + 2) / 5 + 1) as u32;
    let month = (shifted + if shifted < 10 { 3 } else { -9 }) as u32;
    (year + i64::from(month <= 2), month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_protocols_own_examples_format_and_parse() {
        // The rxpk example of section 4, and the stat example beside it.
        assert_eq!(
            compact(1_364_746_877_528_002),
            "2013-03-31T16:21:17.528002Z"
        );
        assert_eq!(
            from_compact("2013-03-31T16:21:17.528002Z"),
            Some(1_364_746_877_528_002)
        );
        assert_eq!(expanded(1_389_517_168), "2014-01-12 08:59:28 GMT");
        assert_eq!(
            from_expanded("2014-01-12 08:59:28 GMT"),
            Some(1_389_517_168)
        );
        assert_eq!(from_expanded("2014-01-12T08:59:28Z"), None);
    }

    #[test]
    fn the_calendar_holds_at_its_edges() {
        assert_eq!(compact(0), "1970-01-01T00:00:00.000000Z");
        // A leap day in a year divisible by 400, and the last microsecond of a leap year.
        assert_eq!(compact(951_825_600_000_000), "2000-02-29T12:00:00.000000Z");
        assert_eq!(
            compact(1_735_689_599_999_999),
            "2024-12-31T23:59:59.999999Z"
        );
        // 2100 is divisible by 100 but not by 400, so February has 28 days.
        assert_eq!(
            compact(4_107_542_400_000_000),
            "2100-03-01T00:00:00.000000Z"
        );
    }

    #[test]
    fn a_shorter_fraction_is_read_as_written() {
        assert_eq!(
            from_compact("2013-03-31T16:21:17.528Z"),
            Some(1_364_746_877_528_000)
        );
        assert_eq!(
            from_compact("2013-03-31T16:21:17Z"),
            Some(1_364_746_877_000_000)
        );
    }

    #[test]
    fn every_timestamp_survives_a_round_trip() {
        let mut micros = 0u64;
        while micros < 4_200_000_000_000_000 {
            let text = compact(micros);
            assert_eq!(from_compact(&text), Some(micros), "{text}");
            micros += 97_777_777_777;
        }
    }

    #[test]
    fn what_is_not_a_timestamp_is_refused() {
        for text in [
            "2013-03-31T16:21:17.528002",
            "2013-03-31 16:21:17.528002Z",
            "2013-13-31T16:21:17.528002Z",
            "2013-03-31T24:21:17.528002Z",
            "2013-03-31T16:21:17.5280021Z",
            "1969-12-31T23:59:59.000000Z",
            "not a time",
        ] {
            assert_eq!(from_compact(text), None, "{text}");
        }
    }
}
