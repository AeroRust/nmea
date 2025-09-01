use chrono::NaiveTime;
use nom::{
    IResult, Parser as _,
    bytes::complete::take_until,
    character::complete::{char, one_of},
    combinator::{map_res, opt},
    number::complete::float,
};

use crate::{
    Error, SentenceType,
    parse::NmeaSentence,
    sentences::{
        FixType,
        utils::{number, parse_float_num, parse_hms, parse_lat_lon},
    },
};

/// GGA - Global Positioning System Fix Data
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_gga_global_positioning_system_fix_data>
///
/// ```text
///                                                       11
///         1         2       3 4        5 6 7  8   9  10 |  12 13  14   15
///         |         |       | |        | | |  |   |   | |   | |   |    |
///  $--GGA,hhmmss.ss,ddmm.mm,a,ddmm.mm,a,x,xx,x.x,x.x,M,x.x,M,x.x,xxxx*hh<CR><LF>
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, PartialEq)]
pub struct GgaData {
    #[cfg_attr(
        not(feature = "std"),
        cfg_attr(feature = "serde", serde(with = "serde_naive_time"))
    )]
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
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
    let (i, fix_time) = opt(parse_hms).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, lat_lon) = parse_lat_lon(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, fix_quality) = one_of("012345678").parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, fix_satellites) = opt(number::<u32>).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, hdop) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, altitude) = opt(map_res(take_until(","), parse_float_num::<f32>)).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, _) = opt(char('M')).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, geoid_height) = opt(map_res(take_until(","), parse_float_num::<f32>)).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, _) = opt(char('M')).parse(i)?;

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
pub fn parse_gga(sentence: NmeaSentence<'_>) -> Result<GgaData, Error<'_>> {
    if sentence.message_id != SentenceType::GGA {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::GGA,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_gga(sentence.data)?.1)
    }
}

#[cfg(not(feature = "std"))]
#[cfg(feature = "serde")]
mod serde_naive_time {
    use super::*;
    use core::fmt::{self, Write};
    use serde::de::Visitor;

    pub fn serialize<S>(v: &Option<NaiveTime>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match v {
            Some(time) => {
                let mut str: heapless::String<32> = heapless::String::new();
                write!(&mut str, "{}", time).map_err(serde::ser::Error::custom)?;
                s.serialize_str(&str)
            }
            None => s.serialize_none(),
        }
    }

    struct NaiveTimeVisitor;

    impl<'de> Visitor<'de> for NaiveTimeVisitor {
        type Value = NaiveTime;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("expecting NaiveTime in format H:M:S.f")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let time =
                NaiveTime::parse_from_str(v, "%H:%M:%S%.f").map_err(serde::de::Error::custom)?;
            return Ok(time);
        }
    }

    struct OptionVisitor;

    impl<'de> Visitor<'de> for OptionVisitor {
        type Value = Option<NaiveTime>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("expecting Option<NaiveTime>")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let time = deserializer
                .deserialize_str(NaiveTimeVisitor)
                .map_err(serde::de::Error::custom)?;
            Ok(Some(time))
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<NaiveTime>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        d.deserialize_option(OptionVisitor)
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
        assert_eq!(
            data.fix_time,
            Some(NaiveTime::from_hms_opt(3, 37, 45).expect("invalid time"))
        );
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

    #[cfg(feature = "serde")]
    #[test]
    fn test_serialize_deserialize_gga_data_with_fix_time_milis() {
        // hhmmss.sss
        let data = parse_gga(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::GGA,
            data: "033745.222,5650.82344,N,03548.9778,E,1,07,1.8,101.2,M,14.7,M,,",
            checksum: 0x57,
        })
        .unwrap();

        assert_eq!(
            data.fix_time,
            Some(NaiveTime::from_hms_milli_opt(3, 37, 45, 222).expect("invalid time"))
        );

        let serialized = serde_json::to_string(&data).unwrap();
        let gga: GgaData = serde_json::from_str(&serialized).unwrap();

        assert_eq!(data.fix_time, gga.fix_time);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serialize_deserialize_gga_data_with_fix_time_nano() {
        // hhmmss.sss
        let data = parse_gga(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::GGA,
            data: "033745.222222222,5650.82344,N,03548.9778,E,1,07,1.8,101.2,M,14.7,M,,",
            checksum: 0x57,
        })
        .unwrap();

        assert_eq!(
            data.fix_time,
            Some(NaiveTime::from_hms_nano_opt(3, 37, 45, 222_222_222).expect("invalid time"))
        );

        let serialized = serde_json::to_string(&data).unwrap();
        let gga: GgaData = serde_json::from_str(&serialized).unwrap();

        assert_eq!(data.fix_time, gga.fix_time);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serialize_deserialize_gga_data_with_fix_time() {
        // hhmmss.sss
        let data = parse_gga(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::GGA,
            data: "033745.000,5650.82344,N,03548.9778,E,1,07,1.8,101.2,M,14.7,M,,",
            checksum: 0x57,
        })
        .unwrap();

        assert_eq!(
            data.fix_time,
            Some(NaiveTime::from_hms_opt(3, 37, 45).expect("invalid time"))
        );

        let serialized = serde_json::to_string(&data).unwrap();
        let gga: GgaData = serde_json::from_str(&serialized).unwrap();

        assert_eq!(data.fix_time, gga.fix_time);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serialize_deserialize_gga_data_without_fix_time() {
        let data = parse_gga(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::GGA,
            data: ",5650.82344,N,03548.9778,E,1,07,1.8,101.2,M,14.7,M,,",
            checksum: 0x57,
        })
        .unwrap();

        assert_eq!(data.fix_time, None);

        let serialized = serde_json::to_string(&data).unwrap();
        let gga: GgaData = serde_json::from_str(&serialized).unwrap();

        assert_eq!(data.fix_time, gga.fix_time);
    }
}
