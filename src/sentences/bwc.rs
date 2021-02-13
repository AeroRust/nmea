use arrayvec::ArrayString;
use chrono::NaiveTime;
use nom::bytes::complete::is_not;
use nom::character::complete::char;
use nom::combinator::{map_res, opt};
use nom::number::complete::float;
use nom::IResult;

use crate::parse::NmeaSentence;
use crate::sentences::utils::{parse_hms, parse_lat_lon};
use crate::NmeaError;

const MAX_LEN: usize = 64;

#[derive(Debug, PartialEq)]
pub struct BwcData {
    pub fix_time: Option<NaiveTime>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub true_bearing: Option<f32>,
    pub magnetic_bearing: Option<f32>,
    pub distance: Option<f32>,
    pub waypoint_id: Option<ArrayString<[u8; MAX_LEN]>>,
}

fn do_parse_bwc(i: &[u8]) -> Result<BwcData, NmeaError> {
    /*
    BWC - Bearing & Distance to Waypoint - Great Circle
                                                            12
           1         2       3 4        5 6   7 8   9 10  11|    13 14
           |         |       | |        | |   | |   | |   | |    |   |
    $--BWC,hhmmss.ss,llll.ll,a,yyyyy.yy,a,x.x,T,x.x,M,x.x,N,c--c,m,*hh<CR><LF>
    */

    // 1. UTC Time or observation
    let (i, fix_time) = opt(parse_hms)(i)?;
    let (i, _) = char(',')(i)?;

    // 2. Waypoint Latitude
    // 3. N = North, S = South
    // 4. Waypoint Longitude
    // 5. E = East, W = West
    let (i, lat_lon) = parse_lat_lon(i)?;
    let (i, _) = char(',')(i)?;

    // 6. Bearing, degrees True
    let (i, true_bearing) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    // 7. T = True
    let (i, _) = opt(char('T'))(i)?;
    let (i, _) = char(',')(i)?;

    // 8. Bearing, degrees Magnetic
    let (i, magnetic_bearing) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    // 9. M = Magnetic
    let (i, _) = opt(char('M'))(i)?;
    let (i, _) = char(',')(i)?;

    // 10. Distance, Nautical Miles
    let (i, distance) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    // 11. N = Nautical Miles
    let (i, _) = opt(char('N'))(i)?;
    let (i, _) = char(',')(i)?;

    // 12. Waypoint ID
    let (i, waypoint_id) = opt(map_res(is_not(",*"), std::str::from_utf8))(i)?;

    // 13. FAA mode indicator (NMEA 2.3 and later, optional)

    let waypoint_id = if let Some(waypoint_id) = waypoint_id {
        Some(
            ArrayString::from(waypoint_id)
                .map_err(|_e| NmeaError::SentenceLength(waypoint_id.len()))?,
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

/// Parse BWC message
/// See: https://gpsd.gitlab.io/gpsd/NMEA.html#_bwc_bearing_distance_to_waypoint_great_circle
pub fn parse_bwc(sentence: NmeaSentence) -> Result<BwcData, NmeaError> {
    if sentence.message_id != b"BWC" {
        Err(NmeaError::WrongSentenceHeader {
            expected: b"BWC",
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_bwc(sentence.data)?)
    }
}

#[cfg(test)]
mod tests {
    use approx::relative_eq;

    use crate::parse::parse_nmea_sentence;

    use super::*;

    #[test]
    fn test_parse_bwc_full() {
        let sentence = parse_nmea_sentence(
            b"$GPBWC,220516,5130.02,N,00046.34,W,213.8,T,218.0,M,0004.6,N,EGLM*21",
        )
        .unwrap();
        assert_eq!(sentence.checksum, sentence.calc_checksum());
        assert_eq!(sentence.checksum, 0x21);

        let data = parse_bwc(sentence).unwrap();

        assert_eq!(data.fix_time.unwrap(), NaiveTime::from_hms(22, 5, 16));
        relative_eq!(data.latitude.unwrap(), 51. + 30.02 / 60.);
        relative_eq!(data.longitude.unwrap(), 46.34 / 60.);
        relative_eq!(data.true_bearing.unwrap(), 213.8);
        relative_eq!(data.magnetic_bearing.unwrap(), 218.0);
        relative_eq!(data.distance.unwrap(), 4.6);
        assert_eq!(&data.waypoint_id.unwrap(), "EGLM");
    }

    #[test]
    fn test_parse_bwc_with_optional_fields() {
        let sentence = parse_nmea_sentence(b"$GPBWC,081837,,,,,,T,,M,,N,*13").unwrap();
        assert_eq!(sentence.checksum, sentence.calc_checksum());
        assert_eq!(sentence.checksum, 0x13);

        let data = parse_bwc(sentence).unwrap();

        assert_eq!(
            BwcData {
                fix_time: Some(NaiveTime::from_hms(8, 18, 37)),
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
