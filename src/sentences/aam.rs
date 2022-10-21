use arrayvec::ArrayString;
use nom::{
    bytes::complete::is_not,
    character::complete::{anychar, char},
    combinator::opt,
    number::complete::float,
};

use crate::{parse::NmeaSentence, sentences::utils::array_string, Error, SentenceType};

const MAX_LEN: usize = 64;

/// AAM - Waypoint Arrival Alarm
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_aam_waypoint_arrival_alarm>
///
/// ```text
///        1 2 3   4 5    6
///        | | |   | |    |
/// $--AAM,A,A,x.x,N,c--c*hh<CR><LF>
/// ```
#[derive(Debug, PartialEq)]
pub struct AamData {
    pub arrival_circle_entered: Option<bool>,
    pub perpendicular_passed: Option<bool>,
    pub arrival_circle_radius: Option<f32>,
    pub radius_units: Option<char>,
    pub waypoint_id: Option<ArrayString<MAX_LEN>>,
}

/// Parse AAM message
pub fn parse_aam(sentence: NmeaSentence) -> Result<AamData, Error> {
    if sentence.message_id != SentenceType::AAM {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::AAM,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_aam(sentence.data)?)
    }
}

fn do_parse_aam(i: &str) -> Result<AamData, Error> {
    let (i, field1) = anychar(i)?;
    let arrival_circle_entered = if field1 == 'A' {
        Some(true)
    } else {
        Some(false)
    };
    let (i, _) = char(',')(i)?;

    let (i, field2) = anychar(i)?;
    let perpendicular_passed = if field2 == 'A' {
        Some(true)
    } else {
        Some(false)
    };
    let (i, _) = char(',')(i)?;

    let (i, arrival_circle_radius) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;

    let (i, radius_units) = opt(anychar)(i)?;
    let (i, _) = char(',')(i)?;

    let (_i, waypoint_id) = opt(is_not("\n"))(i)?;

    Ok(AamData {
        arrival_circle_entered,
        perpendicular_passed,
        arrival_circle_radius,
        radius_units,
        waypoint_id: waypoint_id.map(array_string::<MAX_LEN>).transpose()?,
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::{parse::parse_nmea_sentence, SentenceType};

    #[test]
    fn parse_aam_with_nmea_sentence_struct() {
        let data = parse_aam(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::AAM,
            data: "A,V,0.10,N,WPTNME",
            checksum: 0x0,
        })
        .unwrap();

        assert!(data.arrival_circle_entered.unwrap());
        assert!(!data.perpendicular_passed.unwrap());
        assert_relative_eq!(data.arrival_circle_radius.unwrap(), 0.10);
        assert_eq!(data.radius_units.unwrap(), 'N');
        assert_eq!(&data.waypoint_id.unwrap(), "WPTNME");
    }

    #[test]
    fn parse_aam_full_sentence() {
        let sentence = parse_nmea_sentence("$GPAAM,A,A,0.10,N,WPTNME*32").unwrap();
        assert_eq!(sentence.checksum, 0x32);
        assert_eq!(sentence.calc_checksum(), 0x32);

        let data = parse_aam(sentence).unwrap();
        assert!(data.arrival_circle_entered.unwrap());
        assert!(data.perpendicular_passed.unwrap());
        assert_relative_eq!(data.arrival_circle_radius.unwrap(), 0.10);
        assert_eq!(data.radius_units.unwrap(), 'N');
        assert_eq!(&data.waypoint_id.unwrap(), "WPTNME");
    }

    #[test]
    fn parse_aam_with_wrong_message_id() {
        let error = parse_aam(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::ABK,
            data: "A,V,0.10,N,WPTNME",
            checksum: 0x43,
        })
        .unwrap_err();

        if let Error::WrongSentenceHeader { expected, found } = error {
            assert_eq!(expected, SentenceType::AAM);
            assert_eq!(found, SentenceType::ABK);
        }
    }
}
