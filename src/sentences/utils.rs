use std::str;

use chrono::{NaiveDate, NaiveTime};
use nom::branch::alt;
use nom::bytes::complete::{tag, take, take_until};
use nom::character::complete::{char, digit1, one_of};
use nom::combinator::{map, map_parser, map_res};
use nom::number::complete::double;
use nom::sequence::tuple;
use nom::IResult;

pub(crate) fn parse_hms(i: &[u8]) -> IResult<&[u8], NaiveTime> {
    map_res(
        tuple((
            map_res(take(2usize), parse_num::<u32>),
            map_res(take(2usize), parse_num::<u32>),
            map_parser(take_until(","), double),
        )),
        |(hour, minutes, sec)| -> std::result::Result<NaiveTime, &'static str> {
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
            Ok(NaiveTime::from_hms_nano(
                hour,
                minutes,
                sec.trunc() as u32,
                (sec.fract() * 1_000_000_000f64).round() as u32,
            ))
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
            if month < 1 || month > 12 {
                return Err("Invalid month < 1 or > 12");
            }
            if day < 1 || day > 31 {
                return Err("Invalid day < 1 or > 31");
            }
            Ok(NaiveDate::from_ymd(year, month, day))
        },
    )(i)
}

pub(crate) fn parse_num<I: std::str::FromStr>(data: &[u8]) -> std::result::Result<I, &'static str> {
    //    println!("parse num {}", unsafe { str::from_utf8_unchecked(data) });
    str::parse::<I>(unsafe { str::from_utf8_unchecked(data) }).map_err(|_| "parse of number failed")
}

pub(crate) fn parse_float_num<T: str::FromStr>(
    input: &[u8],
) -> std::result::Result<T, &'static str> {
    let s = str::from_utf8(input).map_err(|_| "invalid float number")?;
    str::parse::<T>(s).map_err(|_| "parse of float number failed")
}

pub(crate) fn number<T: std::str::FromStr>(i: &[u8]) -> IResult<&[u8], T> {
    map_res(digit1, parse_num)(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::relative_eq;

    #[test]
    fn test_do_parse_lat_lon() {
        let (_, lat_lon) = do_parse_lat_lon(b"4807.038,N,01131.324,E").unwrap();
        relative_eq!(lat_lon.0, 48. + 7.038 / 60.);
        relative_eq!(lat_lon.1, 11. + 31.324 / 60.);
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
        assert_eq!(time.nanosecond(), 5_00_000_000);
    }
}
