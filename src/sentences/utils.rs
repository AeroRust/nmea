use core::str;

use arrayvec::ArrayString;
use chrono::{Duration, NaiveDate, NaiveTime};
use nom::{
    IResult, Parser as _,
    branch::alt,
    bytes::complete::{tag, take, take_until, take_while, take_while_m_n},
    character::complete::{char, digit1, one_of},
    combinator::{all_consuming, eof, map, map_parser, map_res, opt, recognize},
    number::complete::float,
    sequence::{preceded, terminated},
};

use crate::Error;

/// Parse a complete `hhmmss[.fraction]` field before its comma delimiter.
/// Fractions beyond nanosecond precision are truncated.
pub fn parse_hms(i: &str) -> IResult<&str, NaiveTime> {
    const NANOSECOND_DIGITS: usize = 9;
    const DECIMAL_BASE: u32 = 10;
    let two_digits = || {
        map_res(
            take_while_m_n(2, 2, |character: char| character.is_ascii_digit()),
            parse_num::<u32>,
        )
    };
    map_parser(
        take_until(","),
        all_consuming(map_res(
            (
                two_digits(),
                two_digits(),
                two_digits(),
                opt(preceded(char('.'), digit1)),
            ),
            |(hours, minutes, seconds, fraction): (_, _, _, Option<&str>)| {
                let nanoseconds = fraction
                    .unwrap_or("")
                    .bytes()
                    .map(|byte| u32::from(byte - b'0'))
                    .chain(core::iter::repeat(0))
                    .take(NANOSECOND_DIGITS)
                    .fold(0, |value, digit| value * DECIMAL_BASE + digit);
                NaiveTime::from_hms_nano_opt(hours, minutes, seconds, nanoseconds)
                    .ok_or("Invalid time")
            },
        )),
    )
    .parse(i)
}

/// Parses values like `125619,` and `125619.5,` to [`Duration`]
pub fn parse_duration_hms(i: &str) -> IResult<&str, Duration> {
    map(parse_hms, |time| time.signed_duration_since(NaiveTime::MIN)).parse(i)
}

pub fn do_parse_lat_lon(i: &str) -> IResult<&str, (f64, f64)> {
    let (i, lat) = parse_coordinate(i, CoordinateAxis::Latitude)?;
    let (i, _) = char(',').parse(i)?;
    let (i, lat_dir) = one_of("NS").parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, lon) = parse_coordinate(i, CoordinateAxis::Longitude)?;
    let (i, _) = char(',').parse(i)?;
    let (i, lon_dir) = one_of("EW").parse(i)?;

    Ok((
        i,
        (
            if lat_dir == 'S' { -lat } else { lat },
            if lon_dir == 'W' { -lon } else { lon },
        ),
    ))
}

#[derive(Clone, Copy)]
enum CoordinateAxis {
    Latitude,
    Longitude,
}

fn parse_coordinate(i: &str, axis: CoordinateAxis) -> IResult<&str, f64> {
    let (degree_digits, maximum_degrees) = match axis {
        CoordinateAxis::Latitude => (2, 90),
        CoordinateAxis::Longitude => (3, 180),
    };
    let decimal_digits =
        |count| take_while_m_n(count, count, |character: char| character.is_ascii_digit());
    map_res(
        (
            map_res(decimal_digits(degree_digits), parse_num::<u16>),
            recognize((decimal_digits(2), opt((char('.'), digit1)))),
        ),
        |(degrees, minutes)| {
            let (whole_minutes, fraction) = minutes.split_once('.').unwrap_or((minutes, ""));
            let whole_minutes = parse_num::<u8>(whole_minutes)?;
            if degrees > maximum_degrees
                || whole_minutes >= 60
                || (degrees == maximum_degrees
                    && (whole_minutes != 0 || fraction.bytes().any(|byte| byte != b'0')))
            {
                return Err("Invalid geographic coordinate");
            }
            let minutes = parse_num::<f64>(minutes)?;
            Ok(f64::from(degrees) + minutes / 60.0)
        },
    )
    .parse(i)
}

/// Parses the variation between magnetic north and true north.
///
/// The angle returned will be positive or negative depending on
/// the East or West direction.<br>
/// E.g:<br>
/// "14.2,E" => 14.2 <br>
/// "14.2,W" => -14.2 <br>
pub fn do_parse_magnetic_variation(i: &str) -> IResult<&str, f32> {
    let (i, variation_deg) = float(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, direction) = one_of("EW").parse(i)?;
    let variation_deg = match direction {
        'E' => variation_deg,
        'W' => -variation_deg,
        _ => unreachable!(),
    };
    Ok((i, variation_deg))
}

pub(crate) fn parse_lat_lon(i: &str) -> IResult<&str, Option<(f64, f64)>> {
    alt((map(tag(",,,"), |_| None), map(do_parse_lat_lon, Some))).parse(i)
}

pub(crate) fn parse_magnetic_variation(i: &str) -> IResult<&str, Option<f32>> {
    alt((
        map(tag(","), |_| None),
        map(do_parse_magnetic_variation, Some),
    ))
    .parse(i)
}

pub(crate) fn parse_date(i: &str) -> IResult<&str, NaiveDate> {
    map_res(
        (
            map_res(take(2usize), parse_num::<u8>),
            map_res(take(2usize), parse_num::<u8>),
            map_res(take(2usize), parse_num::<u8>),
        ),
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
    )
    .parse(i)
}

pub(crate) fn parse_num<I: str::FromStr>(data: &str) -> Result<I, &'static str> {
    data.parse::<I>().map_err(|_| "parse of number failed")
}

pub(crate) fn parse_float_num<T: str::FromStr>(input: &str) -> Result<T, &'static str> {
    str::parse::<T>(input).map_err(|_| "parse of float number failed")
}

pub(crate) fn number<T: str::FromStr>(i: &str) -> IResult<&str, T> {
    map_res(digit1, parse_num).parse(i)
}

pub(crate) fn parse_number_in_range<T>(
    i: &str,
    lower_bound: T,
    upper_bound_inclusive: T,
) -> IResult<&str, T>
where
    T: PartialOrd + str::FromStr,
{
    map_res(number::<T>, |parsed_num| {
        if parsed_num < lower_bound || parsed_num > upper_bound_inclusive {
            return Err("Parsed number is outside of the expected range");
        }
        Ok(parsed_num)
    })
    .parse(i)
}

/// Parses a given `&str` slice to an owned `ArrayString` with a given `MAX_LEN`.
///
/// # Errors
///
/// If `&str` length > `MAX_LEN` it returns a [`Error::ParameterLength`] error.
pub(crate) fn array_string<const MAX_LEN: usize>(
    string: &str,
) -> Result<ArrayString<MAX_LEN>, Error<'_>> {
    ArrayString::from(string).map_err(|_| Error::ParameterLength {
        max_length: MAX_LEN,
        parameter_length: string.len(),
    })
}

pub(crate) fn parse_until_end(input: &str) -> IResult<&str, &str> {
    all_consuming(terminated(take_while(|_| true), eof)).parse(input)
}

#[cfg(test)]
mod tests {

    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_do_parse_lat_lon() {
        let (_, lat_lon) = do_parse_lat_lon("4807.038,N,01131.324,E").unwrap();
        assert_relative_eq!(lat_lon.0, 48. + 7.038 / 60.);
        assert_relative_eq!(lat_lon.1, 11. + 31.324 / 60.);
    }

    #[test]
    fn test_parse_hms() {
        use chrono::Timelike;
        let (_, time) = parse_hms("125619,").unwrap();
        assert_eq!(time.hour(), 12);
        assert_eq!(time.minute(), 56);
        assert_eq!(time.second(), 19);
        assert_eq!(time.nanosecond(), 0);
        let (_, time) = parse_hms("125619.5,").unwrap();
        assert_eq!(time.hour(), 12);
        assert_eq!(time.minute(), 56);
        assert_eq!(time.second(), 19);
        assert_eq!(time.nanosecond(), 500_000_000);
    }

    #[test]
    fn test_parse_duration_hms() {
        let (_, time) = parse_duration_hms("125619,").unwrap();
        assert_eq!(time.num_hours(), 12);
        assert_eq!(time.num_minutes(), 12 * 60 + 56);
        assert_eq!(time.num_seconds(), 12 * 60 * 60 + 56 * 60 + 19);
        assert_eq!(
            time.num_nanoseconds().unwrap(),
            (12 * 60 * 60 + 56 * 60 + 19) * 1_000_000_000
        );
        let (_, time) = parse_duration_hms("125619.5,").unwrap();
        assert_eq!(time.num_hours(), 12);
        assert_eq!(time.num_minutes(), 12 * 60 + 56);
        assert_eq!(time.num_seconds(), 12 * 60 * 60 + 56 * 60 + 19);
        assert_eq!(
            time.num_nanoseconds().unwrap(),
            (12 * 60 * 60 + 56 * 60 + 19) * 1_000_000_000 + 500_000_000
        );
    }

    #[test]
    fn test_parse_date() {
        let (_, date) = parse_date("180283").unwrap();
        assert_eq!(
            date,
            NaiveDate::from_ymd_opt(1983, 2, 18).expect("invalid time")
        );

        let (_, date) = parse_date("180299").unwrap();
        assert_eq!(
            date,
            NaiveDate::from_ymd_opt(1999, 2, 18).expect("invalid time")
        );

        let (_, date) = parse_date("311200").unwrap();
        assert_eq!(
            date,
            NaiveDate::from_ymd_opt(2000, 12, 31).expect("invalid time")
        );

        let (_, date) = parse_date("311282").unwrap();
        assert_eq!(
            date,
            NaiveDate::from_ymd_opt(2082, 12, 31).expect("invalid time")
        );
    }

    #[test]
    fn test_parse_magnetic_variation() {
        let (_, res) = parse_magnetic_variation("12,E").unwrap();
        assert_relative_eq!(res.unwrap(), 12.0);
        let (_, res) = parse_magnetic_variation("12,W").unwrap();
        assert_relative_eq!(res.unwrap(), -12.0);

        let (_, res) = parse_magnetic_variation(",").unwrap();
        assert!(res.is_none());
        let (_, res) = parse_magnetic_variation(",,").unwrap();
        assert!(res.is_none());
        let (_, res) = parse_magnetic_variation(",W").unwrap();
        assert!(res.is_none());

        //missing direction
        let result = parse_magnetic_variation("12,");
        assert!(result.is_err());
        //illegal character for direction
        let result = parse_magnetic_variation("12,Q");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_array_string() {
        let result = array_string::<5>("12345");
        assert!(result.is_ok());

        let err_expected = Error::ParameterLength {
            max_length: 5,
            parameter_length: 6,
        };
        let result = array_string::<5>("123456");
        assert_eq!(result, Err(err_expected));
    }

    #[test]
    fn test_parse_number_in_range() {
        let result = parse_number_in_range::<u8>("12", 10, 20);
        assert!(result.is_ok());

        let result = parse_number_in_range::<u8>("9", 10, 20);
        let nom_error_expected = nom::error::Error::new("9", nom::error::ErrorKind::MapRes);
        let err = result.unwrap_err();
        assert_eq!(err, nom::Err::Error(nom_error_expected));

        let result = parse_number_in_range::<u8>("21", 10, 20);
        let nom_error_expected = nom::error::Error::new("21", nom::error::ErrorKind::MapRes);
        let err = result.unwrap_err();
        assert_eq!(err, nom::Err::Error(nom_error_expected));
    }

    #[test]
    fn test_parse_number() {
        let result = parse_num::<u8>("12");
        assert!(result.is_ok());

        let result = parse_num::<u8>("12.5");
        let err_expected = "parse of number failed";
        assert_eq!(result, Err(err_expected));
    }

    #[test]
    fn test_parse_float_num() {
        let result = parse_float_num::<f32>("12.5");
        assert!(result.is_ok());
        let result = parse_float_num::<f32>("12");
        assert!(result.is_ok());

        let result = parse_float_num::<f32>("12.5.5");
        let err_expected = "parse of float number failed";
        assert_eq!(result, Err(err_expected));
    }

    #[test]
    fn test_parse_lat_lon() {
        let (_, lat_lon) = parse_lat_lon("4807.038,N,01131.324,E").unwrap();
        assert!(lat_lon.is_some());
        let (_, lat_lon) = parse_lat_lon(",,,,").unwrap();
        assert!(lat_lon.is_none());

        let lat_lon = parse_lat_lon("0000.000,S,00000.000,E");
        assert!(lat_lon.is_ok());

        for input in [
            "51.5074,0.1278",
            "1234.567,N,09876.543,W",
            "1234.567,S,09876.543,E",
            "40.7128,",
            ", -74.0060",
            "abc,def",
        ] {
            assert!(parse_lat_lon(input).is_err(), "accepted {input}");
        }
    }
}
