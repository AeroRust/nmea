use chrono::{NaiveDate, NaiveTime};

use nom::character::complete::{char, one_of};
use nom::combinator::opt;

use nom::number::complete::float;
use nom::IResult;

use crate::parse::NmeaSentence;
use crate::{
    sentences::utils::{parse_date, parse_hms, parse_lat_lon},
    NmeaError,
};
#[derive(Debug, PartialEq)]
pub enum RmcStatusOfFix {
    Autonomous,
    Differential,
    Invalid,
}

#[derive(Debug, PartialEq)]
pub struct RmcData {
    pub fix_time: Option<NaiveTime>,
    pub fix_date: Option<NaiveDate>,
    pub status_of_fix: Option<RmcStatusOfFix>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub speed_over_ground: Option<f32>,
    pub true_course: Option<f32>,
}

fn do_parse_rmc(i: &[u8]) -> IResult<&[u8], RmcData> {
    let (i, fix_time) = opt(parse_hms)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, status_of_fix) = one_of("ADV")(i)?;
    let (i, _) = char(',')(i)?;
    let (i, lat_lon) = parse_lat_lon(i)?;
    let (i, _) = char(',')(i)?;
    let (i, speed_over_ground) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, true_course) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, fix_date) = opt(parse_date)(i)?;
    let (i, _) = char(',')(i)?;
    Ok((
        i,
        RmcData {
            fix_time,
            fix_date,
            status_of_fix: Some(match status_of_fix {
                'A' => RmcStatusOfFix::Autonomous,
                'D' => RmcStatusOfFix::Differential,
                'V' => RmcStatusOfFix::Invalid,
                _ => unreachable!(),
            }),
            lat: lat_lon.map(|v| v.0),
            lon: lat_lon.map(|v| v.1),
            speed_over_ground,
            true_course,
        },
    ))
}

/// Parse RMC message
/// From gpsd:
/// RMC,225446.33,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E,A*68
/// 1     225446.33    Time of fix 22:54:46 UTC
/// 2     A          Status of Fix: A = Autonomous, valid;
/// D = Differential, valid; V = invalid
/// 3,4   4916.45,N    Latitude 49 deg. 16.45 min North
/// 5,6   12311.12,W   Longitude 123 deg. 11.12 min West
/// 7     000.5      Speed over ground, Knots
/// 8     054.7      Course Made Good, True north
/// 9     181194       Date of fix  18 November 1994
/// 10,11 020.3,E      Magnetic variation 20.3 deg East
/// 12    A      FAA mode indicator (NMEA 2.3 and later)
/// A=autonomous, D=differential, E=Estimated,
/// N=not valid, S=Simulator, M=Manual input mode
/// *68        mandatory nmea_checksum
///
/// SiRF chipsets don't return either Mode Indicator or magnetic variation.
pub fn parse_rmc(sentence: NmeaSentence) -> Result<RmcData, NmeaError> {
    if sentence.message_id != b"RMC" {
        Err(NmeaError::WrongSentenceHeader {
            expected: b"RMC",
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_rmc(sentence.data)?.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_nmea_sentence;
    use approx::relative_eq;

    #[test]
    fn test_parse_rmc() {
        let s = parse_nmea_sentence(
            b"$GPRMC,225446.33,A,4916.45,N,12311.12,W,\
                                  000.5,054.7,191194,020.3,E,A*2B",
        )
        .unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x2b);
        let rmc_data = parse_rmc(s).unwrap();
        assert_eq!(
            rmc_data.fix_time.unwrap(),
            NaiveTime::from_hms_milli(22, 54, 46, 330)
        );
        assert_eq!(rmc_data.fix_date.unwrap(), NaiveDate::from_ymd(1994, 11, 19));

        println!("lat: {}", rmc_data.lat.unwrap());
        relative_eq!(rmc_data.lat.unwrap(), 49.0 + 16.45 / 60.);
        println!(
            "lon: {}, diff {}",
            rmc_data.lon.unwrap(),
            (rmc_data.lon.unwrap() + (123.0 + 11.12 / 60.)).abs()
        );
        relative_eq!(rmc_data.lon.unwrap(), -(123.0 + 11.12 / 60.));

        relative_eq!(rmc_data.speed_over_ground.unwrap(), 0.5);
        relative_eq!(rmc_data.true_course.unwrap(), 54.7);

        let s = parse_nmea_sentence(b"$GPRMC,,V,,,,,,,,,,N*53").unwrap();
        let rmc = parse_rmc(s).unwrap();
        assert_eq!(
            RmcData {
                fix_time: None,
                fix_date: None,
                status_of_fix: Some(RmcStatusOfFix::Invalid),
                lat: None,
                lon: None,
                speed_over_ground: None,
                true_course: None,
            },
            rmc
        );
    }
}
