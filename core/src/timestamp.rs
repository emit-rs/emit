/*!
The [`Timestamp`] type.

A timestamp is a point in time, represented as the number of nanoseconds since the Unix epoch.

Timestamps can be constructed manually through [`Timestamp::from_unix`], or the current timestamp can be read from an instance of [`crate::clock::Clock`].

A timestamp can be converted into a point [`crate::extent::Extent`]. A pair of timestamps representing a timespan can be converted into a span [`crate::extent::Extent`].
*/

/*
Parts of this file are adapted from other libraries:

Post:
https://github.com/tokio-rs/prost/blob/master/prost-types/src/datetime.rs
Licensed under Apache 2.0

humantime:
https://github.com/tailhook/humantime/blob/master/src/date.rs
Licensed under MIT
*/

use core::{
    cmp, fmt,
    ops::{Add, AddAssign, Sub, SubAssign},
    str::{self, FromStr},
    time::Duration,
};

use crate::value::{FromValue, ToValue, Value};

/**
A Unix timestamp with nanosecond precision.
*/
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(Duration);

/**
The individual date and time portions of a timestamp.

Values in parts are represented exactly as they would be when formatted into a timestamp. So months and days are both one-based instead of zero-based values.
*/
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Parts {
    /**
    The zero-based year.
    */
    pub years: u16,
    /**
    The one-based month.
    */
    pub months: u8,
    /**
    The one-based day.
    */
    pub days: u8,
    /**
    The zero-based hour of the day.
    */
    pub hours: u8,
    /**
    The zero-based minute of the hour.
    */
    pub minutes: u8,
    /**
    The zero-based second of the minute.
    */
    pub seconds: u8,
    /**
    The zero-based subsecond precision.
    */
    pub nanos: u32,
}

// 2000-03-01 (mod 400 year, immediately after feb29
const LEAPOCH_SECS: u64 = 946_684_800 + 86400 * (31 + 29);
const DAYS_PER_400Y: i32 = 365 * 400 + 97;
const DAYS_PER_100Y: i32 = 365 * 100 + 24;
const DAYS_PER_4Y: i32 = 365 * 4 + 1;
const DAYS_IN_MONTH: [u8; 12] = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 29];

// 1970-01-01T00:00:00.000000000Z
const MIN: Duration = Duration::new(0, 0);

// 9999-12-31T23:59:59.999999999Z
const MAX: Duration = Duration::new(253402300799, 999999999);

impl Timestamp {
    /**
    The minimum timestamp, `1970-01-01T00:00:00Z`.
    */
    pub const MIN: Self = Timestamp(MIN);

    /**
    The maximum timestamp, `9999-12-31T23:59:59.999999999Z`.
    */
    pub const MAX: Self = Timestamp(MAX);

    /**
    Try create a timestamp from time since the Unix epoch.

    If the `unix_time` is within [`Timestamp::MIN`]..=[`Timestamp::MAX`] then this method will return `Some`. Otherwise it will return `None`.
    */
    pub fn from_unix(unix_time: Duration) -> Option<Self> {
        if unix_time >= MIN && unix_time <= MAX {
            Some(Timestamp(unix_time))
        } else {
            None
        }
    }

    /**
    Get the value of the timestamp as time since the Unix epoch.
    */
    pub fn to_unix(&self) -> Duration {
        self.0
    }

    /**
    Calculate the timespan between two timestamps.

    This method will return `None` if `earlier` is actually after `self`.
    */
    pub fn duration_since(self, earlier: Self) -> Option<Duration> {
        self.0.checked_sub(earlier.0)
    }

    /**
    Convert the timestamp into a system timestamp.

    This method can be used for interoperability with code expecting a standard library timestamp.
    */
    #[cfg(feature = "std")]
    pub fn to_system_time(&self) -> std::time::SystemTime {
        std::time::SystemTime::UNIX_EPOCH + self.0
    }

    /**
    Try get a timestamp from its individual date and time parts.

    If the resulting timestamp is within [`Timestamp::MIN`]..=[`Timestamp::MAX`] then this method will return `Some`. Otherwise it will return `None`.

    If any field of `parts` would overflow its maximum value, such as `days: 32`, then it will wrap into the next unit.
    */
    pub fn from_parts(parts: Parts) -> Option<Self> {
        let is_leap;
        let start_of_year;
        let year = (parts.years as i64) - 1900;

        // Fast path for years 1900 - 2038.
        // The `as u64` conversion here turns negative values
        // into very large positive ones, failing the `<=`
        if year as u64 <= 138 {
            let mut leaps: i64 = (year - 68) >> 2;
            if (year - 68).trailing_zeros() >= 2 {
                leaps -= 1;
                is_leap = true;
            } else {
                is_leap = false;
            }

            start_of_year = i128::from(31_536_000 * (year - 70) + 86400 * leaps);
        } else {
            let centuries: i64;
            let mut leaps: i64;

            let mut cycles: i64 = (year - 100) / 400;
            let mut rem: i64 = (year - 100) % 400;

            if rem < 0 {
                cycles -= 1;
                rem += 400
            }
            if rem == 0 {
                is_leap = true;
                centuries = 0;
                leaps = 0;
            } else {
                if rem >= 200 {
                    if rem >= 300 {
                        centuries = 3;
                        rem -= 300;
                    } else {
                        centuries = 2;
                        rem -= 200;
                    }
                } else if rem >= 100 {
                    centuries = 1;
                    rem -= 100;
                } else {
                    centuries = 0;
                }
                if rem == 0 {
                    is_leap = false;
                    leaps = 0;
                } else {
                    leaps = rem / 4;
                    rem %= 4;
                    is_leap = rem == 0;
                }
            }
            leaps += 97 * cycles + 24 * centuries - i64::from(is_leap);

            start_of_year = i128::from((year - 100) * 31_536_000)
                + i128::from(leaps * 86400 + 946_684_800 + 86400);
        }

        let seconds_within_month = 86400 * u32::from(parts.days - 1)
            + 3600 * u32::from(parts.hours)
            + 60 * u32::from(parts.minutes)
            + u32::from(parts.seconds);

        let mut seconds_within_year = [
            0,           // Jan
            31 * 86400,  // Feb
            59 * 86400,  // Mar
            90 * 86400,  // Apr
            120 * 86400, // May
            151 * 86400, // Jun
            181 * 86400, // Jul
            212 * 86400, // Aug
            243 * 86400, // Sep
            273 * 86400, // Oct
            304 * 86400, // Nov
            334 * 86400, // Dec
        ][usize::from(parts.months - 1) % 12]
            + seconds_within_month;

        if is_leap && parts.months > 2 {
            seconds_within_year += 86400
        }

        Timestamp::from_unix(Duration::new(
            (start_of_year + i128::from(seconds_within_year))
                .try_into()
                .ok()?,
            parts.nanos,
        ))
    }

    /**
    Get the individual date and time parts of the timestamp.

    The returned parts are in exactly the form needed to display them. Months and days are both one-based.
    */
    pub fn to_parts(&self) -> Parts {
        let dur = self.0;
        let secs = dur.as_secs();
        let nanos = dur.subsec_nanos();

        let mut days = ((secs as i64 / 86_400) - (LEAPOCH_SECS as i64 / 86_400)) as i64;
        let mut remsecs = (secs % 86_400) as i32;
        if remsecs < 0i32 {
            remsecs += 86_400;
            days -= 1
        }

        let mut qc_cycles: i32 = (days / (DAYS_PER_400Y as i64)) as i32;
        let mut remdays: i32 = (days % (DAYS_PER_400Y as i64)) as i32;
        if remdays < 0 {
            remdays += DAYS_PER_400Y;
            qc_cycles -= 1;
        }

        let mut c_cycles: i32 = remdays / DAYS_PER_100Y;
        if c_cycles == 4 {
            c_cycles -= 1;
        }
        remdays -= c_cycles * DAYS_PER_100Y;

        let mut q_cycles: i32 = remdays / DAYS_PER_4Y;
        if q_cycles == 25 {
            q_cycles -= 1;
        }
        remdays -= q_cycles * DAYS_PER_4Y;

        let mut remyears: i32 = remdays / 365;
        if remyears == 4 {
            remyears -= 1;
        }
        remdays -= remyears * 365;

        let mut years: i64 = i64::from(remyears)
            + 4 * i64::from(q_cycles)
            + 100 * i64::from(c_cycles)
            + 400 * i64::from(qc_cycles);

        let mut months: i32 = 0;
        while i32::from(DAYS_IN_MONTH[months as usize]) <= remdays {
            remdays -= i32::from(DAYS_IN_MONTH[months as usize]);
            months += 1
        }

        if months >= 10 {
            months -= 12;
            years += 1;
        }

        let years = (years + 2000) as u16;
        let months = (months + 3) as u8;
        let days = (remdays + 1) as u8;
        let hours = (remsecs / 3600) as u8;
        let minutes = (remsecs / 60 % 60) as u8;
        let seconds = (remsecs % 60) as u8;

        Parts {
            years,
            months,
            days,
            hours,
            minutes,
            seconds,
            nanos,
        }
    }

    /**
    Add a duration to this timestamp.

    If the result would be greater than [`Timestamp::MAX`] then `None` is returned.
    */
    #[must_use = "the result of addition is returned without modifying the original"]
    pub fn checked_add(&self, rhs: Duration) -> Option<Self> {
        Timestamp::from_unix(self.to_unix().checked_add(rhs)?)
    }

    /**
    Subtract a duration to this timestamp.

    If the result would be less than [`Timestamp::MIN`] then `None` is returned.
    */
    #[must_use = "the result of subtraction is returned without modifying the original"]
    pub fn checked_sub(&self, rhs: Duration) -> Option<Self> {
        Timestamp::from_unix(self.to_unix().checked_sub(rhs)?)
    }
}

impl Add<Duration> for Timestamp {
    type Output = Timestamp;

    fn add(self, rhs: Duration) -> Self::Output {
        self.checked_add(rhs).expect("overflow adding to timestamp")
    }
}

impl AddAssign<Duration> for Timestamp {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Timestamp;

    fn sub(self, rhs: Duration) -> Self::Output {
        self.checked_sub(rhs)
            .expect("overflow subtracting from timestamp")
    }
}

impl SubAssign<Duration> for Timestamp {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use fmt::Write as _;

        f.write_char('"')?;
        fmt_rfc3339(*self, f)?;
        f.write_char('"')
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_rfc3339(*self, f)
    }
}

impl FromStr for Timestamp {
    type Err = ParseTimestampError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_rfc3339(s)
    }
}

impl ToValue for Timestamp {
    fn to_value(&self) -> Value {
        Value::capture_display(self)
    }
}

impl<'v> FromValue<'v> for Timestamp {
    fn from_value(value: Value<'v>) -> Option<Self> {
        value
            .downcast_ref::<Timestamp>()
            .copied()
            .or_else(|| value.parse())
    }
}

impl<'a> PartialEq<&'a Timestamp> for Timestamp {
    fn eq(&self, other: &&'a Timestamp) -> bool {
        self == *other
    }
}

impl<'a> PartialEq<Timestamp> for &'a Timestamp {
    fn eq(&self, other: &Timestamp) -> bool {
        *self == other
    }
}

/**
An error attempting to parse a [`Timestamp`] from text.
*/
#[derive(Debug)]
pub struct ParseTimestampError {}

impl fmt::Display for ParseTimestampError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the input was not a valid timestamp")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseTimestampError {}

fn parse_rfc3339(fmt: &str) -> Result<Timestamp, ParseTimestampError> {
    if fmt.len() > 30 || fmt.len() < 19 {
        // Invalid length
        return Err(ParseTimestampError {});
    }

    if *fmt.as_bytes().last().unwrap() != b'Z' {
        // Non-UTC
        return Err(ParseTimestampError {});
    }

    let years = u16::from_str_radix(&fmt[0..4], 10).map_err(|_| ParseTimestampError {})?;
    let months = u8::from_str_radix(&fmt[5..7], 10).map_err(|_| ParseTimestampError {})?;
    let days = u8::from_str_radix(&fmt[8..10], 10).map_err(|_| ParseTimestampError {})?;
    let hours = u8::from_str_radix(&fmt[11..13], 10).map_err(|_| ParseTimestampError {})?;
    let minutes = u8::from_str_radix(&fmt[14..16], 10).map_err(|_| ParseTimestampError {})?;
    let seconds = u8::from_str_radix(&fmt[17..19], 10).map_err(|_| ParseTimestampError {})?;
    let nanos = if fmt.len() > 19 {
        let subsecond = &fmt[20..fmt.len() - 1];
        u32::from_str_radix(subsecond, 10).unwrap() * 10u32.pow(9 - subsecond.len() as u32)
    } else {
        0
    };

    Timestamp::from_parts(Parts {
        years,
        months,
        days,
        hours,
        minutes,
        seconds,
        nanos,
    })
    .ok_or_else(|| ParseTimestampError {})
}

fn fmt_rfc3339(ts: Timestamp, f: &mut fmt::Formatter) -> fmt::Result {
    let Parts {
        years,
        months,
        days,
        hours,
        minutes,
        seconds,
        nanos: subsecond_nanos,
    } = ts.to_parts();

    const BUF_INIT: [u8; 30] = *b"0000-00-00T00:00:00.000000000Z";

    let mut buf: [u8; 30] = BUF_INIT;
    buf[0] = b'0' + (years / 1000) as u8;
    buf[1] = b'0' + (years / 100 % 10) as u8;
    buf[2] = b'0' + (years / 10 % 10) as u8;
    buf[3] = b'0' + (years % 10) as u8;
    buf[5] = b'0' + (months / 10) as u8;
    buf[6] = b'0' + (months % 10) as u8;
    buf[8] = b'0' + (days / 10) as u8;
    buf[9] = b'0' + (days % 10) as u8;
    buf[11] = b'0' + (hours / 10) as u8;
    buf[12] = b'0' + (hours % 10) as u8;
    buf[14] = b'0' + (minutes / 10) as u8;
    buf[15] = b'0' + (minutes % 10) as u8;
    buf[17] = b'0' + (seconds / 10) as u8;
    buf[18] = b'0' + (seconds % 10) as u8;

    let i = match f.precision() {
        Some(0) => 19,
        precision => {
            let mut i = 20;
            let mut divisor = 100_000_000;
            let end = i + cmp::min(9, precision.unwrap_or(9));

            while i < end {
                buf[i] = b'0' + (subsecond_nanos / divisor % 10) as u8;

                i += 1;
                divisor /= 10;
            }

            i
        }
    };

    buf[i] = b'Z';

    // we know our chars are all ascii
    f.write_str(str::from_utf8(&buf[..=i]).expect("Conversion to utf8 failed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let ts = Timestamp::from_unix(Duration::new(1691961703, 17532)).unwrap();

        let fmt = ts.to_string();

        let parsed: Timestamp = fmt.parse().unwrap();

        assert_eq!(ts, parsed, "{}", fmt);
    }

    #[test]
    fn parts_max() {
        let ts = Timestamp::from_parts(Parts {
            years: 9999,
            months: 12,
            days: 31,
            hours: 23,
            minutes: 59,
            seconds: 59,
            nanos: 999999999,
        })
        .unwrap();

        assert_eq!(ts.to_unix(), MAX);
    }

    #[test]
    fn parts_min() {
        let ts = Timestamp::from_parts(Parts {
            years: 1970,
            months: 1,
            days: 1,
            hours: 0,
            minutes: 0,
            seconds: 0,
            nanos: 0,
        })
        .unwrap();

        assert_eq!(ts.to_unix(), MIN);
    }

    #[test]
    fn parts_overflow() {
        let ts = Timestamp::from_parts(Parts {
            years: 2000,
            months: 13,
            days: 32,
            hours: 25,
            minutes: 61,
            seconds: 61,
            nanos: 1000000000,
        })
        .unwrap();

        let expected = Timestamp::from_parts(Parts {
            years: 2000,
            months: 13,
            days: 32,
            hours: 25,
            minutes: 61,
            seconds: 62,
            nanos: 0,
        })
        .unwrap();

        assert_eq!(expected, ts);
    }

    #[test]
    fn add() {
        for (case, add, expected) in [
            (
                Timestamp::MIN,
                Duration::from_nanos(1),
                Some(Timestamp::from_unix(Duration::from_nanos(1)).unwrap()),
            ),
            (Timestamp::MAX, Duration::from_nanos(1), None),
            (Timestamp::MAX, Duration::MAX, None),
        ] {
            assert_eq!(expected, case.checked_add(add));
        }
    }

    #[test]
    fn sub() {
        for (case, sub, expected) in [
            (
                Timestamp::MAX,
                Duration::from_nanos(1),
                Some(Timestamp::from_unix(MAX - Duration::from_nanos(1)).unwrap()),
            ),
            (Timestamp::MIN, Duration::from_nanos(1), None),
            (Timestamp::MIN, Duration::MAX, None),
        ] {
            assert_eq!(expected, case.checked_sub(sub));
        }
    }

    #[test]
    fn to_from_value() {
        for case in [
            Timestamp::MIN,
            Timestamp::MAX,
            Timestamp::from_unix(Duration::from_secs(1)).unwrap(),
        ] {
            let value = case.to_value();

            assert_eq!(case, value.cast::<Timestamp>().unwrap());
        }

        for (case, expected) in [
            (
                Value::from("2024-01-01T00:13:00.000Z"),
                Some(
                    Timestamp::from_parts(Parts {
                        years: 2024,
                        months: 01,
                        days: 01,
                        hours: 00,
                        minutes: 13,
                        seconds: 00,
                        nanos: 000,
                    })
                    .unwrap(),
                ),
            ),
            (Value::from(""), None),
            (Value::from("12024-01-01T00:13:00.000Z"), None),
            (Value::from("2024-01-01T00:13:00.000+10"), None),
        ] {
            assert_eq!(expected, case.cast::<Timestamp>());
        }
    }
}
