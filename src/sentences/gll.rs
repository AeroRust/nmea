use chrono::NaiveTime;
use nom::bytes::complete::take;
use nom::character::complete::{char, one_of};
use nom::combinator::opt;
use nom::IResult;

use crate::parse::NmeaSentence;
use crate::{
    sentences::utils::{parse_hms, parse_lat_lon},
    NmeaError,
};

use super::{nom_parse_failure, parse_faa_mode, FaaMode};

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
        Err(NmeaError::WrongSentenceHeader {
            expected: b"GLL",
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_gll(sentence.data)?.1)
    }
}

#[derive(Debug, PartialEq)]
pub struct GllData {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub fix_time: NaiveTime,
    pub valid: bool,
    pub faa_mode: Option<FaaMode>,
}

fn do_parse_gll(i: &[u8]) -> IResult<&[u8], GllData> {
    let (i, lat_lon) = parse_lat_lon(i)?;
    let (i, _) = char(',')(i)?;
    let (i, fix_time) = parse_hms(i)?;
    let (i, _) = char(',')(i)?;
    let (i, valid) = one_of("AV")(i)?; // A: valid, V: invalid
    let valid = match valid {
        'A' => true,
        'V' => false,
        _ => unreachable!(),
    };
    let (i, _) = char(',')(i)?;
    let (rest, mode) = opt(take(1usize))(i)?;
    let mut faa_mode = None;
    if let Some(mode) = mode {
        match parse_faa_mode(mode[0]) {
            Some(x) => faa_mode = Some(x),
            None => return Err(nom_parse_failure(i)),
        }
    }

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
    use super::*;
    use crate::parse::parse_nmea_sentence;
    use approx::assert_relative_eq;

    #[test]
    fn test_parse_gpgll() {
        let parse = |data, checksum| {
            let s = parse_nmea_sentence(data).unwrap();
            assert_eq!(s.checksum, s.calc_checksum());
            assert_eq!(s.checksum, checksum);
            s
        };

        let s = parse(
            b"$GPGLL,5107.0013414,N,11402.3279144,W,205412.00,A,A*73",
            0x73,
        );
        let gll_data = parse_gll(s).unwrap();
        assert_relative_eq!(gll_data.latitude.unwrap(), 51.0 + (7.0013414 / 60.0));
        assert_relative_eq!(gll_data.longitude.unwrap(), -(114.0 + (2.3279144 / 60.0)));
        assert_eq!(gll_data.fix_time, NaiveTime::from_hms_milli(20, 54, 12, 0));
        assert_eq!(gll_data.faa_mode, Some(FaaMode::Autonomous));

        let s = parse(b"$GNGLL,,,,,181604.00,V,N*5E", 0x5e);
        let gll_data = parse_gll(s).unwrap();
        assert_eq!(NaiveTime::from_hms_milli(18, 16, 4, 0), gll_data.fix_time);
        assert!(!gll_data.valid);
    }
}
