use nom::{
    character::complete::{char, one_of},
    combinator::opt,
    number::complete::double,
    sequence::preceded,
    IResult,
};

use crate::{parse::NmeaSentence, Error, SentenceType};

/// MTW - Mean Temperature of Water
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_mtw_mean_temperature_of_water>
///
/// ```text
///        1   2 3
///        |   | |
/// $--MTW,x.x,C*hh<CR><LF>
/// ```
/// 1:  Temperature, degrees
/// 2:  Unit of Measurement, (only) Celsius
/// 3:  Mandatory NMEA checksum
#[derive(Debug, PartialEq)]
pub struct MtwData {
    pub temperature: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MtwUnit {
    Celsius,
}

/// # Parse MTW message
///
/// Information from mtw:
///
/// NMEA 0183 standard Mean Temperature of Water.
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_mtw_mean_temperature_of_water>
///
/// ## Example (Ignore the line break):
/// ```text
/// $INMTW,17.9,C*1B
///```
///
/// 1:  17.9         Temperature, degrees
/// 2:  C            Unit of Measurement, (only) Celsius
/// 3:  *16          Mandatory NMEA checksum
pub fn parse_mtw(sentence: NmeaSentence) -> Result<MtwData, Error> {
    if sentence.message_id != SentenceType::MTW {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::MTW,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_mtw(sentence.data)?.1)
    }
}

fn do_parse_mtw(i: &str) -> IResult<&str, MtwData> {
    let (i, temperature_value) = opt(double)(i)?;
    preceded(char(','), one_of("C"))(i)?;
    Ok((
        i,
        MtwData {
            temperature: temperature_value,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_parse_mtw() {
        let s = parse_nmea_sentence("$INMTW,17.9,C*1B").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x1B);
        let mtw_data = parse_mtw(s).unwrap();
        assert_eq!(Some(17.9), mtw_data.temperature);
    }

    #[test]
    fn test_parse_mtw_invalid_unit() {
        let s = parse_nmea_sentence("$INMTW,17.9,x*20").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x20);
        assert_eq!(true, parse_mtw(s).is_err());
    }

    #[test]
    fn test_parse_mtw_invalid_temp() {
        let s = parse_nmea_sentence("$INMTW,x.9,C*65").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x65);
        assert_eq!(true, parse_mtw(s).is_err());
    }
}
