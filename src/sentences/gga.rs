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
    sentences::utils::{number, parse_float_num, parse_hms, parse_lat_lon},
    Error, FixType, SentenceType,
};

#[derive(Debug, PartialEq)]
pub struct GgaData {
    pub fix_time: Option<NaiveTime>,
    pub fix_type: Option<FixType>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub fix_satellites: Option<u32>,
    pub hdop: Option<f32>,
    pub altitude: Option<f32>,
    pub geoid_separation: Option<f32>,
}

fn do_parse_gga(i: &str) -> IResult<&str, GgaData> {
    let (i, fix_time) = opt(parse_hms)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, lat_lon) = parse_lat_lon(i)?;
    let (i, _) = char(',')(i)?;
    let (i, fix_quality) = one_of("012345678")(i)?;
    let (i, _) = char(',')(i)?;
    let (i, fix_satellites) = opt(number::<u32>)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, hdop) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, altitude) = opt(map_res(take_until(","), parse_float_num::<f32>))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = opt(char('M'))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, geoid_height) = opt(map_res(take_until(","), parse_float_num::<f32>))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = opt(char('M'))(i)?;

    Ok((
        i,
        GgaData {
            fix_time,
            fix_type: Some(FixType::from(fix_quality)),
            latitude: lat_lon.map(|v| v.0),
            longitude: lat_lon.map(|v| v.1),
            fix_satellites,
            hdop,
            altitude,
            geoid_separation: geoid_height,
        },
    ))
}

/// # Parse GGA message
///
/// From gpsd/driver_nmea0183.c
///
/// `GGA,123519,4807.038,N,01131.324,E,1,08,0.9,545.4,M,46.9,M, , *42`
/// 1     123519       Fix taken at 12:35:19 UTC
/// 2,3   4807.038,N   Latitude 48 deg 07.038' N
/// 4,5   01131.324,E  Longitude 11 deg 31.324' E
/// 6         1            Fix quality: 0 = invalid, 1 = GPS, 2 = DGPS,
/// 3=PPS (Precise Position Service),
/// 4=RTK (Real Time Kinematic) with fixed integers,
/// 5=Float RTK, 6=Estimated, 7=Manual, 8=Simulator
/// 7     08       Number of satellites being tracked
/// 8     0.9              Horizontal dilution of position
/// 9,10  545.4,M      Altitude, Metres above mean sea level
/// 11,12 46.9,M       Height of geoid (mean sea level) above WGS84
/// ellipsoid, in Meters
/// (empty field) time in seconds since last DGPS update
/// (empty field) DGPS station ID number (0000-1023)
pub fn parse_gga(sentence: NmeaSentence) -> Result<GgaData, Error> {
    if sentence.message_id != SentenceType::GGA {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::GGA,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_gga(sentence.data)?.1)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_parse_gga_full() {
        let data = parse_gga(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::GGA,
            data: "033745.0,5650.82344,N,03548.9778,E,1,07,1.8,101.2,M,14.7,M,,",
            checksum: 0x57,
        })
        .unwrap();
        assert_eq!(data.fix_time.unwrap(), NaiveTime::from_hms(3, 37, 45));
        assert_eq!(data.fix_type.unwrap(), FixType::Gps);
        assert_relative_eq!(data.latitude.unwrap(), 56. + 50.82344 / 60.);
        assert_relative_eq!(data.longitude.unwrap(), 35. + 48.9778 / 60.);
        assert_eq!(data.fix_satellites.unwrap(), 7);
        assert_relative_eq!(data.hdop.unwrap(), 1.8);
        assert_relative_eq!(data.altitude.unwrap(), 101.2);
        assert_relative_eq!(data.geoid_separation.unwrap(), 14.7);

        let s = parse_nmea_sentence("$GPGGA,,,,,,0,,,,,,,,*66").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        let data = parse_gga(s).unwrap();
        assert_eq!(
            GgaData {
                fix_time: None,
                fix_type: Some(FixType::Invalid),
                latitude: None,
                longitude: None,
                fix_satellites: None,
                hdop: None,
                altitude: None,
                geoid_separation: None,
            },
            data
        );
    }

    #[test]
    fn test_parse_gga_with_optional_fields() {
        let sentence =
            parse_nmea_sentence("$GPGGA,133605.0,5521.75946,N,03731.93769,E,0,00,,,M,,M,,*4F")
                .unwrap();
        assert_eq!(sentence.checksum, sentence.calc_checksum());
        assert_eq!(sentence.checksum, 0x4f);
        let data = parse_gga(sentence).unwrap();
        assert_eq!(data.fix_type.unwrap(), FixType::Invalid);
    }
}
