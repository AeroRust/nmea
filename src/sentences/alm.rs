use core::{fmt::Debug, ops::RangeInclusive, str};
use nom::{
    character::{
        complete::{char, hex_digit0},
        streaming::hex_digit1,
    },
    combinator::{map_res, opt},
    number::complete::float,
    IResult,
};

use crate::{Error, NmeaSentence, SentenceType};

use super::utils::number;

/// ALM - GPS Almanac Data
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_alm_gps_almanac_data>
/// ```text
///         1   2   3  4   5  6    7  8    9    10     11     12     13     14  15   16
///         |   |   |  |   |  |    |  |    |    |      |      |      |      |   |    |
///  $--ALM,x.x,x.x,xx,x.x,hh,hhhh,hh,hhhh,hhhh,hhhhhh,hhhhhh,hhhhhh,hhhhhh,hhh,hhh*hh<CR><LF>
/// ```
///  
///  Field Number:
///  
///  1. Total number of messages
///  2. Sentence Number
///  3. Satellite PRN number (01 to 32)
///  4. GPS Week Number
///  5. SV health, bits 17-24 of each almanac page
///  6. Eccentricity
///  7. Almanac Reference Time
///  8. Inclination Angle
///  9. Rate of Right Ascension
/// 10. Root of semi-major axis
/// 11. Argument of perigee
/// 12. Longitude of ascension node
/// 13. Mean anomaly
/// 14. F0 Clock Parameter
/// 15. F1 Clock Parameter
/// 16. Checksum
///  
///  Fields 5 through 15 are dumped as raw hex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlmData {
    total_number_of_messages: Option<f32>,
    sentence_number: Option<f32>,
    satellite_prn_number: Option<u8>,
    gps_week_number: Option<f32>,
    sv_health: Option<u8>,
    eccentricity: Option<u16>,
    almanac_reference_time: Option<u8>,
    inclination_angle: Option<u16>,
    rate_of_right_ascension: Option<u16>,
    root_of_semi_major_axis: Option<u32>,
    argument_of_perigee: Option<u32>,
    longitude_of_ascension_node: Option<u32>,
    mean_anomaly: Option<u32>,
    f0_clock_parameter: Option<u16>,
    f1_clock_parameter: Option<u16>,
}

pub fn parse_alm(sentence: NmeaSentence) -> Result<AlmData, Error> {
    if sentence.message_id != SentenceType::ALM {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::ALM,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_alm(sentence.data)?.1)
    }
}

fn number_in_range<T>(i: &str, r: RangeInclusive<T>) -> IResult<&str, T>
where
    T: str::FromStr + PartialOrd + Debug,
{
    map_res(number::<T>, |i| {
        if r.contains(&i) {
            return Ok(i);
        }
        Err(format!("Number {i:?} not in expected range {r:?}"))
    })(i)
}

fn do_parse_alm(i: &str) -> IResult<&str, AlmData> {
    // 1. Total number of messages
    let (i, total_number_of_messages) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;

    // 2. Sentence number
    let (i, sentence_number) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;

    //  3. Satellite PRN number (01 to 32)
    let (i, satellite_prn_number) = opt(|d| number_in_range::<u8>(d, 1u8..=32))(i)?;
    let (i, _) = char(',')(i)?;

    //  4. GPS Week Number
    let (i, gps_week_number) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;

    //  5. SV health, bits 17-24 of each almanac page
    let (i, sv_health) = opt(map_res(hex_digit1, |s| u8::from_str_radix(s, 16)))(i)?;
    let (i, _) = char(',')(i)?;

    //  6. Eccentricity
    let (i, eccentricity) = opt(map_res(hex_digit1, |s| u16::from_str_radix(s, 16)))(i)?;
    let (i, _) = char(',')(i)?;

    //  7. Almanac Reference Time
    let (i, almanac_reference_time) = opt(map_res(hex_digit1, |s| u8::from_str_radix(s, 16)))(i)?;
    let (i, _) = char(',')(i)?;

    //  8. Inclination Angle
    let (i, inclination_angle) = opt(map_res(hex_digit1, |s| u16::from_str_radix(s, 16)))(i)?;
    let (i, _) = char(',')(i)?;
    //  9. Rate of Right Ascension
    let (i, rate_of_right_ascension) = opt(map_res(hex_digit1, |s| u16::from_str_radix(s, 16)))(i)?;
    let (i, _) = char(',')(i)?;
    // 10. Root of semi-major axis
    let (i, root_of_semi_major_axis) = opt(map_res(hex_digit1, |s| u32::from_str_radix(s, 16)))(i)?;
    let (i, _) = char(',')(i)?;
    // 11. Argument of perigee
    let (i, argument_of_perigee) = opt(map_res(hex_digit1, |s| u32::from_str_radix(s, 16)))(i)?;
    let (i, _) = char(',')(i)?;
    // 12. Longitude of ascension node
    let (i, longitude_of_ascension_node) =
        opt(map_res(hex_digit1, |s| u32::from_str_radix(s, 16)))(i)?;
    let (i, _) = char(',')(i)?;
    // 13. Mean anomaly
    let (i, mean_anomaly) = opt(map_res(hex_digit1, |s| u32::from_str_radix(s, 16)))(i)?;
    let (i, _) = char(',')(i)?;
    // 14. F0 Clock Parameter
    let (i, f0_clock_parameter) = opt(map_res(hex_digit0, |s| u16::from_str_radix(s, 16)))(i)?;
    let (i, _) = char(',')(i)?;
    // 15. F1 Clock Parameter
    let (i, f1_clock_parameter) = opt(map_res(hex_digit0, |s| u16::from_str_radix(s, 16)))(i)?;

    Ok((
        i,
        AlmData {
            total_number_of_messages,
            sentence_number,
            satellite_prn_number,
            gps_week_number,
            sv_health,
            eccentricity,
            almanac_reference_time,
            inclination_angle,
            rate_of_right_ascension,
            root_of_semi_major_axis,
            argument_of_perigee,
            longitude_of_ascension_node,
            mean_anomaly,
            f0_clock_parameter,
            f1_clock_parameter,
        },
    ))
}
