use crate::{parse::*, sentences::utils::array_string, Error, SentenceType};

use arrayvec::ArrayString;
use nom::{
    bytes::complete::{is_not, take_until},
    character::complete::char,
    combinator::{map_parser, opt},
    number::complete::float,
    sequence::preceded,
};

const MAX_LEN: usize = 64;

/// BOD - Bearing - Waypoint to Waypoint
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_bod_bearing_waypoint_to_waypoint>
///
/// ```text
///        1   2 3   4 5    6    7
///        |   | |   | |    |    |
/// $--BOD,x.x,T,x.x,M,c--c,c--c*hh<CR><LF>
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodData {
    pub bearing_true: Option<f32>,
    pub bearing_magnetic: Option<f32>,
    pub to_waypoint: Option<ArrayString<MAX_LEN>>,
    pub from_waypoint: Option<ArrayString<MAX_LEN>>,
}

/// BOD - Bearing - Waypoint to Waypoint
///
/// ```text
///        1   2 3   4 5    6    7
///        |   | |   | |    |    |
/// $--BOD,x.x,T,x.x,M,c--c,c--c*hh<CR><LF>
/// ```
fn do_parse_bod(i: &str) -> Result<BodData, Error> {
    // 1. Bearing Degrees, True
    let (i, bearing_true) = opt(map_parser(take_until(","), float))(i)?;
    let (i, _) = char(',')(i)?;

    // 2. T = True
    let (i, _) = char('T')(i)?;
    let (i, _) = char(',')(i)?;

    // 3. Bearing Degrees, Magnetic
    let (i, bearing_magnetic) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;

    // 4. M = Magnetic
    let (i, _) = char('M')(i)?;
    let (i, _) = char(',')(i)?;

    // 5. Destination Waypoint
    let (i, to_waypoint) = opt(is_not(",*"))(i)?;

    // 6. origin Waypoint
    let from_waypoint = opt(preceded(char(','), is_not("*")))(i)?.1;

    // 7. Checksum

    Ok(BodData {
        bearing_true,
        bearing_magnetic,
        to_waypoint: to_waypoint.map(array_string::<MAX_LEN>).transpose()?,
        from_waypoint: from_waypoint.map(array_string::<MAX_LEN>).transpose()?,
    })
}

/// # Parse BOD message
///
/// See: <https://gpsd.gitlab.io/gpsd/NMEA.html#_bod_bearing_waypoint_to_waypoint>
pub fn parse_bod(sentence: NmeaSentence) -> Result<BodData, Error> {
    if sentence.message_id != SentenceType::BOD {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::BOD,
            found: sentence.message_id,
        })
    } else {
        do_parse_bod(sentence.data)
    }
}

#[cfg(test)]
mod tests {

    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn parse_bod_with_route_active_example_full() {
        let sentence = parse_nmea_sentence("$GPBOD,097.0,T,103.2,M,POINTB,POINTA*4A").unwrap();
        assert_eq!(sentence.checksum, sentence.calc_checksum());
        assert_eq!(sentence.checksum, 0x4A);

        let data = parse_bod(sentence).unwrap();
        assert_relative_eq!(data.bearing_true.unwrap(), 97.0);
        assert_relative_eq!(data.bearing_magnetic.unwrap(), 103.2);
        assert_eq!(data.to_waypoint.as_deref(), Some("POINTB"));
        assert_eq!(data.from_waypoint.as_deref(), Some("POINTA"));
    }

    #[test]
    fn parse_bod_with_route_active_missing_destination_waypoint_example_full() {
        let sentence = parse_nmea_sentence("$GPBOD,097.0,T,103.2,M,,POINTA*44").unwrap();
        assert_eq!(sentence.checksum, sentence.calc_checksum());
        assert_eq!(sentence.checksum, 0x44);

        let data = parse_bod(sentence).unwrap();
        assert_relative_eq!(data.bearing_true.unwrap(), 97.0);
        assert_relative_eq!(data.bearing_magnetic.unwrap(), 103.2);
        assert!(data.to_waypoint.is_none());
        assert_eq!(data.from_waypoint.as_deref(), Some("POINTA"));
    }

    #[test]
    fn parse_bod_with_route_active_missing_source_waypoint_example_full() {
        // this is equivalent to the "no route active" test, except here there is a comma with an empty field
        // before the checksum. This is just to make sure parsing is resilient to missing data.
        let sentence = parse_nmea_sentence("$GPBOD,097.0,T,103.2,M,POINTB,*47").unwrap();
        assert_eq!(sentence.checksum, sentence.calc_checksum());
        assert_eq!(sentence.checksum, 0x47);

        let data = parse_bod(sentence).unwrap();
        assert_relative_eq!(data.bearing_true.unwrap(), 97.0);
        assert_relative_eq!(data.bearing_magnetic.unwrap(), 103.2);
        assert_eq!(data.to_waypoint.as_deref(), Some("POINTB"));
        assert!(data.from_waypoint.is_none());
    }

    #[test]
    fn parse_bod_no_route_active_example_full() {
        let sentence = parse_nmea_sentence("$GPBOD,099.3,T,105.6,M,POINTB*64").unwrap();
        assert_eq!(sentence.checksum, sentence.calc_checksum());
        assert_eq!(sentence.checksum, 0x64);
        let data = parse_bod(sentence).unwrap();

        assert_relative_eq!(data.bearing_true.unwrap(), 99.3);
        assert_relative_eq!(data.bearing_magnetic.unwrap(), 105.6);
        assert_eq!(data.to_waypoint.as_deref(), Some("POINTB"));
        assert!(data.from_waypoint.is_none());
    }

    #[test]
    fn parse_bod_no_route_active_no_bearing_example_full() {
        let sentence = parse_nmea_sentence("$GPBOD,,T,105.6,M,POINTB*49").unwrap();
        assert_eq!(sentence.checksum, sentence.calc_checksum());
        assert_eq!(sentence.checksum, 0x49);
        let data = parse_bod(sentence).unwrap();

        assert!(data.bearing_true.is_none());
        assert_relative_eq!(data.bearing_magnetic.unwrap(), 105.6);
        assert_eq!(data.to_waypoint.as_deref(), Some("POINTB"));
        assert!(data.from_waypoint.is_none());
    }
}
