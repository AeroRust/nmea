use core::str;

use arrayvec::ArrayString;
use chrono::{NaiveDate, NaiveTime};
use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_until},
    character::complete::{char, digit1, one_of},
    combinator::{map, map_parser, map_res},
    number::complete::double,
    sequence::tuple,
    IResult,
};
#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
use num_traits::float::FloatCore;

use crate::NmeaError;

pub(crate) fn parse_hms(i: &[u8]) -> IResult<&[u8], NaiveTime> {
    map_res(
        tuple((
            map_res(take(2usize), parse_num::<u32>),
            map_res(take(2usize), parse_num::<u32>),
            map_parser(take_until(","), double),
        )),
        |(hour, minutes, sec)| -> core::result::Result<NaiveTime, &'static str> {
            if sec.is_sign_negative() {
                return Err("Invalid time: second is negative");
            }
            if hour >= 24 {
                return Err("Invalid time: hour >= 24");
            }
            if minutes >= 60 {
                return Err("Invalid time: min >= 60");
            }
            if sec >= 60. {
                return Err("Invalid time: sec >= 60");
            }
            NaiveTime::from_hms_nano_opt(
                hour,
                minutes,
                sec.trunc() as u32,
                (sec.fract() * 1_000_000_000f64).round() as u32,
            )
            .ok_or("Invalid time")
        },
    )(i)
}

pub fn do_parse_lat_lon(i: &[u8]) -> IResult<&[u8], (f64, f64)> {
    let (i, lat_deg) = map_res(take(2usize), parse_num::<u8>)(i)?;
    let (i, lat_min) = double(i)?;
    let (i, _) = char(',')(i)?;
    let (i, lat_dir) = one_of("NS")(i)?;
    let (i, _) = char(',')(i)?;
    let (i, lon_deg) = map_res(take(3usize), parse_num::<u8>)(i)?;
    let (i, lon_min) = double(i)?;
    let (i, _) = char(',')(i)?;
    let (i, lon_dir) = one_of("EW")(i)?;

    let mut lat = f64::from(lat_deg) + lat_min / 60.;
    if lat_dir == 'S' {
        lat = -lat;
    }
    let mut lon = f64::from(lon_deg) + lon_min / 60.;
    if lon_dir == 'W' {
        lon = -lon;
    }

    Ok((i, (lat, lon)))
}

pub(crate) fn parse_lat_lon(i: &[u8]) -> IResult<&[u8], Option<(f64, f64)>> {
    alt((map(tag(",,,"), |_| None), map(do_parse_lat_lon, Some)))(i)
}

pub(crate) fn parse_date(i: &[u8]) -> IResult<&[u8], NaiveDate> {
    map_res(
        tuple((
            map_res(take(2usize), parse_num::<u8>),
            map_res(take(2usize), parse_num::<u8>),
            map_res(take(2usize), parse_num::<u8>),
        )),
        |data| -> Result<NaiveDate, &'static str> {
            let (day, month, year) = (u32::from(data.0), u32::from(data.1), i32::from(data.2));

            // We only receive a 2digit year code in this message, this has the potential
            // to be ambiguous regarding the year. We assume that anything above 83 is 1900's, and
            // anything above 0 is 2000's.
            //
            // The reason for 83 is that NMEA0183 was released in 1983.
            // Parsing dates from ZDA messages is preferred, since it includes a 4 digit year.
            let year = match year {
                83..=99 => year + 1900,
                _ => year + 2000,
            };

            if !(1..=12).contains(&month) {
                return Err("Invalid month < 1 or > 12");
            }
            if !(1..=31).contains(&day) {
                return Err("Invalid day < 1 or > 31");
            }
            NaiveDate::from_ymd_opt(year, month, day).ok_or("Invalid date")
        },
    )(i)
}

pub(crate) fn parse_num<I: str::FromStr>(data: &[u8]) -> Result<I, &'static str> {
    str::from_utf8(data)
        .map_err(|_| "Number str is not UTF-8")?
        .parse::<I>()
        .map_err(|_| "parse of number failed")
}

pub(crate) fn parse_float_num<T: str::FromStr>(input: &[u8]) -> Result<T, &'static str> {
    let s = str::from_utf8(input).map_err(|_| "invalid float number")?;
    str::parse::<T>(s).map_err(|_| "parse of float number failed")
}

pub(crate) fn number<T: str::FromStr>(i: &[u8]) -> IResult<&[u8], T> {
    map_res(digit1, parse_num)(i)
}

/// Parses a given `&str` slice to an owned `ArrayString` with a given `MAX_LEN`.
///
/// # Errors
///
/// If `&str` length > `MAX_LEN` it returns a [`NmeaError::SentenceLength`] error.
pub(crate) fn array_string<const MAX_LEN: usize>(
    string: &str,
) -> Result<ArrayString<MAX_LEN>, NmeaError> {
    ArrayString::from(string).map_err(|_e| NmeaError::SentenceLength(string.len()))
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_do_parse_lat_lon() {
        let (_, lat_lon) = do_parse_lat_lon(b"4807.038,N,01131.324,E").unwrap();
        assert_relative_eq!(lat_lon.0, 48. + 7.038 / 60.);
        assert_relative_eq!(lat_lon.1, 11. + 31.324 / 60.);
    }

    #[test]
    fn test_parse_hms() {
        use chrono::Timelike;
        let (_, time) = parse_hms(b"125619,").unwrap();
        assert_eq!(time.hour(), 12);
        assert_eq!(time.minute(), 56);
        assert_eq!(time.second(), 19);
        assert_eq!(time.nanosecond(), 0);
        let (_, time) = parse_hms(b"125619.5,").unwrap();
        assert_eq!(time.hour(), 12);
        assert_eq!(time.minute(), 56);
        assert_eq!(time.second(), 19);
        assert_eq!(time.nanosecond(), 500_000_000);
    }

    #[test]
    fn test_parse_date() {
        let (_, date) = parse_date(b"180283").unwrap();
        assert_eq!(date, NaiveDate::from_ymd(1983, 2, 18));

        let (_, date) = parse_date(b"180299").unwrap();
        assert_eq!(date, NaiveDate::from_ymd(1999, 2, 18));

        let (_, date) = parse_date(b"311200").unwrap();
        assert_eq!(date, NaiveDate::from_ymd(2000, 12, 31));

        let (_, date) = parse_date(b"311282").unwrap();
        assert_eq!(date, NaiveDate::from_ymd(2082, 12, 31));
    }
}
