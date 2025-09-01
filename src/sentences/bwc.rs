use arrayvec::ArrayString;
use chrono::NaiveTime;
use nom::{
    Parser as _, bytes::complete::is_not, character::complete::char, combinator::opt,
    number::complete::float,
};

use crate::{
    Error, SentenceType,
    parse::{NmeaSentence, TEXT_PARAMETER_MAX_LEN},
    sentences::utils::{parse_hms, parse_lat_lon},
};

/// BWC - Bearing & Distance to Waypoint - Great Circle
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_bwc_bearing_distance_to_waypoint_great_circle>
///
/// ```text
///                                                         12
///         1         2       3 4        5 6   7 8   9 10  11|    13 14
///         |         |       | |        | |   | |   | |   | |    |   |
/// $--BWC,hhmmss.ss,llll.ll,a,yyyyy.yy,a,x.x,T,x.x,M,x.x,N,c--c,m,*hh<CR><LF>
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, PartialEq)]
pub struct BwcData {
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub fix_time: Option<NaiveTime>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub true_bearing: Option<f32>,
    pub magnetic_bearing: Option<f32>,
    pub distance: Option<f32>,
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub waypoint_id: Option<ArrayString<TEXT_PARAMETER_MAX_LEN>>,
}

/// BWC - Bearing & Distance to Waypoint - Great Circle
/// ```text
///                                                         12
///         1         2       3 4        5 6   7 8   9 10  11|    13 14
///         |         |       | |        | |   | |   | |   | |    |   |
/// $--BWC,hhmmss.ss,llll.ll,a,yyyyy.yy,a,x.x,T,x.x,M,x.x,N,c--c,m,*hh<CR><LF>
/// ```
fn do_parse_bwc(i: &str) -> Result<BwcData, Error<'_>> {
    // 1. UTC Time or observation
    let (i, fix_time) = opt(parse_hms).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    // 2. Waypoint Latitude
    // 3. N = North, S = South
    // 4. Waypoint Longitude
    // 5. E = East, W = West
    let (i, lat_lon) = parse_lat_lon(i)?;
    let (i, _) = char(',').parse(i)?;

    // 6. Bearing, degrees True
    let (i, true_bearing) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    // 7. T = True
    let (i, _) = opt(char('T')).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    // 8. Bearing, degrees Magnetic
    let (i, magnetic_bearing) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    // 9. M = Magnetic
    let (i, _) = opt(char('M')).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    // 10. Distance, Nautical Miles
    let (i, distance) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    // 11. N = Nautical Miles
    let (i, _) = opt(char('N')).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    // 12. Waypoint ID
    let (_i, waypoint_id) = opt(is_not(",*")).parse(i)?;

    // 13. FAA mode indicator (NMEA 2.3 and later, optional)

    let waypoint_id = if let Some(waypoint_id) = waypoint_id {
        Some(
            ArrayString::from(waypoint_id)
                .map_err(|_e| Error::SentenceLength(waypoint_id.len()))?,
        )
    } else {
        None
    };

    Ok(BwcData {
        fix_time,
        latitude: lat_lon.map(|v| v.0),
        longitude: lat_lon.map(|v| v.1),
        true_bearing,
        magnetic_bearing,
        distance,
        waypoint_id,
    })
}

/// # Parse BWC message
///
/// See: <https://gpsd.gitlab.io/gpsd/NMEA.html#_bwc_bearing_distance_to_waypoint_great_circle>
pub fn parse_bwc(sentence: NmeaSentence<'_>) -> Result<BwcData, Error<'_>> {
    if sentence.message_id != SentenceType::BWC {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::BWC,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_bwc(sentence.data)?)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_parse_bwc_full() {
        let sentence = parse_nmea_sentence(
            "$GPBWC,220516,5130.02,N,00046.34,W,213.8,T,218.0,M,0004.6,N,EGLM*21",
        )
        .unwrap();
        assert_eq!(sentence.checksum, sentence.calc_checksum());
        assert_eq!(sentence.checksum, 0x21);

        let data = parse_bwc(sentence).unwrap();

        assert_eq!(
            data.fix_time,
            Some(NaiveTime::from_hms_opt(22, 5, 16).expect("invalid time"))
        );
        assert_relative_eq!(data.latitude.unwrap(), 51. + 30.02 / 60.);
        assert_relative_eq!(data.longitude.unwrap(), -46.34 / 60.0);
        assert_relative_eq!(data.true_bearing.unwrap(), 213.8);
        assert_relative_eq!(data.magnetic_bearing.unwrap(), 218.0);
        assert_relative_eq!(data.distance.unwrap(), 4.6);
        assert_eq!(&data.waypoint_id.unwrap(), "EGLM");
    }

    #[test]
    fn test_parse_bwc_with_optional_fields() {
        let sentence = parse_nmea_sentence("$GPBWC,081837,,,,,,T,,M,,N,*13").unwrap();
        assert_eq!(sentence.checksum, sentence.calc_checksum());
        assert_eq!(sentence.checksum, 0x13);

        let data = parse_bwc(sentence).unwrap();

        assert_eq!(
            BwcData {
                fix_time: Some(NaiveTime::from_hms_opt(8, 18, 37).expect("invalid time")),
                latitude: None,
                longitude: None,
                true_bearing: None,
                magnetic_bearing: None,
                distance: None,
                waypoint_id: None,
            },
            data
        );
    }
}
