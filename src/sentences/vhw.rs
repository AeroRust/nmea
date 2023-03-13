use nom::{
    bytes::complete::take_until,
    character::streaming::char,
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
/// ```
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

fn do_parse_vhw(i: &str) -> IResult<&str, VhwData> {
    let comma = char(',');

    let (i, heading_true) = opt(map_res(take_until(","), parse_float_num))(i)?;
    let (i, _) = comma(i)?;
    let (i, _) = char('T')(i)?;
    let (i, _) = comma(i)?;

    let (i, heading_magnetic) = opt(map_res(take_until(","), parse_float_num))(i)?;
    let (i, _) = comma(i)?;
    let (i, _) = char('M')(i)?;
    let (i, _) = comma(i)?;

    let (i, relative_speed_knots) = opt(map_res(take_until(","), parse_float_num))(i)?;
    let (i, _) = comma(i)?;
    let (i, _) = char('N')(i)?;
    let (i, _) = comma(i)?;

    let (i, relative_speed_kmph) = opt(map_res(take_until(","), parse_float_num))(i)?;
    let (i, _) = comma(i)?;
    let (i, _) = char('K')(i)?;

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