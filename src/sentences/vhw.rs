use nom::{
    bytes::complete::take_until,
    character::complete::char,
    combinator::{map_res, opt},
    IResult,
};

use crate::{Error, NmeaSentence, SentenceType};

use super::utils::parse_float_num;

/// VHW - Water speed and heading
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_vhw_water_speed_and_heading>
///
/// ```text
///        1   2 3   4 5   6 7   8 9
///        |   | |   | |   | |   | |
/// $--VHW,x.x,T,x.x,M,x.x,N,x.x,K*hh<CR><LF>
/// ```
/// 1. Heading degrees, True
/// 2. T = True
/// 3. Heading degrees, Magnetic
/// 4. M = Magnetic
/// 5. Speed of vessel relative to the water, knots
/// 6. N = Knots
/// 7. Speed of vessel relative to the water, km/hr
/// 8. K = Kilometers
/// 9. Checksum
///
/// Note that this implementation follows the documentation published by `gpsd`, but the GLOBALSAT documentation may have conflicting definitions.
/// > [[GLOBALSAT](https://gpsd.gitlab.io/gpsd/NMEA.html#GLOBALSAT)] describes a different format in which the first three fields are water-temperature measurements.
/// > It’s not clear which is correct.
#[derive(Clone, PartialEq, Debug)]
pub struct VhwData {
    /// Heading degrees, True
    pub heading_true: Option<f64>,
    /// Heading degrees, Magnetic
    pub heading_magnetic: Option<f64>,
    /// Speed of vessel relative to the water, knots
    pub relative_speed_knots: Option<f64>,
    /// Speed of vessel relative to the water, km/hr
    pub relative_speed_kmph: Option<f64>,
}

/// # Parse VHW message
///
/// ```text
/// $GPVHW,100.5,T,105.5,M,10.5,N,19.4,K*4F
/// ```
/// 1. 100.5 Heading True
/// 2. T
/// 3. 105.5 Heading Magnetic
/// 4. M
/// 5. 10.5 Speed relative to water, knots
/// 6. N
/// 7. 19.4 Speed relative to water, km/hr
/// 8. K
///
/// Each is considered as a pair of a float value and a single character,
/// and if the float value exists but the single character is not correct, it is treated as `None`.
/// For example, if 1 is "100.5" and 2 is not "T", then heading_true is `None`.
pub fn parse_vhw(sentence: NmeaSentence) -> Result<VhwData, Error> {
    if sentence.message_id == SentenceType::VHW {
        Ok(do_parse_vhw(sentence.data)?.1)
    } else {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::VHW,
            found: sentence.message_id,
        })
    }
}

/// Parses a float value
/// and returns `None` if the float value can be parsed but the next field does not match the specified character.
fn do_parse_float_with_char(c: char, i: &str) -> IResult<&str, Option<f64>> {
    let (i, value) = opt(map_res(take_until(","), parse_float_num::<f64>))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, tag) = opt(char(c))(i)?;
    Ok((i, tag.and(value)))
}

fn do_parse_vhw(i: &str) -> IResult<&str, VhwData> {
    let comma = char(',');

    let (i, heading_true) = do_parse_float_with_char('T', i)?;
    let (i, _) = comma(i)?;

    let (i, heading_magnetic) = do_parse_float_with_char('M', i)?;
    let (i, _) = comma(i)?;

    let (i, relative_speed_knots) = do_parse_float_with_char('N', i)?;
    let (i, _) = comma(i)?;

    let (i, relative_speed_kmph) = do_parse_float_with_char('K', i)?;

    Ok((
        i,
        VhwData {
            heading_true,
            heading_magnetic,
            relative_speed_knots,
            relative_speed_kmph,
        },
    ))
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_do_parse_float_with_char() {
        assert_eq!(do_parse_float_with_char('T', "1.5,T"), Ok(("", Some(1.5))));
        assert_eq!(do_parse_float_with_char('T', "1.5,"), Ok(("", None)));
        assert_eq!(do_parse_float_with_char('T', ","), Ok(("", None)));
    }

    #[test]
    fn test_wrong_sentence() {
        let invalid_aam_sentence = NmeaSentence {
            message_id: SentenceType::AAM,
            data: "",
            talker_id: "GP",
            checksum: 0,
        };
        assert_eq!(
            Err(Error::WrongSentenceHeader {
                expected: SentenceType::VHW,
                found: SentenceType::AAM
            }),
            parse_vhw(invalid_aam_sentence)
        );
    }

    #[test]
    fn test_parse_vhw() {
        let s = NmeaSentence {
            message_id: SentenceType::VHW,
            talker_id: "GP",
            data: "100.5,T,105.5,M,10.5,N,19.4,K",
            checksum: 0x4f,
        };
        let vhw_data = parse_vhw(s).unwrap();
        assert_relative_eq!(vhw_data.heading_true.unwrap(), 100.5);
        assert_relative_eq!(vhw_data.heading_magnetic.unwrap(), 105.5);
        assert_relative_eq!(vhw_data.relative_speed_knots.unwrap(), 10.5);
        assert_relative_eq!(vhw_data.relative_speed_kmph.unwrap(), 19.4);

        let s = parse_nmea_sentence("$GPVHW,100.5,T,105.5,M,10.5,N,19.4,K*4F").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x4F);

        let vhw_data = parse_vhw(s).unwrap();
        assert_relative_eq!(vhw_data.heading_true.unwrap(), 100.5);
        assert_relative_eq!(vhw_data.heading_magnetic.unwrap(), 105.5);
        assert_relative_eq!(vhw_data.relative_speed_knots.unwrap(), 10.5);
        assert_relative_eq!(vhw_data.relative_speed_kmph.unwrap(), 19.4);
    }

    #[test]
    fn test_parse_incomplete_vhw() {
        // Pattern with all single letter alphabetical fields filled, but all numeric fields blank.
        let s = NmeaSentence {
            message_id: SentenceType::VHW,
            talker_id: "GP",
            data: ",T,,M,,N,,K",
            checksum: 0,
        };
        assert_eq!(
            parse_vhw(s),
            Ok(VhwData {
                heading_true: None,
                heading_magnetic: None,
                relative_speed_knots: None,
                relative_speed_kmph: None,
            })
        );

        // Pattern with all single letter alphabetical fields filled and some numerical fields filled.
        let s = NmeaSentence {
            message_id: SentenceType::VHW,
            talker_id: "GP",
            data: ",T,,M,10.5,N,20.0,K",
            checksum: 0,
        };
        assert_eq!(
            parse_vhw(s),
            Ok(VhwData {
                heading_true: None,
                heading_magnetic: None,
                relative_speed_knots: Some(10.5),
                relative_speed_kmph: Some(20.0),
            })
        );

        // Pattern with all fields missing
        let s = NmeaSentence {
            message_id: SentenceType::VHW,
            talker_id: "GP",
            data: ",,,,,,,",
            checksum: 0,
        };
        assert_eq!(
            parse_vhw(s),
            Ok(VhwData {
                heading_true: None,
                heading_magnetic: None,
                relative_speed_knots: None,
                relative_speed_kmph: None
            })
        );
    }
}
