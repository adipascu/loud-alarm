//! Pure scheduling helpers: given a wall-clock time-of-day, work out the next
//! moment it occurs. Kept free of side effects so it can be unit-tested.

use chrono::{DateTime, Duration, TimeZone, Timelike};

/// The next occurrence of `hour:minute` at or after `now`.
///
/// If the time has already passed today, it rolls over to tomorrow. If the
/// target is later today, it returns today.
pub fn next_occurrence<Tz: TimeZone>(now: DateTime<Tz>, hour: u32, minute: u32) -> DateTime<Tz> {
    let today = now
        .with_hour(hour)
        .and_then(|d| d.with_minute(minute))
        .and_then(|d| d.with_second(0))
        .and_then(|d| d.with_nanosecond(0))
        .expect("hour/minute already validated by caller");

    if today > now {
        today
    } else {
        today + Duration::days(1)
    }
}

/// Whole seconds from `now` until `target`, never negative.
pub fn seconds_until<Tz: TimeZone>(now: DateTime<Tz>, target: DateTime<Tz>) -> i64 {
    (target - now).num_seconds().max(0)
}

/// Validate a user-supplied time-of-day.
pub fn validate(hour: u32, minute: u32) -> Result<(), String> {
    if hour > 23 {
        return Err(format!("hour {hour} out of range (0-23)"));
    }
    if minute > 59 {
        return Err(format!("minute {minute} out of range (0-59)"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};

    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> DateTime<FixedOffset> {
        FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(y, mo, d, h, mi, s)
            .unwrap()
    }

    #[test]
    fn later_today_stays_today() {
        let now = at(2026, 6, 22, 13, 30, 0);
        let next = next_occurrence(now, 17, 0);
        assert_eq!(next, at(2026, 6, 22, 17, 0, 0));
    }

    #[test]
    fn earlier_today_rolls_to_tomorrow() {
        let now = at(2026, 6, 22, 18, 0, 0);
        let next = next_occurrence(now, 17, 0);
        assert_eq!(next, at(2026, 6, 23, 17, 0, 0));
    }

    #[test]
    fn exactly_now_rolls_to_tomorrow() {
        let now = at(2026, 6, 22, 17, 0, 0);
        let next = next_occurrence(now, 17, 0);
        assert_eq!(next, at(2026, 6, 23, 17, 0, 0));
    }

    #[test]
    fn seconds_until_is_correct_and_clamped() {
        let now = at(2026, 6, 22, 16, 59, 0);
        let target = at(2026, 6, 22, 17, 0, 0);
        assert_eq!(seconds_until(now, target), 60);
        assert_eq!(seconds_until(target, now), 0);
    }

    #[test]
    fn validate_rejects_out_of_range() {
        assert!(validate(17, 0).is_ok());
        assert!(validate(23, 59).is_ok());
        assert!(validate(24, 0).is_err());
        assert!(validate(0, 60).is_err());
    }
}
