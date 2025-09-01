use arrayvec::ArrayString;
use nom::{
    Parser as _, bytes::complete::is_not, character::complete::char, combinator::opt,
    number::complete::float,
};

use super::utils::array_string;
use crate::{
    Error, SentenceType,
    parse::{NmeaSentence, TEXT_PARAMETER_MAX_LEN},
};

/// WNC - Distance - Waypoint to Waypoint
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_wnc_distance_waypoint_to_waypoint>
///
/// Example of WNC sentences:
/// - $GPWNC,200.00,N,370.40,K,Dest,Origin*58
///
/// ```text  
///             1  2  3  4   5    6  
///             |  |  |  |   |    |  
///     $--WNC,x.x,N,x.x,K,c--c,c--c*hh  
/// ```
///
/// Key:
/// 1. Distance, Nautical Miles
/// 2. N = Nautical Miles
/// 3. Distance, Kilometers
/// 4. K = Kilometers
/// 5. Waypoint ID, Destination
/// 6. Waypoint ID, Origin
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct WncData {
    /// Distance, Nautical Miles
    pub distance_nautical_miles: Option<f32>,
    /// Distance, Kilometers
    pub distance_kilometers: Option<f32>,
    /// Waypoint ID, Destination
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub waypoint_id_destination: Option<ArrayString<TEXT_PARAMETER_MAX_LEN>>,
    /// Waypoint ID, Origin
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub waypoint_id_origin: Option<ArrayString<TEXT_PARAMETER_MAX_LEN>>,
}

pub fn do_parse_wnc(i: &str) -> Result<WncData, Error<'_>> {
    let (i, distance_nautical_miles) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, _) = opt(char('N')).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, distance_kilometers) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, _) = opt(char('K')).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, waypoint_id_destination) = opt(is_not(",")).parse(i)?;
    let waypoint_id_destination = waypoint_id_destination
        .map(array_string::<TEXT_PARAMETER_MAX_LEN>)
        .transpose()?;
    let (i, _) = char(',').parse(i)?;
    let (_i, waypoint_id_origin) = opt(is_not(",")).parse(i)?;
    let waypoint_id_origin = waypoint_id_origin
        .map(array_string::<TEXT_PARAMETER_MAX_LEN>)
        .transpose()?;

    Ok(WncData {
        distance_nautical_miles,
        distance_kilometers,
        waypoint_id_destination,
        waypoint_id_origin,
    })
}

pub fn parse_wnc(sentence: NmeaSentence<'_>) -> Result<WncData, Error<'_>> {
    if sentence.message_id != SentenceType::WNC {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::WNC,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_wnc(sentence.data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Error, parse::parse_nmea_sentence};
    use approx::assert_relative_eq;

    fn run_parse_wnc(line: &str) -> Result<WncData, Error<'_>> {
        let s = parse_nmea_sentence(line).expect("WNC sentence initial parse failed");
        assert_eq!(s.checksum, s.calc_checksum());
        parse_wnc(s)
    }

    #[test]
    fn test_parse_wnc() {
        let sentence = parse_nmea_sentence("$GPWNC,200.00,N,370.40,K,Dest,Origin*58").unwrap();
        assert_eq!(sentence.checksum, sentence.calc_checksum());
        assert_eq!(sentence.checksum, 0x58);

        let data = run_parse_wnc("$GPWNC,200.00,N,370.40,K,Dest,Origin*58").unwrap();
        assert_relative_eq!(data.distance_nautical_miles.unwrap(), 200.00);
        assert_relative_eq!(data.distance_kilometers.unwrap(), 370.40);
        assert_eq!(data.waypoint_id_destination.as_deref(), Some("Dest"));
        assert_eq!(data.waypoint_id_origin.as_deref(), Some("Origin"));
    }
}
