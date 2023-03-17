use nom::{
    character::complete::{char, one_of},
    combinator::opt,
    number::complete::double,
    sequence::preceded,
    IResult,
};

use crate::{parse::NmeaSentence, Error, SentenceType};

/// DBK - Depth Below Keel
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_dbk_depth_below_keel>
///
/// ```text
///        1   2 3   4 5   6 7
///        |   | |   | |   | |
/// $--DBK,x.x,f,x.x,M,x.x,F*hh<CR><LF>
/// ```
/// 1:    Depth, feet
/// 2:    f = feet
/// 3:    Depth, meters
/// 4:    M = meters
/// 5:    Depth, Fathoms
/// 6:    F = Fathoms
/// 7:    Mandatory NMEA checksum
#[derive(Debug, PartialEq)]
pub struct DbkData {
    pub depth_feet: Option<f64>,
    pub depth_meters: Option<f64>,
    pub depth_fathoms: Option<f64>,
}

/// # Parse DBK message
///
/// Information from DBK:
///
/// NMEA 0183 standard Depth Below Keel. (Obsolete sentence)
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_dbk_depth_below_keel>
///
/// ## Example (Ignore the line break):
/// ```text
/// $SDDBK,1330.5,f,0405.5,M,0221.6,F*2E
///```
///
/// 1:    1330.5 Depth feet
/// 2:    f      Units: f = feet
/// 3:    0405.5 Depth meters
/// 4:    M      Units: M = meters
/// 5:    0221.6 Depth Fathoms
/// 6:    F      Units: F = Fathoms
/// 7:    2E     CRC Checksum of NMEA data
pub fn parse_dbk(sentence: NmeaSentence) -> Result<DbkData, Error> {
    if sentence.message_id != SentenceType::DBK {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::DBK,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_dbk(sentence.data)?.1)
    }
}

fn do_parse_dbk(i: &str) -> IResult<&str, DbkData> {
    let (i, depth_feet_value) = opt(double)(i)?;
    let (i, _) = preceded(char(','), one_of("f"))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, depth_meters_value) = opt(double)(i)?;
    let (i, _) = preceded(char(','), one_of("M"))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, depth_fathoms_value) = opt(double)(i)?;
    let (i, _) = preceded(char(','), one_of("F"))(i)?;
    Ok((
        i,
        DbkData {
            depth_feet: depth_feet_value,
            depth_meters: depth_meters_value,
            depth_fathoms: depth_fathoms_value,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_parse_dbk() {
        let s = parse_nmea_sentence("$SDDBK,1330.5,f,0405.5,M,0221.6,F*2E").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x2E);
        let dbk_data = parse_dbk(s).unwrap();
        assert_eq!(Some(1330.5), dbk_data.depth_feet);
        assert_eq!(Some(0405.5), dbk_data.depth_meters);
        assert_eq!(Some(0221.6), dbk_data.depth_fathoms);
    }
    #[test]
    fn test_parse_dbk_invalid_depth_feet_value() {
        let s = parse_nmea_sentence("$SDDBK,1FF0.5,f,0405.5,M,0221.6,F*2E").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x2E);
        assert_eq!(true, parse_dbk(s).is_err());
    }

    #[test]
    fn test_parse_dbk_invalid_depth_feet_unit() {
        let s = parse_nmea_sentence("$SDDBK,1330.5,X,0405.5,M,0221.6,F*10").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x10);
        assert_eq!(true, parse_dbk(s).is_err());
    }

    #[test]
    fn test_parse_dbk_invalid_depth_meters_value() {
        let s = parse_nmea_sentence("$SDDBK,1330.5,f,04F5.5,M,0221.6,F*58").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x58);
        assert_eq!(true, parse_dbk(s).is_err());
    }

    #[test]
    fn test_parse_dbk_invalid_depth_meters_unit() {
        let s = parse_nmea_sentence("$SDDBK,1330.5,f,0405.5,X,0221.6,F*3B").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x3B);
        assert_eq!(true, parse_dbk(s).is_err());
    }

    #[test]
    fn test_parse_dbk_invalid_depth_fathoms_value() {
        let s = parse_nmea_sentence("$SDDBK,1330.5,f,0405.5,M,02F1.6,F*5A").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x5A);
        assert_eq!(true, parse_dbk(s).is_err());
    }

    #[test]
    fn test_parse_dbk_invalid_depth_fathoms_unit() {
        let s = parse_nmea_sentence("$SDDBK,1330.5,f,0405.5,M,0221.6,X*30").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x30);
        assert_eq!(true, parse_dbk(s).is_err());
    }

    #[test]
    fn test_parse_dbk_invalid_sentence_type() {
        let s = parse_nmea_sentence("$INMTW,17.9,x*20").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x20);
        assert_eq!(true, parse_dbk(s).is_err());
    }
}
