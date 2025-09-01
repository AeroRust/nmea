use nom::{
    IResult, Parser as _,
    bytes::complete::take_until,
    character::complete::char,
    combinator::{map_res, opt},
};

use super::utils::parse_float_num;
use crate::{Error, NmeaSentence, SentenceType};

/// HDT - Heading - True
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_hdt_heading_true>
///
/// ```text
///        1   2 3
///        |   | |
/// $--HDT,x.x,T*hh<CR><LF>
/// ```
/// 1. Heading, degrees True
/// 2. T = True
/// 3. Checksum
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, PartialEq)]
pub struct HdtData {
    /// Heading, degrees True
    pub heading: Option<f32>,
}

/// # Parse HDT message
///
/// From gpsd/driver_nmea0183.c
///
/// ```text
/// $HEHDT,341.8,T*21
///
/// HDT,x.x*hh<cr><lf>
/// ```
///
/// The only data field is true heading in degrees.
/// The following field is required to be 'T' indicating a true heading.
/// It is followed by a mandatory nmea_checksum.
pub fn parse_hdt(sentence: NmeaSentence<'_>) -> Result<HdtData, Error<'_>> {
    if sentence.message_id != SentenceType::HDT {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::HDT,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_hdt(sentence.data)?.1)
    }
}

fn do_parse_hdt(i: &str) -> IResult<&str, HdtData> {
    let (i, heading) = opt(map_res(take_until(","), parse_float_num::<f32>)).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, _) = char('T').parse(i)?;
    Ok((i, HdtData { heading }))
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_parse_hdt_full() {
        let data = parse_hdt(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::HDT,
            data: "274.07,T",
            checksum: 0x03,
        })
        .unwrap();
        assert_relative_eq!(data.heading.unwrap(), 274.07);

        let s = parse_nmea_sentence("$GPHDT,,T*1B").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());

        let data = parse_hdt(s);
        assert_eq!(data, Ok(HdtData { heading: None }));
    }
}
