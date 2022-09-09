use chrono::NaiveTime;
use nom::{
    bytes::complete::take_until,
    character::complete::{char, one_of},
    combinator::{map_res, opt},
    number::complete::float,
    IResult,
};

use crate::{
    parse::NmeaSentence,
    sentences::utils::{number, parse_hms, parse_lat_lon},
    NmeaError,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GbsData {
    pub time: Option<NaiveTime>,
    pub lat_error: Option<f64>,
    pub lon_error: Option<f64>,
    pub alt_error: Option<f32>,
    pub most_likely_failed_sat: Option<u8>,
    pub missed_probability: Option<f32>,
    pub bias_estimate: Option<f32>,
    pub bias_standard_deviation: Option<f32>,
}
/// GBS - GPS Satellite Fault Detection

/// ```text
/// 1      2   3   4   5   6   7   8   9
/// |      |   |   |   |   |   |   |   |
/// $--GBS,hhmmss.ss,x.x,x.x,x.x,x.x,x.x,x.x,x.x*hh<CR><LF>
/// ```
fn do_parse_gbs(i: &[u8]) -> IResult<&[u8], GbsData> {
    // 1. UTC time of the GGA or GNS fix associated with this sentence. hh is hours, mm is minutes, ss.ss is seconds
    let (i, time) = opt(parse_hms)(i)?;
    let (i, _) = char(',')(i)?;

    // 2. Expected 1-sigma error in latitude (meters)
    // 3. Expected 1-sigma error in longitude (meters)
    let (i, lat_lon_errors) = parse_lat_lon(i)?;
    let (i, _) = char(',')(i)?;

    // 4. Expected 1-sigma error in altitude (meters)
    let (i, alt_error) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;

    // 5. ID of most likely failed satellite (1 to 138)
    let (i, most_likely_failed_sat) = opt(number::<u8>)(i)?;
    let (i, _) = char(',')(i)?;

    // 6. Probability of missed detection for most likely failed satellite
    let (i, missed_probability) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;

    // 7. Estimate of bias in meters on most likely failed satellite
    let (i, bias_estimate) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    // 8. Standard deviation of bias estimate
    let (i, bias_standard_deviation) = opt(float)(i)?;
    // 9. Checksum

    Ok((
        i,
        GbsData {
            time,
            lat_error: lat_lon_errors.map(|(lat, _lon)| lat),
            lon_error: lat_lon_errors.map(|(_lat, lon)| lon),
            alt_error,
            most_likely_failed_sat,
            missed_probability,
            bias_estimate,
            bias_standard_deviation,
        },
    ))
}

/// # Parse BOD message
///
/// See: <https://gpsd.gitlab.io/gpsd/NMEA.html#_gbs_gps_satellite_fault_detection>
pub fn parse_gbs(sentence: NmeaSentence) -> Result<GbsData, NmeaError> {
    if sentence.message_id != b"GBS" {
        Err(NmeaError::WrongSentenceHeader {
            expected: b"GBS",
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_gbs(sentence.data)?.1)
    }
}
