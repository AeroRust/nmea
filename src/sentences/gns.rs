use chrono::NaiveTime;
use nom::{
    bytes::complete::{take_until, take_while},
    character::complete::{char, one_of},
    combinator::{map_parser, opt},
    number::complete::float,
    sequence::preceded,
    IResult,
};

use super::{
    parse_faa_modes,
    utils::{number, parse_hms, parse_lat_lon},
    FaaModes,
};
use crate::{parse::NmeaSentence, NmeaError};

#[derive(Debug, PartialEq)]
pub struct GnsData {
    pub fix_time: Option<NaiveTime>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub faa_modes: FaaModes,
    pub nsattelites: u16,
    pub hdop: Option<f32>,
    pub alt: Option<f32>,
    pub geoid_separation: Option<f32>,
    pub nav_status: Option<NavigationStatus>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationStatus {
    Safe,
    Caution,
    Unsafe,
    NotValidForNavigation,
}

/// # Parse GNS message
///
/// Information from gpsd:
///
/// Introduced in NMEA 4.0?
///
/// This mostly duplicates RMC, except for the multi GNSS mode
/// indicator.
///
/// ## Example (Ignore the line break):
/// ```text
/// $GPGNS,224749.00,3333.4268304,N,11153.3538273,W,D,19,0.6,406.110,
///        -26.294,6.0,0138,S,*6A
///```
///
/// 1:  224749.00     UTC HHMMSS.SS.  22:47:49.00
/// 2:  3333.4268304  Latitude DDMM.MMMMM. 33 deg. 33.4268304 min
/// 3:  N             Latitude North
/// 4:  12311.12      Longitude 111 deg. 53.3538273 min
/// 5:  W             Longitude West
/// 6:  D             FAA mode indicator
///                     see faa_mode() for possible mode values
///                     May be one to six characters.
///                       Char 1 = GPS
///                       Char 2 = GLONASS
///                       Char 3 = Galileo
///                       Char 4 = BDS
///                       Char 5 = QZSS
///                       Char 6 = NavIC (IRNSS)
/// 7:  19           Number of Satellites used in solution
/// 8:  0.6          HDOP
/// 9:  406110       MSL Altitude in meters
/// 10: -26.294      Geoid separation in meters
/// 11: 6.0          Age of differential corrections, in seconds
/// 12: 0138         Differential reference station ID
/// 13: S            NMEA 4.1+ Navigation status
///                   S = Safe
///                   C = Caution
///                   U = Unsafe
///                   V = Not valid for navigation
/// 8:   *6A          Mandatory NMEA checksum
pub fn parse_gns(sentence: NmeaSentence) -> Result<GnsData, NmeaError> {
    if sentence.message_id != b"GNS" {
        Err(NmeaError::WrongSentenceHeader {
            expected: b"GNS",
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_gns(sentence.data)?.1)
    }
}

fn do_parse_gns(i: &[u8]) -> IResult<&[u8], GnsData> {
    let (i, fix_time) = opt(parse_hms)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, lat_lon) = parse_lat_lon(i)?;
    let (i, _) = char(',')(i)?;
    let (i, faa_modes) = map_parser(take_until(","), parse_faa_modes)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, nsattelites) = number::<u16>(i)?;
    let (i, _) = char(',')(i)?;
    let (i, hdop) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, alt) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, geoid_separation) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _age_of_diff) = take_until(",")(i)?; // TODO parse age of diff. corr.
    let (i, _) = char(',')(i)?;
    let (i, _station_id) = take_while(|c| c != b',')(i)?;
    let (i, nav_status) = opt(preceded(char(','), one_of("SCUV")))(i)?;
    let nav_status = nav_status.map(|ch| match ch {
        'S' => NavigationStatus::Safe,
        'C' => NavigationStatus::Caution,
        'U' => NavigationStatus::Unsafe,
        'V' => NavigationStatus::NotValidForNavigation,
        _ => unreachable!(),
    });
    Ok((
        i,
        GnsData {
            fix_time,
            lat: lat_lon.map(|x| x.0),
            lon: lat_lon.map(|x| x.1),
            faa_modes,
            nsattelites,
            hdop,
            alt,
            geoid_separation,
            nav_status,
        },
    ))
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_parse_gns() {
        let s = parse_nmea_sentence(b"$GPGNS,224749.00,3333.4268304,N,11153.3538273,W,D,19,0.6,406.110,-26.294,6.0,0138,S,*46").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x46);
        let gns_data = parse_gns(s).unwrap();
        assert_eq!(
            gns_data.fix_time.unwrap(),
            NaiveTime::from_hms_milli(22, 47, 49, 0)
        );
        assert_relative_eq!(33.0 + 33.4268304 / 60., gns_data.lat.unwrap());
        assert_relative_eq!(-(111.0 + 53.3538273 / 60.), gns_data.lon.unwrap());
        assert_eq!(19, gns_data.nsattelites);
        assert_relative_eq!(0.6, gns_data.hdop.unwrap());
        assert_relative_eq!(406.110, gns_data.alt.unwrap());
        assert_relative_eq!(-26.294, gns_data.geoid_separation.unwrap());
        assert_eq!(Some(NavigationStatus::Safe), gns_data.nav_status);
    }
}
