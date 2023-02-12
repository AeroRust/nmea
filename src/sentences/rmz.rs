use nom::{
    character::complete::{char, one_of},
    IResult,
};

use crate::sentences::utils::number;
use crate::{parse::NmeaSentence, Error, SentenceType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PgrmzFixType {
    NoFix,
    TwoDimensional,
    ThreeDimensional,
}

/// PGRMZ - Garmin Altitude
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_pgrmz_garmin_altitude>
///
/// ```text
///          1  2 3  4
///          |  | |  |
///  $PGRMZ,hhh,f,M*hh<CR><LF>
/// ```
///
/// 1. Current Altitude Feet
/// 2. `f` = feet
/// 3. Mode (`1` = no fix, `2` = 2D fix, `3` = 3D fix)
/// 4. Checksum
///
/// Example: `$PGRMZ,2282,f,3*21`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PgrmzData {
    /// Current altitude in feet
    pub altitude: u32,
    pub fix_type: PgrmzFixType,
}

fn do_parse_pgrmz(i: &str) -> IResult<&str, PgrmzData> {
    let (i, altitude) = number::<u32>(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = char('f')(i)?;
    let (i, _) = char(',')(i)?;
    let (i, fix_type) = one_of("123")(i)?;
    let fix_type = match fix_type {
        '1' => PgrmzFixType::NoFix,
        '2' => PgrmzFixType::TwoDimensional,
        '3' => PgrmzFixType::ThreeDimensional,
        _ => unreachable!(),
    };
    Ok((i, PgrmzData { altitude, fix_type }))
}

/// # Parse PGRMZ message
///
/// Example:
///
/// `$PGRMZ,2282,f,3*21`
pub fn parse_pgrmz(sentence: NmeaSentence) -> Result<PgrmzData, Error> {
    if sentence.message_id != SentenceType::RMZ {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::RMZ,
            found: sentence.message_id,
        })
    } else if sentence.talker_id != "PG" {
        Err(Error::UnknownTalkerId {
            expected: "PG",
            found: sentence.talker_id,
        })
    } else {
        Ok(do_parse_pgrmz(sentence.data)?.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_successful_parse() {
        let s = parse_nmea_sentence("$PGRMZ,2282,f,3*21").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x21);

        let data = parse_pgrmz(s).unwrap();
        assert_eq!(data.altitude, 2282);
        assert_eq!(data.fix_type, PgrmzFixType::ThreeDimensional);
    }

    #[test]
    fn test_wrong_talker_id() {
        let s = parse_nmea_sentence("$XXRMZ,2282,f,3*21").unwrap();
        assert!(matches!(
            parse_pgrmz(s),
            Err(Error::UnknownTalkerId {
                expected: "PG",
                found: "XX"
            })
        ));
    }
}
