use chrono::{NaiveDate, NaiveTime};
use nom::{
    character::complete::{anychar, char, one_of},
    combinator::{cond, map_res, opt},
    number::complete::float,
    IResult,
};

use crate::{
    parse::NmeaSentence,
    sentences::utils::{parse_date, parse_hms, parse_lat_lon},
    Error, SentenceType,
};

use super::{faa_mode::parse_faa_mode, utils::parse_magnetic_variation, FaaMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RmcStatusOfFix {
    Autonomous,
    Differential,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RmcNavigationStatus {
    Autonomous,
    Differential,
    Estimated,
    Manual,
    NotValid,
    Simulator,
    Valid,
}

/// RMC - Recommended Minimum Navigation Information
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_rmc_recommended_minimum_navigation_information>
///
/// ```text
///         1         2 3       4 5        6  7   8   9    10 11
///         |         | |       | |        |  |   |   |    |  |
///  $--RMC,hhmmss.ss,A,ddmm.mm,a,dddmm.mm,a,x.x,x.x,xxxx,x.x,a*hh<CR><LF>
/// ```
///
/// NMEA 2.3:
///
/// ```text
///         1         2 3       4 5        6  7   8   9    10 1112
///         |         | |       | |        |  |   |   |    |  | |
///  $--RMC,hhmmss.ss,A,ddmm.mm,a,dddmm.mm,a,x.x,x.x,xxxx,x.x,a,m*hh<CR><LF>
/// ```
///
/// NMEA 4.1:
/// ```text
///         1         2 3       4 5        6  7   8   9    10 111213
///         |         | |       | |        |  |   |   |    |  | | |
///  $--RMC,hhmmss.ss,A,ddmm.mm,a,dddmm.mm,a,x.x,x.x,xxxx,x.x,a,m,s*hh<CR><LF>
/// ```
///
/// 1.  UTC of position fix, `hh` is hours, `mm` is minutes, `ss.ss` is seconds.
/// 2.  Status, `A` = Valid, `V` = Warning
/// 3.  Latitude, `dd` is degrees. `mm.mm` is minutes.
/// 4.  `N` or `S`
/// 5.  Longitude, `ddd` is degrees. `mm.mm` is minutes.
/// 6.  `E` or `W`
/// 7.  Speed over ground, knots
/// 8.  Track made good, degrees true
/// 9.  Date, `ddmmyy`
/// 10. Magnetic Variation, degrees
/// 11. `E` or `W`
/// 12. FAA mode indicator (NMEA 2.3 and later)
/// 13. Nav Status (NMEA 4.1 and later)
///     `A` = autonomous, `D` = differential, `E` = Estimated,
///     `M` = Manual input mode, `N` = not valid, `S` = Simulator, `V` = Valid
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RmcData {
    pub fix_time: Option<NaiveTime>,
    pub fix_date: Option<NaiveDate>,
    pub status_of_fix: RmcStatusOfFix,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub speed_over_ground: Option<f32>,
    pub true_course: Option<f32>,
    pub magnetic_variation: Option<f64>,
    pub faa_mode: Option<FaaMode>,
    pub nav_status: Option<RmcNavigationStatus>,
}

fn do_parse_rmc(i: &str) -> IResult<&str, RmcData> {
    // 1.  UTC of position fix, `hh` is hours, `mm` is minutes, `ss.ss` is seconds.
    let (i, fix_time) = opt(parse_hms)(i)?;
    let (i, _) = char(',')(i)?;
    // 2.  Status, `A` = Valid, `V` = Warning
    let (i, status_of_fix) = one_of("ADV")(i)?;
    let status_of_fix = match status_of_fix {
        'A' => RmcStatusOfFix::Autonomous,
        'D' => RmcStatusOfFix::Differential,
        'V' => RmcStatusOfFix::Invalid,
        _ => unreachable!(),
    };
    let (i, _) = char(',')(i)?;
    // 3.  Latitude, `dd` is degrees. `mm.mm` is minutes.
    // 4.  `N` or `S`
    // 5.  Longitude, `ddd` is degrees. `mm.mm` is minutes.
    // 6.  `E` or `W`
    let (i, lat_lon) = parse_lat_lon(i)?;
    let (i, _) = char(',')(i)?;
    // 7.  Speed over ground, knots
    let (i, speed_over_ground) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    // 8.  Track made good, degrees true
    let (i, true_course) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    // 9.  Date, `ddmmyy`
    let (i, fix_date) = opt(parse_date)(i)?;
    let (i, _) = char(',')(i)?;
    // 10. Magnetic Variation, degrees
    // // 11. `E` or `W`
    let (i, magnetic_variation) = parse_magnetic_variation(i)?;
    let (i, next) = opt(char(','))(i)?;
    // 12. FAA mode indicator (NMEA 2.3 and later)
    let (i, faa_mode) = cond(
        next.is_some(),
        opt(map_res(anychar, |c| parse_faa_mode(c).ok_or("argh"))),
    )(i)?;
    let (i, next) = opt(char(','))(i)?;
    // 13. Nav Status (NMEA 4.1 and later)
    //     `A` = autonomous, `D` = differential, `E` = Estimated,
    //     `M` = Manual input mode, `N` = not valid, `S` = Simulator, `V` = Valid
    let (i, nav_status) = cond(next.is_some(), opt(parse_navigation_status))(i)?;

    Ok((
        i,
        RmcData {
            fix_time,
            fix_date,
            status_of_fix,
            lat: lat_lon.map(|v| v.0),
            lon: lat_lon.map(|v| v.1),
            speed_over_ground,
            true_course,
            magnetic_variation,
            faa_mode: faa_mode.flatten(),
            nav_status: nav_status.flatten(),
        },
    ))
}

fn parse_navigation_status(i: &str) -> IResult<&str, RmcNavigationStatus> {
    let (i, c) = one_of("ADEMNSV")(i)?;
    let status = match c {
        'A' => RmcNavigationStatus::Autonomous,
        'D' => RmcNavigationStatus::Differential,
        'E' => RmcNavigationStatus::Estimated,
        'M' => RmcNavigationStatus::Manual,
        'N' => RmcNavigationStatus::NotValid,
        'S' => RmcNavigationStatus::Simulator,
        'V' => RmcNavigationStatus::Valid,
        _ => unreachable!(),
    };
    Ok((i, status))
}

/// # Parse RMC message
///
/// From gpsd:
///
/// `RMC,225446.33,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E,A*68`
///
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
pub fn parse_rmc(sentence: NmeaSentence) -> Result<RmcData, Error> {
    if sentence.message_id != SentenceType::RMC {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::RMC,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_rmc(sentence.data)?.1)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn parse_rmc_v23_all_fields_have_value() {
        let s = parse_nmea_sentence(
            "$GPRMC,225446.33,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E,A*2B",
        )
        .unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x2b);
        let rmc_data = parse_rmc(s).unwrap();
        assert_eq!(
            rmc_data.fix_time.unwrap(),
            NaiveTime::from_hms_milli(22, 54, 46, 330)
        );
        assert_eq!(
            rmc_data.fix_date.unwrap(),
            NaiveDate::from_ymd(1994, 11, 19)
        );

        println!("lat: {}", rmc_data.lat.unwrap());
        assert_relative_eq!(rmc_data.lat.unwrap(), 49.0 + 16.45 / 60.);
        println!(
            "lon: {}, diff {}",
            rmc_data.lon.unwrap(),
            (rmc_data.lon.unwrap() + (123.0 + 11.12 / 60.)).abs()
        );
        assert_relative_eq!(rmc_data.lon.unwrap(), -(123.0 + 11.12 / 60.));
        assert_relative_eq!(rmc_data.speed_over_ground.unwrap(), 0.5);
        assert_relative_eq!(rmc_data.true_course.unwrap(), 54.7);
        assert_relative_eq!(rmc_data.magnetic_variation.unwrap(), 20.3);

        assert_eq!(rmc_data.faa_mode, Some(FaaMode::Autonomous));
        assert_eq!(rmc_data.nav_status, None);
    }

    #[test]
    fn parse_rmc_pre_v23_all_fields_have_value() {
        // only 11 fields pre NMEA v2.3
        let s = parse_nmea_sentence(
            "$GPRMC,225446.33,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E*46",
        )
        .unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x46);
        let RmcData {
            fix_time,
            status_of_fix,
            lat,
            lon,
            speed_over_ground,
            true_course,
            fix_date,
            magnetic_variation,
            faa_mode,
            nav_status,
        } = parse_rmc(s).unwrap();

        assert_eq!(fix_time, Some(NaiveTime::from_hms_milli(22, 54, 46, 330)));
        assert_eq!(status_of_fix, RmcStatusOfFix::Autonomous);

        assert_eq!(fix_date, Some(NaiveDate::from_ymd(1994, 11, 19)));
        println!("lat: {:?}", lat);
        assert_relative_eq!(lat.unwrap(), 49.0 + 16.45 / 60.);

        println!(
            "lon: {}, diff {}",
            lon.unwrap(),
            (lon.unwrap() + (123.0 + 11.12 / 60.)).abs()
        );
        assert_relative_eq!(lon.unwrap(), -(123.0 + 11.12 / 60.));

        assert_relative_eq!(speed_over_ground.unwrap(), 0.5);
        assert_relative_eq!(true_course.unwrap(), 54.7);

        assert_relative_eq!(magnetic_variation.unwrap(), 20.3);

        assert_eq!(faa_mode, None);
        assert_eq!(nav_status, None);
    }

    #[test]
    fn parse_rmc_v23_warning_status_most_fields_empty() {
        let s = parse_nmea_sentence("$GPRMC,,V,,,,,,,,,,N*53").unwrap();
        let rmc = parse_rmc(s).unwrap();
        assert_eq!(
            RmcData {
                fix_time: None,
                fix_date: None,
                status_of_fix: RmcStatusOfFix::Invalid,
                lat: None,
                lon: None,
                speed_over_ground: None,
                true_course: None,
                magnetic_variation: None,
                faa_mode: Some(FaaMode::DataNotValid),
                nav_status: None
            },
            rmc
        );
    }

    #[test]
    fn parse_rmc_v23_missing_true_course_and_magnetic_variation() {
        let gpsd_example = "$GNRMC,001031.00,A,4404.13993,N,12118.86023,W,0.146,,100117,,,A*7B";
        let RmcData {
            fix_time,
            fix_date,
            status_of_fix,
            lat,
            lon,
            speed_over_ground,
            true_course,
            magnetic_variation,
            faa_mode,
            nav_status,
        } = parse_nmea_sentence(gpsd_example)
            .map(parse_rmc)
            .unwrap()
            .unwrap();
        assert_eq!(fix_time.unwrap(), NaiveTime::from_hms_milli(0, 10, 31, 0));
        assert_eq!(fix_date.unwrap(), NaiveDate::from_ymd(2017, 1, 10));
        assert_eq!(status_of_fix, RmcStatusOfFix::Autonomous);
        assert_relative_eq!(lat.unwrap(), (44. + 4.13993 / 60.));
        assert_relative_eq!(lon.unwrap(), -(121. + 18.86023 / 60.));
        assert_relative_eq!(speed_over_ground.unwrap(), 0.146);
        assert_eq!(true_course, None);
        assert_eq!(magnetic_variation, None);
        assert_eq!(faa_mode, Some(FaaMode::Autonomous));
        assert_eq!(nav_status, None);
    }

    #[test]
    fn parse_rmc_v41_full() {
        let rmc_v41 =
            "$GPRMC,225207.376,A,5232.067,N,01325.658,E,038.9,324.5,011122,000.0,W,M,E*7A";
        let RmcData {
            fix_time,
            fix_date,
            status_of_fix,
            lat,
            lon,
            speed_over_ground,
            true_course,
            magnetic_variation,
            faa_mode,
            nav_status,
        } = parse_nmea_sentence(rmc_v41)
            .map(parse_rmc)
            .unwrap()
            .unwrap();
        assert_eq!(fix_time.unwrap(), NaiveTime::from_hms_milli(22, 52, 7, 376));
        assert_eq!(fix_date.unwrap(), NaiveDate::from_ymd(2022, 11, 1));
        assert_eq!(status_of_fix, RmcStatusOfFix::Autonomous);
        assert_relative_eq!(lat.unwrap(), (52. + 32.067 / 60.));
        assert_relative_eq!(lon.unwrap(), (13. + 25.658 / 60.));
        assert_relative_eq!(speed_over_ground.unwrap(), 38.9);
        assert_relative_eq!(true_course.unwrap(), 324.5);
        assert_relative_eq!(magnetic_variation.unwrap(), 0.0);
        assert_eq!(faa_mode, Some(FaaMode::Manual));
        assert_eq!(nav_status, Some(RmcNavigationStatus::Estimated));
    }
}
