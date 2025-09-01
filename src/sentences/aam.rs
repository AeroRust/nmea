use crate::parse::TEXT_PARAMETER_MAX_LEN;

use arrayvec::ArrayString;
use nom::{
    Parser as _,
    bytes::complete::is_not,
    character::complete::{char, one_of},
    combinator::opt,
    number::complete::float,
};

use crate::{Error, SentenceType, parse::NmeaSentence, sentences::utils::array_string};

/// AAM - Waypoint Arrival Alarm
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_aam_waypoint_arrival_alarm>
///
/// ```text
///        1 2 3   4 5    6
///        | | |   | |    |
/// $--AAM,A,A,x.x,N,c--c*hh<CR><LF>
///
/// Field Number:
///   1. Status, BOOLEAN, A = Arrival circle entered, V = not passed
///   2. Status, BOOLEAN, A = perpendicular passed at waypoint, V = not passed
///   3. Arrival circle radius
///   4. Units of radius, nautical miles
///   5. Waypoint ID
///   6. Checksum
///
/// Example: $GPAAM,A,A,0.10,N,WPTNME*43
/// WPTNME is the waypoint name.
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, PartialEq)]
pub struct AamData {
    pub arrival_circle_entered: Option<bool>,
    pub perpendicular_passed: Option<bool>,
    pub arrival_circle_radius: Option<f32>,
    pub radius_units: Option<char>,
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub waypoint_id: Option<ArrayString<TEXT_PARAMETER_MAX_LEN>>,
}

/// Parse AAM message
pub fn parse_aam(sentence: NmeaSentence<'_>) -> Result<AamData, Error<'_>> {
    if sentence.message_id != SentenceType::AAM {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::AAM,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_aam(sentence.data)?)
    }
}

fn do_parse_aam(i: &str) -> Result<AamData, Error<'_>> {
    let (i, arrival_circle_entered) = one_of("AV").parse(i)?;
    let arrival_circle_entered = match arrival_circle_entered {
        'A' => Some(true),
        'V' => Some(false),
        _ => unreachable!(),
    };
    let (i, _) = char(',').parse(i)?;

    let (i, perpendicular_passed) = one_of("AV").parse(i)?;
    let perpendicular_passed = match perpendicular_passed {
        'A' => Some(true),
        'V' => Some(false),
        _ => unreachable!(),
    };
    let (i, _) = char(',').parse(i)?;

    let (i, arrival_circle_radius) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, radius_units) = opt(char('N')).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (_i, waypoint_id) = opt(is_not("*")).parse(i)?;

    Ok(AamData {
        arrival_circle_entered,
        perpendicular_passed,
        arrival_circle_radius,
        radius_units,
        waypoint_id: waypoint_id
            .map(array_string::<TEXT_PARAMETER_MAX_LEN>)
            .transpose()?,
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::{SentenceType, parse::parse_nmea_sentence};

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
    #[should_panic]
    fn parse_aam_with_invalid_arrival_circle_entered_value() {
        parse_aam(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::AAM,
            data: "G,V,0.10,N,WPTNME",
            checksum: 0x0,
        })
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn parse_aam_with_invalid_perpendicular_passed_value() {
        parse_aam(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::AAM,
            data: "V,X,0.10,N,WPTNME",
            checksum: 0x0,
        })
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn parse_aam_with_invalid_radius_units_value() {
        parse_aam(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::AAM,
            data: "V,A,0.10,P,WPTNME",
            checksum: 0x0,
        })
        .unwrap();
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
