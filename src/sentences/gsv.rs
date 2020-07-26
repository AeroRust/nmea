use nom::character::complete::char;
use nom::combinator::{cond, opt, rest_len};
use nom::IResult;

use crate::parse::NmeaSentence;
use crate::sentences::utils::number;
use crate::{GnssType, NmeaError, Satellite};

pub struct GsvData {
    pub gnss_type: GnssType,
    pub number_of_sentences: u16,
    pub sentence_num: u16,
    pub _sats_in_view: u16,
    pub sats_info: [Option<Satellite>; 4],
}

fn parse_gsv_sat_info(i: &[u8]) -> IResult<&[u8], Satellite> {
    let (i, prn) = number::<u32>(i)?;
    let (i, _) = char(',')(i)?;
    let (i, elevation) = opt(number::<i32>)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, azimuth) = opt(number::<i32>)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, snr) = opt(number::<i32>)(i)?;
    let (i, _) = cond(rest_len(i)?.1 > 0, char(','))(i)?;
    Ok((
        i,
        Satellite {
            gnss_type: GnssType::Galileo,
            prn,
            elevation: elevation.map(|v| v as f32),
            azimuth: azimuth.map(|v| v as f32),
            snr: snr.map(|v| v as f32),
        },
    ))
}

fn do_parse_gsv(i: &[u8]) -> IResult<&[u8], GsvData> {
    let (i, number_of_sentences) = number::<u16>(i)?;
    let (i, _) = char(',')(i)?;
    let (i, sentence_num) = number::<u16>(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _sats_in_view) = number::<u16>(i)?;
    let (i, _) = char(',')(i)?;
    let (i, sat0) = opt(parse_gsv_sat_info)(i)?;
    let (i, sat1) = opt(parse_gsv_sat_info)(i)?;
    let (i, sat2) = opt(parse_gsv_sat_info)(i)?;
    let (i, sat3) = opt(parse_gsv_sat_info)(i)?;
    Ok((
        i,
        GsvData {
            gnss_type: GnssType::Galileo,
            number_of_sentences,
            sentence_num,
            _sats_in_view,
            sats_info: [sat0, sat1, sat2, sat3],
        },
    ))
}

/// Parsin one GSV sentence
/// from gpsd/driver_nmea0183.c:
/// $IDGSV,2,1,08,01,40,083,46,02,17,308,41,12,07,344,39,14,22,228,45*75
/// 2           Number of sentences for full data
/// 1           Sentence 1 of 2
/// 08          Total number of satellites in view
/// 01          Satellite PRN number
/// 40          Elevation, degrees
/// 083         Azimuth, degrees
/// 46          Signal-to-noise ratio in decibels
/// <repeat for up to 4 satellites per sentence>
///
/// Can occur with talker IDs:
///   BD (Beidou),
///   GA (Galileo),
///   GB (Beidou),
///   GL (GLONASS),
///   GN (GLONASS, any combination GNSS),
///   GP (GPS, SBAS, QZSS),
///   QZ (QZSS).
///
/// GL may be (incorrectly) used when GSVs are mixed containing
/// GLONASS, GN may be (incorrectly) used when GSVs contain GLONASS
/// only.  Usage is inconsistent.
pub fn parse_gsv(sentence: NmeaSentence) -> Result<GsvData, NmeaError> {
    if sentence.message_id != b"GSV" {
        Err(NmeaError::WrongSentenceHeader {
            expected: b"GSV",
            found: sentence.message_id,
        })
    } else {
        let gnss_type = match sentence.talker_id {
            b"GP" => GnssType::Gps,
            b"GL" => GnssType::Glonass,
            _ => {
                return Err(NmeaError::WrongSentenceHeader {
                    expected: b"GP|GL",
                    found: sentence.message_id,
                })
            }
        };
        let mut res = do_parse_gsv(sentence.data)?.1;
        res.gnss_type = gnss_type;
        for sat in &mut res.sats_info {
            if let Some(v) = (*sat).as_mut() {
                v.gnss_type = gnss_type;
            }
        }
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gsv_full() {
        let data = parse_gsv(NmeaSentence {
            talker_id: b"GP",
            message_id: b"GSV",
            data: b"2,1,08,01,,083,46,02,17,308,,12,07,344,39,14,22,228,",
            checksum: 0,
        })
        .unwrap();
        assert_eq!(data.gnss_type, GnssType::Gps);
        assert_eq!(data.number_of_sentences, 2);
        assert_eq!(data.sentence_num, 1);
        assert_eq!(data._sats_in_view, 8);
        assert_eq!(
            data.sats_info[0].clone().unwrap(),
            Satellite {
                gnss_type: data.gnss_type.clone(),
                prn: 1,
                elevation: None,
                azimuth: Some(83.),
                snr: Some(46.),
            }
        );
        assert_eq!(
            data.sats_info[1].clone().unwrap(),
            Satellite {
                gnss_type: data.gnss_type.clone(),
                prn: 2,
                elevation: Some(17.),
                azimuth: Some(308.),
                snr: None,
            }
        );
        assert_eq!(
            data.sats_info[2].clone().unwrap(),
            Satellite {
                gnss_type: data.gnss_type.clone(),
                prn: 12,
                elevation: Some(7.),
                azimuth: Some(344.),
                snr: Some(39.),
            }
        );
        assert_eq!(
            data.sats_info[3].clone().unwrap(),
            Satellite {
                gnss_type: data.gnss_type.clone(),
                prn: 14,
                elevation: Some(22.),
                azimuth: Some(228.),
                snr: None,
            }
        );

        let data = parse_gsv(NmeaSentence {
            talker_id: b"GL",
            message_id: b"GSV",
            data: b"3,3,10,72,40,075,43,87,00,000,",
            checksum: 0,
        })
        .unwrap();
        assert_eq!(data.gnss_type, GnssType::Glonass);
        assert_eq!(data.number_of_sentences, 3);
        assert_eq!(data.sentence_num, 3);
        assert_eq!(data._sats_in_view, 10);
    }
}
