use nom::{
    character::complete::{char, one_of},
    combinator::opt,
    number::complete::float,
    sequence::preceded,
    IResult,
};

use crate::{parse::NmeaSentence, Error, SentenceType};

/// MWV - Wind Speed and Angle
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_mwv_wind_speed_and_angle>
///
/// ```text
///        1   2 3   4 5
///        |   | |   | |
/// $--MWV,x.x,a,x.x,a*hh<CR><LF>
/// ```
#[derive(Debug, PartialEq)]
pub struct MwvData {
    pub wind_direction: Option<f32>,
    pub reference: Option<MwvReference>,
    pub wind_speed: Option<f32>,
    pub wind_speed_units: Option<MwvWindSpeedUnits>,
    pub data_valid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MwvReference {
    Relative,
    Theoretical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MwvWindSpeedUnits {
    KilometersPerHour,
    MetersPerSecond,
    Knots,
    MilesPerHour,
}

/// # Parse MWV message
///
/// Information from mwv:
///
/// NMEA 0183 standard Wind Speed and Angle, in relation to the vessel’s bow/centerline.
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_mwv_wind_speed_and_angle>
///
/// ## Example (Ignore the line break):
/// ```text
/// $WIMWV,041.1,R,01.0,N,A*16
///```
///
/// 1:  041.1         Wind direction, cardinal NED degrees
/// 2:  R             Relative or Theoretical windspeed
/// 3:  01.0          Wind speed
/// 4:  N             Wind speed units (Knots)
/// 5:  A             Data is OK
/// 6:  *16           Mandatory NMEA checksum
pub fn parse_mwv(sentence: NmeaSentence) -> Result<MwvData, Error> {
    if sentence.message_id != SentenceType::MWV {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::MWV,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_mwv(sentence.data)?.1)
    }
}

fn do_parse_mwv(i: &str) -> IResult<&str, MwvData> {
    let (i, direction) = opt(float)(i)?;
    let (i, reference_type) = opt(preceded(char(','), one_of("RT")))(i)?;
    let reference_type = reference_type.map(|ch| match ch {
        'R' => MwvReference::Relative,
        'T' => MwvReference::Theoretical,
        _ => unreachable!(),
    });
    let (i, _) = char(',')(i)?;
    let (i, speed) = opt(float)(i)?;
    let (i, wind_speed_type) = opt(preceded(char(','), one_of("KMNS")))(i)?;
    let wind_speed_type = wind_speed_type.map(|ch| match ch {
        'K' => MwvWindSpeedUnits::KilometersPerHour,
        'M' => MwvWindSpeedUnits::MetersPerSecond,
        'N' => MwvWindSpeedUnits::Knots,
        'S' => MwvWindSpeedUnits::MilesPerHour,
        _ => unreachable!(),
    });
    let (i, is_data_valid) = preceded(char(','), one_of("AV"))(i)?;
    let is_data_valid = match is_data_valid {
        'A' => true,
        'V' => false,
        _ => unreachable!(),
    };

    Ok((
        i,
        MwvData {
            wind_direction: direction,
            reference: reference_type,
            wind_speed: speed,
            wind_speed_units: wind_speed_type,
            data_valid: is_data_valid,
        },
    ))
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_parse_mwv() {
        let s = parse_nmea_sentence("$WIMWV,041.1,R,01.0,N,A*16").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x16);
        let wimwv_data = parse_mwv(s).unwrap();
        assert_relative_eq!(41.1, wimwv_data.wind_direction.unwrap());
        assert_eq!(MwvReference::Relative, wimwv_data.reference.unwrap());
        assert_relative_eq!(1.0, wimwv_data.wind_speed.unwrap());
        assert_eq!(
            MwvWindSpeedUnits::Knots,
            wimwv_data.wind_speed_units.unwrap()
        );
        assert!(wimwv_data.data_valid);
    }
}
