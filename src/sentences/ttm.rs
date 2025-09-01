use chrono::NaiveTime;
use nom::{
    IResult, Parser as _,
    bytes::complete::take_until,
    character::complete::{char, one_of},
    combinator::{map_res, opt},
    error::ErrorKind,
};

use super::utils::{parse_float_num, parse_hms, parse_number_in_range};
use crate::{Error, NmeaSentence, SentenceType};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtmReference {
    Relative,
    Theoretical,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TtmAngle {
    angle: f32,
    reference: TtmReference,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtmDistanceUnit {
    Kilometer,
    NauticalMile,
    StatuteMile,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtmStatus {
    /// Tracked target has been lost
    Lost,
    /// Target in the process of acquisition
    Query,
    /// Target is being tracked
    Tracking,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtmTypeOfAcquisition {
    Automatic,
    Manual,
    Reported,
}

/// TTM - Tracked Target Message
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_ttm_tracked_target_message>
///
/// ```text
///                                         11     13            16
///        1  2   3   4 5   6   7 8   9   10|    12| 14       15  |
///        |  |   |   | |   |   | |   |   | |    | | |         |  |
/// $--TTM,xx,x.x,x.x,a,x.x,x.x,a,x.x,x.x,a,c--c,a,a,hhmmss.ss,a*hh<CR><LF>
/// ```
/// 1. Target Number (0-99)
/// 2. Target Distance
/// 3. Bearing from own ship
/// 4. T = True, R = Relative
/// 5. Target Speed
/// 6. Target Course
/// 7. T = True, R = Relative
/// 8. Distance of closest-point-of-approach
/// 9. Time until closest-point-of-approach "-" means increasing
/// 10. Speed/distance units, K/N
/// 11. Target name
/// 12. Target Status
/// 13. Reference Target
/// 14. UTC of data (NMEA 3 and above) hh is hours, mm is minutes, ss.ss is seconds.
/// 15. Type, A = Auto, M = Manual, R = Reported (NMEA 3 and above)
/// 16. Checksum
///
///
/// Example:
/// ```text
/// $RATTM,01,0.2,190.8,T,12.1,109.7,T,0.1,0.5,N,TGT01,T,,100021.00,A*79
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, PartialEq)]
pub struct TtmData {
    /// Target number
    pub target_number: Option<u8>,
    /// Target distance
    pub target_distance: Option<f32>,
    /// Bearing from own ship
    pub bearing_from_own_ship: Option<TtmAngle>,
    /// Target speed
    pub target_speed: Option<f32>,
    /// Target course
    pub target_course: Option<TtmAngle>,
    /// Distance of closest-point-of-approach
    pub distance_of_cpa: Option<f32>,
    /// Time to closest-point-of-approach
    pub time_to_cpa: Option<f32>,
    /// Unit used for speed and distance
    pub speed_or_distance_unit: Option<TtmDistanceUnit>,
    /// Target name
    pub target_name: Option<heapless::String<32>>,
    /// Target status
    pub target_status: Option<TtmStatus>,
    /// Set to true if target is a reference used to determine own-ship position or velocity
    pub is_target_reference: bool,
    /// Time of data
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub time_of_data: Option<NaiveTime>,
    /// Type of acquisition
    pub type_of_acquisition: Option<TtmTypeOfAcquisition>,
}

/// # Parse TTM message
pub fn parse_ttm(sentence: NmeaSentence<'_>) -> Result<TtmData, Error<'_>> {
    if sentence.message_id != SentenceType::TTM {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::TTM,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_ttm(sentence.data)?.1)
    }
}

fn do_parse_ttm(i: &str) -> IResult<&str, TtmData> {
    let (i, target_number) = opt(|i| parse_number_in_range::<u8>(i, 0, 99)).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, target_distance) = opt(map_res(take_until(","), parse_float_num::<f32>)).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, bearing_from_own_ship) = parse_ttm_angle(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, target_speed) = opt(map_res(take_until(","), parse_float_num::<f32>)).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, target_course) = parse_ttm_angle(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, distance_of_cpa) = opt(map_res(take_until(","), parse_float_num::<f32>)).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, time_to_cpa) = opt(map_res(take_until(","), parse_float_num::<f32>)).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, unit_char) = opt(one_of("KNS")).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let unit = unit_char.map(|unit| match unit {
        'K' => TtmDistanceUnit::Kilometer,
        'N' => TtmDistanceUnit::NauticalMile,
        'S' => TtmDistanceUnit::StatuteMile,
        _ => unreachable!(),
    });

    let (i, target_name) = take_until(",").parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let target_name = if target_name.is_empty() {
        None
    } else {
        Some(heapless::String::try_from(target_name).map_err(|_| {
            nom::Err::Failure(nom::error::Error {
                input: i,
                code: ErrorKind::Fail,
            })
        })?)
    };

    let (i, target_status_char) = opt(one_of("LQT")).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let target_status = target_status_char.map(|char| match char {
        'L' => TtmStatus::Lost,
        'Q' => TtmStatus::Query,
        'T' => TtmStatus::Tracking,
        _ => unreachable!(),
    });

    let (i, is_target_reference_char) = opt(one_of("R")).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let is_target_reference = is_target_reference_char.is_some();

    let (i, time_of_data) = opt(parse_hms).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, type_of_acquisition_char) = opt(one_of("AMR")).parse(i)?;
    let type_of_acquisition = type_of_acquisition_char.map(|char| match char {
        'A' => TtmTypeOfAcquisition::Automatic,
        'M' => TtmTypeOfAcquisition::Manual,
        'R' => TtmTypeOfAcquisition::Reported,
        _ => unreachable!(),
    });

    Ok((
        i,
        TtmData {
            target_number,
            target_distance,
            bearing_from_own_ship,
            target_speed,
            target_course,
            distance_of_cpa,
            time_to_cpa,
            speed_or_distance_unit: unit,
            target_name,
            target_status,
            is_target_reference,
            time_of_data,
            type_of_acquisition,
        },
    ))
}

fn parse_ttm_angle(i: &str) -> IResult<&str, Option<TtmAngle>> {
    let (i, angle) = opt(map_res(take_until(","), parse_float_num::<f32>)).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, reference) = opt(one_of("RT")).parse(i)?;

    Ok((
        i,
        angle.and_then(|angle| {
            reference.map(|reference_char| {
                let reference = match reference_char {
                    'R' => TtmReference::Relative,
                    'T' => TtmReference::Theoretical,
                    _ => unreachable!(),
                };

                TtmAngle { angle, reference }
            })
        }),
    ))
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_parse_ttm_full() {
        let data = parse_ttm(NmeaSentence {
            talker_id: "RA",
            message_id: SentenceType::TTM,
            data: "00,0.5,187.5,T,12.0,17.6,T,0.0,1.2,N,TGT00,T,,100023.00,A",
            checksum: 0x4e,
        })
        .unwrap();
        assert_eq!(data.target_number.unwrap(), 0);
        assert_relative_eq!(data.target_distance.unwrap(), 0.5);

        let bearing_from_own_ship = data.bearing_from_own_ship.unwrap();
        assert_relative_eq!(bearing_from_own_ship.angle, 187.5,);
        assert_eq!(bearing_from_own_ship.reference, TtmReference::Theoretical);

        assert_relative_eq!(data.target_speed.unwrap(), 12.0);

        let target_course = data.target_course.unwrap();
        assert_relative_eq!(target_course.angle, 17.6);
        assert_eq!(target_course.reference, TtmReference::Theoretical);

        assert_relative_eq!(data.distance_of_cpa.unwrap(), 0.0);
        assert_relative_eq!(data.time_to_cpa.unwrap(), 1.2);
        assert_eq!(
            data.speed_or_distance_unit.unwrap(),
            TtmDistanceUnit::NauticalMile
        );
        assert_eq!(data.target_name.unwrap(), "TGT00");
        assert_eq!(data.target_status.unwrap(), TtmStatus::Tracking);
        assert!(!data.is_target_reference);
        assert_eq!(
            data.time_of_data.unwrap(),
            NaiveTime::from_hms_opt(10, 0, 23).unwrap()
        );
        assert_eq!(
            data.type_of_acquisition.unwrap(),
            TtmTypeOfAcquisition::Automatic
        );
    }

    #[test]
    fn test_parse_ttm_all_optional() {
        let s = parse_nmea_sentence("$RATTM,,,,,,,,,,,,,,,*72").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());

        let data = parse_ttm(s);
        assert_eq!(
            data,
            Ok(TtmData {
                target_number: None,
                target_distance: None,
                bearing_from_own_ship: None,
                target_speed: None,
                target_course: None,
                distance_of_cpa: None,
                time_to_cpa: None,
                speed_or_distance_unit: None,
                target_name: None,
                target_status: None,
                is_target_reference: false,
                time_of_data: None,
                type_of_acquisition: None,
            })
        );
    }
}
