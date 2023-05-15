use arrayvec::ArrayString;
use chrono::{Duration, NaiveTime};
use nom::{bytes::complete::is_not, character::complete::char, combinator::opt};

use crate::{
    parse::{NmeaSentence, TEXT_PARAMETER_MAX_LEN},
    sentences::utils::{array_string, parse_duration_hms, parse_hms},
    Error, SentenceType,
};

/// ZFO - UTC & Time from origin Waypoint
///```text
///        1         2         3    4
///        |         |         |    |
/// $--ZFO,hhmmss.ss,hhmmss.ss,c--c*hh<CR><LF>
///```
/// Field Number:
/// 1. Universal Time Coordinated (UTC) hh is hours, mm is minutes, ss.ss is seconds.
/// 2. Elapsed Time
/// 3. Origin Waypoint ID
/// 4. Checksum
#[derive(Debug, PartialEq)]
pub struct ZfoData {
    pub fix_time: Option<NaiveTime>,
    pub fix_duration: Option<Duration>,
    pub waypoint_id: Option<ArrayString<TEXT_PARAMETER_MAX_LEN>>,
}

fn do_parse_zfo(i: &str) -> Result<ZfoData, Error> {
    // 1. UTC Time or observation
    let (i, fix_time) = opt(parse_hms)(i)?;
    let (i, _) = char(',')(i)?;
    // 2. Duration
    let (i, fix_duration) = opt(parse_duration_hms)(i)?;
    let (i, _) = char(',')(i)?;

    // 12. Waypoint ID
    let (_i, waypoint_id) = opt(is_not(",*"))(i)?;

    let waypoint_id = waypoint_id
        .map(array_string::<TEXT_PARAMETER_MAX_LEN>)
        .transpose()?;

    Ok(ZfoData {
        fix_time,
        fix_duration,
        waypoint_id,
    })
}

/// # Parse ZFO message
///
/// See: <https://gpsd.gitlab.io/gpsd/NMEA.html#_zfo_utc_time_from_origin_waypoint>
pub fn parse_zfo(sentence: NmeaSentence) -> Result<ZfoData, Error> {
    if sentence.message_id != SentenceType::ZFO {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::ZFO,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_zfo(sentence.data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse::parse_nmea_sentence, Error};

    fn run_parse_zfo(line: &str) -> Result<ZfoData, Error> {
        let s = parse_nmea_sentence(line).expect("ZFO sentence initial parse failed");
        assert_eq!(s.checksum, s.calc_checksum());
        parse_zfo(s)
    }

    #[test]
    fn test_parse_zfo() {
        assert_eq!(
            ZfoData {
                fix_duration: Some(
                    Duration::hours(4)
                        + Duration::minutes(23)
                        + Duration::seconds(59)
                        + Duration::milliseconds(170)
                ),
                fix_time: NaiveTime::from_hms_milli_opt(14, 58, 32, 120),
                waypoint_id: Some(ArrayString::from("WPT").unwrap()),
            },
            run_parse_zfo("$GPZFO,145832.12,042359.17,WPT*3E").unwrap()
        );
        assert_eq!(
            ZfoData {
                fix_duration: None,
                fix_time: None,
                waypoint_id: None,
            },
            run_parse_zfo("$GPZFO,,,*68").unwrap()
        );
        assert_eq!(
            ZfoData {
                fix_duration: Some(
                    Duration::hours(4)
                        + Duration::minutes(23)
                        + Duration::seconds(59)
                        + Duration::milliseconds(170)
                ),
                fix_time: None,
                waypoint_id: None,
            },
            run_parse_zfo("$GPZFO,,042359.17,*49").unwrap()
        );
    }

    #[test]
    fn test_parse_zfo_with_too_long_waypoint() {
        assert_eq!(
            Error::ParameterLength { max_length: 64, parameter_length: 72 },
            run_parse_zfo("$GPZFO,145832.12,042359.17,ABCDEFGHIJKLMNOPRSTUWXYZABCDEFGHIJKLMNOPRSTUWXYZABCDEFGHIJKLMNOPRSTUWXYZ*71").unwrap_err()
        );
    }
}
