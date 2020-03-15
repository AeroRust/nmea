use nom::character::complete::char;
use nom::combinator::opt;
use nom::number::complete::float;
use nom::IResult;

use crate::parse::NmeaSentence;

#[derive(Debug, PartialEq)]
pub struct VtgData {
    pub true_course: Option<f32>,
    pub speed_over_ground: Option<f32>,
}

fn do_parse_vtg(i: &[u8]) -> IResult<&[u8], VtgData> {
    let (i, true_course) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = opt(char('T'))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _magn_course) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = opt(char('M'))(i)?;
    let (i, _) = char(',')(i)?;
    let (i, knots_ground_speed) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = opt(char('N'))(i)?;
    let (i, kph_ground_speed) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = opt(char('K'))(i)?;

    Ok((
        i,
        VtgData {
            true_course,
            speed_over_ground: match (knots_ground_speed, kph_ground_speed) {
                (Some(val), _) => Some(val),
                (_, Some(val)) => Some(val / 1.852),
                (None, None) => None,
            },
        },
    ))
}

/// parse VTG
/// from http://aprs.gids.nl/nmea/#vtg
/// Track Made Good and Ground Speed.
///
/// eg1. $GPVTG,360.0,T,348.7,M,000.0,N,000.0,K*43
/// eg2. $GPVTG,054.7,T,034.4,M,005.5,N,010.2,K
///
///
/// 054.7,T      True track made good
/// 034.4,M      Magnetic track made good
/// 005.5,N      Ground speed, knots
/// 010.2,K      Ground speed, Kilometers per hour
///
///
/// eg3. $GPVTG,t,T,,,s.ss,N,s.ss,K*hh
/// 1    = Track made good
/// 2    = Fixed text 'T' indicates that track made good is relative to true north
/// 3    = not used
/// 4    = not used
/// 5    = Speed over ground in knots
/// 6    = Fixed text 'N' indicates that speed over ground in in knots
/// 7    = Speed over ground in kilometers/hour
/// 8    = Fixed text 'K' indicates that speed over ground is in kilometers/hour
/// 9    = Checksum
/// The actual track made good and speed relative to the ground.
///
/// $--VTG,x.x,T,x.x,M,x.x,N,x.x,K
/// x.x,T = Track, degrees True
/// x.x,M = Track, degrees Magnetic
/// x.x,N = Speed, knots
/// x.x,K = Speed, Km/hr
pub fn parse_vtg(s: &NmeaSentence) -> Result<VtgData, String> {
    if s.message_id != b"VTG" {
        return Err("VTG message should starts with $..VTG".into());
    }
    let ret: VtgData = do_parse_vtg(s.data)
        .map(|(_, data)| data)
        .map_err(|err| match err {
            nom::Err::Incomplete(_) => "Incomplete nmea sentence".to_string(),
            nom::Err::Error((_, kind)) | nom::Err::Failure((_, kind)) => {
                kind.description().to_string()
            }
        })?;
    Ok(ret)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_parse_vtg() {
        let run_parse_vtg = |line: &str| -> Result<VtgData, String> {
            let s =
                parse_nmea_sentence(line.as_bytes()).expect("VTG sentence initial parse failed");
            assert_eq!(s.checksum, s.calc_checksum());
            parse_vtg(&s)
        };
        assert_eq!(
            VtgData {
                true_course: None,
                speed_over_ground: None,
            },
            run_parse_vtg("$GPVTG,,T,,M,,N,,K,N*2C").unwrap()
        );
        assert_eq!(
            VtgData {
                true_course: Some(360.),
                speed_over_ground: Some(0.),
            },
            run_parse_vtg("$GPVTG,360.0,T,348.7,M,000.0,N,000.0,K*43").unwrap()
        );
        assert_eq!(
            VtgData {
                true_course: Some(54.7),
                speed_over_ground: Some(5.5),
            },
            run_parse_vtg("$GPVTG,054.7,T,034.4,M,005.5,N,010.2,K*48").unwrap()
        );
    }
}
