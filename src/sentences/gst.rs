use crate::{Error, SentenceType, parse::NmeaSentence, sentences::utils::parse_hms};
use chrono::NaiveTime;
use nom::{
    IResult, Parser as _, character::complete::char, combinator::opt, number::complete::float,
};

/// GST - GPS Pseudorange Noise Statistics
/// ```text
///              1    2 3 4 5 6 7 8   9
///              |    | | | | | | |   |
/// $ --GST,hhmmss.ss,x,x,x,x,x,x,x*hh<CR><LF>
/// ```
/// Example: `$GPGST,182141.000,15.5,15.3,7.2,21.8,0.9,0.5,0.8*54`
///
/// 1. UTC time of associated GGA fix
/// 2. Total RMS standard deviation of ranges inputs to the navigation solution
/// 3. Standard deviation (meters) of semi-major axis of error ellipse
/// 4. Standard deviation (meters) of semi-minor axis of error ellipse
/// 5. Orientation of semi-major axis of error ellipse (true north degrees)
/// 6. Standard deviation (meters) of latitude error
/// 7. Standard deviation (meters) of longitude error
/// 8. Standard deviation (meters) of altitude error
/// 9. Checksum
///
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, PartialEq)]
pub struct GstData {
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub time: Option<NaiveTime>,
    pub rms_sd: Option<f32>,
    pub ellipse_semi_major_sd: Option<f32>,
    pub ellipse_semi_minor_sd: Option<f32>,
    pub err_ellipse_orientation: Option<f32>,
    pub lat_sd: Option<f32>,
    pub long_sd: Option<f32>,
    pub alt_sd: Option<f32>,
}

fn do_parse_gst(i: &str) -> IResult<&str, GstData> {
    let (i, time) = opt(parse_hms).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, rms_sd) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, ellipse_semi_major_sd) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, ellipse_semi_minor_sd) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, err_ellipse_orientation) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, lat_sd) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, long_sd) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, alt_sd) = opt(float).parse(i)?;

    Ok((
        i,
        GstData {
            time,
            rms_sd,
            ellipse_semi_major_sd,
            ellipse_semi_minor_sd,
            err_ellipse_orientation,
            lat_sd,
            long_sd,
            alt_sd,
        },
    ))
}
pub fn parse_gst(sentence: NmeaSentence<'_>) -> Result<GstData, Error<'_>> {
    if sentence.message_id != SentenceType::GST {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::GST,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_gst(sentence.data)?.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Error, parse::parse_nmea_sentence};

    fn run_parse_gst(line: &str) -> Result<GstData, Error<'_>> {
        let s = parse_nmea_sentence(line).expect("GST sentence initial parse failed");
        assert_eq!(s.checksum, s.calc_checksum());
        parse_gst(s)
    }

    #[test]
    fn test_parse_gst() {
        assert_eq!(
            GstData {
                time: NaiveTime::from_hms_micro_opt(18, 21, 41, 00),
                rms_sd: Some(15.5),
                ellipse_semi_major_sd: Some(15.3),
                ellipse_semi_minor_sd: Some(7.2),
                err_ellipse_orientation: Some(21.8),
                lat_sd: Some(0.9),
                long_sd: Some(0.5),
                alt_sd: Some(0.8),
            },
            run_parse_gst("$GPGST,182141.000,15.5,15.3,7.2,21.8,0.9,0.5,0.8*54").unwrap()
        );
        assert_eq!(
            GstData {
                time: None,
                rms_sd: None,
                ellipse_semi_major_sd: None,
                ellipse_semi_minor_sd: None,
                err_ellipse_orientation: None,
                lat_sd: None,
                long_sd: None,
                alt_sd: None,
            },
            run_parse_gst("$GPGST,,,,,,,,*57").unwrap()
        );
    }
}
