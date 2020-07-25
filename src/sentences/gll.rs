use chrono::NaiveTime;
use nom::bytes::complete::take_until;
use nom::character::complete::{char, one_of};
use nom::combinator::{map, opt};
use nom::sequence::terminated;
use nom::IResult;

use crate::parse::NmeaSentence;
use crate::{NmeaError, sentences::utils::{do_parse_lat_lon, parse_hms}};

/// Parse GPGLL (Geographic position)
/// From https://docs.novatel.com/OEM7/Content/Logs/GPGLL.htm
///
/// | Field | Structure   | Description
/// |-------|-------------|---------------------------------------------------------------------
/// | 1     | $GPGLL      | Log header.
/// | 2     | lat         | Latitude (DDmm.mm)
/// | 3     | lat dir     | Latitude direction (N = North, S = South)
/// | 4     | lon         | Longitude (DDDmm.mm)
/// | 5     | lon dir     | Longitude direction (E = East, W = West)
/// | 6     | utc         | UTC time status of position (hours/minutes/seconds/decimal seconds)
/// | 7     | data status | Data status: A = Data valid, V = Data invalid
/// | 8     | mode ind    | Positioning system mode indicator, see `PosSystemIndicator`
/// | 9     | *xx         | Check sum
pub fn parse_gll(sentence: NmeaSentence) -> Result<GllData, NmeaError> {
    if sentence.message_id != b"GLL" {
        Err(NmeaError::WrongSentenceHeader{expected: b"GLL", found: sentence.message_id})
    } else {
        Ok(do_parse_gll(sentence.data)?.1)
    }
}



/// Positioning System Mode Indicator (present from NMEA >= 2.3)
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PosSystemIndicator {
    Autonomous,
    Differential,
    EstimatedMode,
    ManualInput,
    DataNotValid,
}

impl From<char> for PosSystemIndicator {
    fn from(b: char) -> Self {
        match b {
            'A' => PosSystemIndicator::Autonomous,
            'D' => PosSystemIndicator::Differential,
            'E' => PosSystemIndicator::EstimatedMode,
            'M' => PosSystemIndicator::ManualInput,
            'N' => PosSystemIndicator::DataNotValid,
            _ => PosSystemIndicator::DataNotValid,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct GllData {
    pub latitude: f64,
    pub longitude: f64,
    pub fix_time: NaiveTime,
    pub mode: Option<PosSystemIndicator>,
}

fn do_parse_gll(i: &[u8]) -> IResult<&[u8], GllData> {
    let (i, (latitude, longitude)) = do_parse_lat_lon(i)?;
    let (i, _) = char(',')(i)?;
    let (i, fix_time) = parse_hms(i)?;
    let (i, _) = take_until(",")(i)?; // decimal ignored
    let (i, _) = char(',')(i)?;
    let (i, _valid) = char('A')(i)?; // A: valid, V: invalid
    let (i, _) = char(',')(i)?;
    let (i, mode) = opt(terminated(
        map(one_of("ADEM"), PosSystemIndicator::from), // ignore 'N' for invalid
        char(','),
    ))(i)?;

    Ok((
        i,
        GllData {
            latitude,
            longitude,
            fix_time,
            mode,
        },
    ))
}
