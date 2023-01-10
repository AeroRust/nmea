use core::{fmt::Debug, ops::RangeInclusive, str};
use nom::{
    character::{
        complete::{char, hex_digit0},
        streaming::hex_digit1,
    },
    combinator::{map_res, opt},
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
///  4. GPS Week Number (range 0 to 2^13 - 1), where:
///     - 0 is the week of the GPS Week Number epoch on January 6th 1980;
///     - 8191 is the week that precedes the next rollover on January 6th 2137;
///     Note: the legacy representation started at the same epoch, but
///           the number is 10-bit wide only, with a rollover every 19.7 years.
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
    pub total_number_of_messages: Option<u16>,
    pub sentence_number: Option<u16>,
    pub satellite_prn_number: Option<u8>,
    /// This is the modern 13-bit representation of the GPS week number.
    /// Use [`AlmData::get_10bit_gps_week_number()`] to get the legacy 10-bit
    /// representation.
    pub gps_week_number: Option<u16>,
    pub sv_health: Option<u8>,
    pub eccentricity: Option<u16>,
    pub almanac_reference_time: Option<u8>,
    pub inclination_angle: Option<u16>,
    pub rate_of_right_ascension: Option<u16>,
    pub root_of_semi_major_axis: Option<u32>,
    pub argument_of_perigee: Option<u32>,
    pub longitude_of_ascension_node: Option<u32>,
    pub mean_anomaly: Option<u32>,
    pub f0_clock_parameter: Option<u16>,
    pub f1_clock_parameter: Option<u16>,
}

impl AlmData {
    /// Returns the 10-bit representation of the GPS week number (range 0 to 1023)
    pub fn get_10bit_gps_week_number(&self) -> Option<u16> {
        self.gps_week_number.map(|n| n % 1024)
    }
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

fn number_in_range<T>(i: &str, range: RangeInclusive<T>) -> IResult<&str, T>
where
    T: str::FromStr + PartialOrd + Debug,
{
    map_res(number::<T>, |number_str| {
        if range.contains(&number_str) {
            return Ok(number_str);
        }
        Err(format!(
            "Number {:?} not in expected range {:?}",
            number_str, range
        ))
    })(i)
}

fn do_parse_alm(i: &str) -> IResult<&str, AlmData> {
    // 1. Total number of messages
    let (i, total_number_of_messages) = opt(number)(i)?;
    let (i, _) = char(',')(i)?;

    // 2. Sentence number
    let (i, sentence_number) = opt(number)(i)?;
    let (i, _) = char(',')(i)?;

    //  3. Satellite PRN number (01 to 32)
    let (i, satellite_prn_number) = opt(|i| number_in_range::<u8>(i, 1u8..=32))(i)?;
    let (i, _) = char(',')(i)?;

    //  4. GPS Week Number (0 to 8191)
    let (i, gps_week_number) = opt(|i| number_in_range::<u16>(i, 0u16..=8191))(i)?;
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

#[cfg(test)]
mod tests {
    use crate::{parse_nmea_sentence, sentences::parse_alm};

    #[test]
    fn test() {
        let total_number_of_messages = 31;
        let sentence_number = 1;
        let satellite_prn_number = 2;
        let gps_week_number = 1617;
        let sv_health = 0x00;
        let eccentricity = 0x50f6;
        let almanac_reference_time = 0x0f;
        let inclination_angle = 0xfd98;
        let rate_of_right_ascension = 0xfd39;
        let root_of_semi_major_axis = 0xa10cf3;
        let argument_of_perigee = 0x81389b;
        let longitude_of_ascension_node = 0x423632;
        let mean_anomaly = 0xbd913c;
        let f0_clock_parameter = 0x148;
        let f1_clock_parameter = 0x001;

        let sentence_string =
            "$GPALM,31,1,02,1617,00,50F6,0F,FD98,FD39,A10CF3,81389B,423632,BD913C,148,001*3C";

        let sentence = parse_nmea_sentence(sentence_string).unwrap();
        assert_eq!(sentence.checksum, sentence.calc_checksum());
        assert_eq!(sentence.checksum, 0x3C);

        let data = parse_alm(sentence).unwrap();
        assert_eq!(
            total_number_of_messages,
            data.total_number_of_messages.unwrap()
        );
        assert_eq!(sentence_number, data.sentence_number.unwrap());
        assert_eq!(satellite_prn_number, data.satellite_prn_number.unwrap());
        assert_eq!(gps_week_number, data.gps_week_number.unwrap());
        assert_eq!(sv_health, data.sv_health.unwrap());
        assert_eq!(eccentricity, data.eccentricity.unwrap());
        assert_eq!(almanac_reference_time, data.almanac_reference_time.unwrap());
        assert_eq!(inclination_angle, data.inclination_angle.unwrap());
        assert_eq!(
            rate_of_right_ascension,
            data.rate_of_right_ascension.unwrap()
        );
        assert_eq!(
            root_of_semi_major_axis,
            data.root_of_semi_major_axis.unwrap()
        );
        assert_eq!(argument_of_perigee, data.argument_of_perigee.unwrap());
        assert_eq!(
            longitude_of_ascension_node,
            data.longitude_of_ascension_node.unwrap()
        );
        assert_eq!(mean_anomaly, data.mean_anomaly.unwrap());
        assert_eq!(f0_clock_parameter, data.f0_clock_parameter.unwrap());
        assert_eq!(f1_clock_parameter, data.f1_clock_parameter.unwrap());
    }
}
