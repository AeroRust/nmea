use arrayvec::ArrayString;
use nom::{
    bytes::complete::is_not, character::complete::char, combinator::opt, number::complete::float,
};

use crate::{parse::NmeaSentence, Error, SentenceType};

const MAX_LEN: usize = 64;

/// BWW - Bearing - Waypoint to Waypoint
///
/// Bearing calculated at the FROM waypoint.
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_bww_bearing_waypoint_to_waypoint>
///
/// ```text
///        1   2 3   4 5    6    7
///        |   | |   | |    |    |
/// $--BWW,x.x,T,x.x,M,c--c,c--c*hh<CR><LF>
/// ```
/// Field Number:
/// 1. Bearing, degrees True
/// 2. T = True
/// 3. Bearing Degrees, Magnetic
/// 4. M = Magnetic
/// 5. TO Waypoint ID
/// 6. FROM Waypoint ID
/// 7. Checksum

#[derive(Debug, PartialEq)]
pub struct BwwData {
    pub true_bearing: Option<f32>,
    pub magnetic_bearing: Option<f32>,
    pub to_waypoint_id: Option<ArrayString<MAX_LEN>>,
    pub from_waypoint_id: Option<ArrayString<MAX_LEN>>,
}

fn do_parse_bww(i: &str) -> Result<BwwData, Error> {
    // 1. Bearing, degrees True
    let (i, true_bearing) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    // 2. T = True
    let (i, _) = opt(char('T'))(i)?;
    let (i, _) = char(',')(i)?;

    // 3. Bearing, degrees Magnetic
    let (i, magnetic_bearing) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    // 4. M = Magnetic
    let (i, _) = opt(char('M'))(i)?;
    let (i, _) = char(',')(i)?;

    // 5. TO Waypoint ID
    let (i, to_waypoint_id) = opt(is_not(","))(i)?;

    let to_waypoint_id = if let Some(to_waypoint_id) = to_waypoint_id {
        Some(
            ArrayString::from(to_waypoint_id).map_err(|_e| Error::ParameterLength {
                max_length: MAX_LEN,
                parameter_length: to_waypoint_id.len(),
            })?,
        )
    } else {
        None
    };

    // 6. FROM Waypoint ID
    let (i, _) = char(',')(i)?;
    let (_i, from_waypoint_id) = opt(is_not(",*"))(i)?;

    let from_waypoint_id = if let Some(from_waypoint_id) = from_waypoint_id {
        Some(
            ArrayString::from(from_waypoint_id).map_err(|_e| Error::ParameterLength {
                max_length: MAX_LEN,
                parameter_length: from_waypoint_id.len(),
            })?,
        )
    } else {
        None
    };

    Ok(BwwData {
        true_bearing,
        magnetic_bearing,
        to_waypoint_id,
        from_waypoint_id,
    })
}

/// # Parse BWW message
///
/// See: <https://gpsd.gitlab.io/gpsd/NMEA.html#_bww_bearing_waypoint_to_waypoint>
pub fn parse_bww(sentence: NmeaSentence) -> Result<BwwData, Error> {
    if sentence.message_id != SentenceType::BWW {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::BWW,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_bww(sentence.data)?)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_parse_bww_full() {
        let sentence = parse_nmea_sentence("$GPBWW,213.8,T,218.0,M,TOWPT,FROMWPT*42").unwrap();
        assert_eq!(sentence.checksum, sentence.calc_checksum());
        assert_eq!(sentence.checksum, 0x42);

        let data = parse_bww(sentence).unwrap();

        assert_relative_eq!(data.true_bearing.unwrap(), 213.8);
        assert_relative_eq!(data.magnetic_bearing.unwrap(), 218.0);
        assert_eq!(&data.to_waypoint_id.unwrap(), "TOWPT");
        assert_eq!(&data.from_waypoint_id.unwrap(), "FROMWPT");
    }

    #[test]
    fn test_parse_bww_with_optional_fields() {
        let sentence = parse_nmea_sentence("$GPBWW,,T,,M,,*4C").unwrap();
        assert_eq!(sentence.checksum, sentence.calc_checksum());
        assert_eq!(sentence.checksum, 0x4C);

        let data = parse_bww(sentence).unwrap();

        assert_eq!(
            BwwData {
                true_bearing: None,
                magnetic_bearing: None,
                to_waypoint_id: None,
                from_waypoint_id: None,
            },
            data
        );
    }

    #[test]
    fn test_parse_bww_with_wrong_sentence() {
        let sentence = parse_nmea_sentence("$GPAAM,,T,,M,,*4C").unwrap();

        assert_eq!(
            parse_bww(sentence).unwrap_err(),
            Error::WrongSentenceHeader {
                expected: SentenceType::BWW,
                found: SentenceType::AAM,
            }
        );
    }

    #[test]
    fn test_parse_bww_with_too_long_to_waypoint_parameter() {
        let sentence = parse_nmea_sentence("$GPBWW,,T,,M,ABCDEFGHIJKLMNOPQRSTUWXYZABCDEFGHIJKLMNOPQRSTUWXYZABCDEFGHIJKLMNOPQRSTUWXYZ,*4C").unwrap();

        assert_eq!(
            parse_bww(sentence).unwrap_err(),
            Error::ParameterLength {
                max_length: 64,
                parameter_length: 75
            }
        );
    }

    #[test]
    fn test_parse_bww_with_too_long_from_waypoint_parameter() {
        let sentence = parse_nmea_sentence("$GPBWW,,T,,M,,ABCDEFGHIJKLMNOPQRSTUWXYZABCDEFGHIJKLMNOPQRSTUWXYZABCDEFGHIJKLMNOPQRSTUWXYZ*4C").unwrap();

        assert_eq!(
            parse_bww(sentence).unwrap_err(),
            Error::ParameterLength {
                max_length: 64,
                parameter_length: 75
            }
        );
    }
}
