use nom::{character::complete::char, combinator::opt, number::complete::float, IResult};

use crate::{parse::NmeaSentence, Error, SentenceType};

/// MDA - Meterological Composite
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_mda_meteorological_composite>
///
/// ```text
///          1   2  3    4  5  6 7 8  9 10 11 12 13 14 15 16 17 18 19 20 21
///          |   |  |    |  |  | | |  |  |  |  |  |  |  |  |  |  |  |  |  |
///  $--MDA,n.nn,I,n.nnn,B,n.n,C,n.C,n.n,n,n.n,C,n.n,T,n.n,M,n.n,N,n.n,M*hh<CR><LF>
/// ```
#[derive(Debug, PartialEq)]
pub struct MdaData {
    /// Pressure in inches of mercury
    pub pressure_in_hg: Option<f32>,
    /// Pressure in bars
    pub pressure_bar: Option<f32>,
    /// Air temp, deg celsius
    pub air_temp_deg: Option<f32>,
    /// Water temp, deg celsius
    pub water_temp_deg: Option<f32>,
    /// Relative humidity, percent
    pub rel_humidity: Option<f32>,
    /// Absolute humidity, percent
    pub abs_humidity: Option<f32>,
    /// Dew point, degrees celsius
    pub dew_point: Option<f32>,
    /// True Wind Direction, NED degrees
    pub wind_direction_true: Option<f32>,
    /// Magnetic Wind Direction, NED degrees
    pub wind_direction_magnetic: Option<f32>,
    /// Wind speed knots
    pub wind_speed_knots: Option<f32>,
    /// Wind speed meters/second
    pub wind_speed_ms: Option<f32>,
}

/// # Parse MDA message
///
/// Information from mda:
///
/// NMEA 0183 standard Wind Speed and Angle, in relation to the vessel’s bow/centerline.
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_mda_meteorological_composite>
///
/// ## Example (Ignore the line break):
/// ```text
/// $WIMDA,29.7544,I,1.0076,B,35.5,C,17.5,C,42.1,30.6,20.6,C,116.4,T,107.7,M,1.2,N,0.6,M*66
///```
///
///
/// 1: 29.7544     Pressure in inches of mercury
/// 2: I
/// 3: 1.0076      Pressure in bars
/// 4: B
/// 5: 35.5        Air temp, deg celsius
/// 6: C
/// 7: 17.5        Water temp, deg celsius
/// 8: C
/// 9: 42.1        Relative humidity, percent
/// 10: 30.6       Absolute humidity, percent
/// 11: 20.6       Dew point, degrees celsius
/// 12: C
/// 13: 116.4      True Wind Direction, NED degrees
/// 14: T
/// 15: 107.7      Magnetic Wind Direction, NED degrees
/// 16: M
/// 17: 1.2        Wind speed knots
/// 18: N
/// 19: 0.6        Wind speed meters/second
/// 20: M
/// 21: *16        Mandatory NMEA checksum

pub fn parse_mda(sentence: NmeaSentence) -> Result<MdaData, Error> {
    if sentence.message_id != SentenceType::MDA {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::MDA,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_mda(sentence.data)?.1)
    }
}

fn do_parse_mda(i: &str) -> IResult<&str, MdaData> {
    let (i, pressure_in_hg) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = opt(char('I'))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, pressure_bar) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = opt(char('B'))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, air_temp_deg) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = opt(char('C'))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, water_temp_deg) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = opt(char('C'))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, rel_humidity) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, abs_humidity) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, dew_point) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = opt(char('C'))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, wind_direction_true) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = opt(char('T'))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, wind_direction_magnetic) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = opt(char('M'))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, wind_speed_knots) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = opt(char('N'))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, wind_speed_ms) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = opt(char('M'))(i)?;

    Ok((
        i,
        MdaData {
            pressure_in_hg,
            pressure_bar,
            air_temp_deg,
            water_temp_deg,
            rel_humidity,
            abs_humidity,
            dew_point,
            wind_direction_true,
            wind_direction_magnetic,
            wind_speed_knots,
            wind_speed_ms,
        },
    ))
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_parse_mda() {
        // Partial sentence from AirMax 150 model weather station
        let s = parse_nmea_sentence(
            "$WIMDA,29.7544,I,1.0076,B,35.5,C,,,42.1,,20.6,C,116.4,T,107.7,M,1.2,N,0.6,M*66",
        )
        .unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x66);
        let mda_data = parse_mda(s).unwrap();
        assert_relative_eq!(29.7544, mda_data.pressure_in_hg.unwrap());
        assert_relative_eq!(1.0076, mda_data.pressure_bar.unwrap());
        assert_relative_eq!(35.5, mda_data.air_temp_deg.unwrap());
        assert!(mda_data.water_temp_deg.is_none());
        assert_relative_eq!(42.1, mda_data.rel_humidity.unwrap());
        assert!(mda_data.abs_humidity.is_none());
        assert_relative_eq!(20.6, mda_data.dew_point.unwrap());
        assert_relative_eq!(116.4, mda_data.wind_direction_true.unwrap());
        assert_relative_eq!(107.7, mda_data.wind_direction_magnetic.unwrap());
        assert_relative_eq!(1.2, mda_data.wind_speed_knots.unwrap());
        assert_relative_eq!(0.6, mda_data.wind_speed_ms.unwrap());
    }
}
