use chrono::NaiveTime;
use nom::{
    IResult, Parser as _,
    character::complete::{anychar, char, one_of},
    combinator::opt,
};

use super::{FaaMode, faa_mode::parse_faa_mode, nom_parse_failure};
use crate::{
    Error, SentenceType,
    parse::NmeaSentence,
    sentences::utils::{parse_hms, parse_lat_lon},
};

/// GLL - Geographic Position - Latitude/Longitude
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_gll_geographic_position_latitudelongitude>
///
/// ```text
///         1       2 3        4 5         6 7
///         |       | |        | |         | |
///  $--GLL,ddmm.mm,a,dddmm.mm,a,hhmmss.ss,a*hh<CR><LF>
/// ```
///
/// NMEA 2.3:
/// ```text
///         1       2 3        4 5         6 7
///         |       | |        | |         | |
///  $--GLL,ddmm.mm,a,dddmm.mm,a,hhmmss.ss,a,m*hh<CR><LF>
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GllData {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub fix_time: Option<NaiveTime>,
    pub valid: bool,
    pub faa_mode: Option<FaaMode>,
}

/// # Parse GLL (Geographic position) message
///
/// From <https://docs.novatel.com/OEM7/Content/Logs/GPGLL.htm>
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
pub fn parse_gll(sentence: NmeaSentence<'_>) -> Result<GllData, Error<'_>> {
    if sentence.message_id != SentenceType::GLL {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::GLL,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_gll(sentence.data)?.1)
    }
}

fn do_parse_gll(i: &str) -> IResult<&str, GllData> {
    let (i, lat_lon) = parse_lat_lon(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, fix_time) = opt(parse_hms).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, valid) = one_of("AV").parse(i)?; // A: valid, V: invalid
    let valid = match valid {
        'A' => true,
        'V' => false,
        _ => unreachable!(),
    };
    let (i, _) = char(',').parse(i)?;
    let (rest, mode) = opt(anychar).parse(i)?;
    let faa_mode = mode
        .map(|mode| parse_faa_mode(mode).ok_or_else(|| nom_parse_failure(i)))
        .transpose()?;

    Ok((
        rest,
        GllData {
            latitude: lat_lon.map(|x| x.0),
            longitude: lat_lon.map(|x| x.1),
            valid,
            fix_time,
            faa_mode,
        },
    ))
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_parse_gpgll() {
        let parse = |data, checksum| {
            let s = parse_nmea_sentence(data).unwrap();
            assert_eq!(s.checksum, s.calc_checksum());
            assert_eq!(s.checksum, checksum);
            s
        };

        let s = parse(
            "$GPGLL,5107.0013414,N,11402.3279144,W,205412.00,A,A*73",
            0x73,
        );
        let gll_data = parse_gll(s).unwrap();
        assert_relative_eq!(gll_data.latitude.unwrap(), 51.0 + (7.0013414 / 60.0));
        assert_relative_eq!(gll_data.longitude.unwrap(), -(114.0 + (2.3279144 / 60.0)));
        assert_eq!(
            gll_data.fix_time,
            Some(NaiveTime::from_hms_milli_opt(20, 54, 12, 0).expect("invalid time"))
        );
        assert_eq!(gll_data.faa_mode, Some(FaaMode::Autonomous));

        let s = parse("$GNGLL,,,,,181604.00,V,N*5E", 0x5e);
        let gll_data = parse_gll(s).unwrap();
        assert_eq!(
            Some(NaiveTime::from_hms_milli_opt(18, 16, 4, 0).expect("invalid time")),
            gll_data.fix_time
        );
        assert!(!gll_data.valid);

        let s = parse("$GNGLL,,,,,,V,N*7A", 0x7a);
        let gll_data = parse_gll(s).unwrap();
        assert_eq!(gll_data.fix_time, None);
        assert!(!gll_data.valid);
    }
}
