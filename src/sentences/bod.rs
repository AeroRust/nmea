use crate::{parse::*, sentences::utils::array_string, Error, SentenceType};

use arrayvec::ArrayString;
use nom::{
    bytes::complete::take_until,
    character::complete::char,
    combinator::{map_parser, opt},
    number::complete::float,
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
    let (i, to_waypoint) = opt(take_until(","))(i)?;
    let (i, _) = char(',')(i)?;

    // 6. origin Waypoint
    let (_i, from_waypoint) = opt(take_until("*"))(i)?;

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
        Ok(do_parse_bod(sentence.data)?)
    }
}
