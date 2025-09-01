use heapless::Vec;
use nom::{
    IResult, Parser as _,
    character::complete::char,
    combinator::{cond, opt, rest_len},
};

use crate::{
    Error, Satellite, SentenceType,
    parse::NmeaSentence,
    sentences::{GnssType, utils::number},
};

/// GSV - Satellites in view
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_gsv_satellites_in_view>
///
/// ```text
///        1 2 3 4 5 6 7     n
///        | | | | | | |     |
/// $--GSV,x,x,x,x,x,x,x,...*hh<CR><LF>
/// ```
///
/// Field Number:
/// 1. total number of GSV sentences to be transmitted in this group
/// 2. Sentence number, 1-9 of this GSV message within current group
/// 3. total number of satellites in view (leading zeros sent)
/// 4. satellite ID or PRN number (leading zeros sent)
/// 5. elevation in degrees (-90 to 90) (leading zeros sent)
/// 6. azimuth in degrees to true north (000 to 359) (leading zeros sent)
/// 7. SNR in dB (00-99) (leading zeros sent) more satellite info quadruples like 4-7 n-1) Signal ID (NMEA 4.11)
///
///    n. checksum
///
/// Example:
///
/// `$GPGSV,3,1,11,03,03,111,00,04,15,270,00,06,01,010,00,13,06,292,00*74`
///
/// `$GPGSV,3,2,11,14,25,170,00,16,57,208,39,18,67,296,40,19,40,246,00*74`
///
/// `$GPGSV,3,3,11,22,42,067,42,24,14,311,43,27,05,244,00,,,,*4D`
///
/// Some GPS receivers may emit more than 12 quadruples (more than three `GPGSV` sentences),
/// even though NMEA-0813 doesn’t allow this. (The extras might be `WAAS` satellites, for example.)
///
/// Receivers may also report quads for satellites they aren’t tracking, in which case the `SNR` field will be null;
/// we don’t know whether this is formally allowed or not.
///
/// Example: `$GLGSV,3,3,09,88,07,028*51`
///
/// Note: NMEA 4.10+ systems (`u-blox 9`, `Quectel LCD79`) may emit an extra field,
/// `Signal ID`, just before the checksum. See the description of `Signal ID`'s above.
///
/// Note: `$GNGSV` uses `PRN` in field 4. Other `$GxGSV` use the `satellite ID` in field 4.
/// Jackson Labs, Quectel, Telit, and others get this wrong, in various conflicting ways.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, PartialEq)]
pub struct GsvData {
    pub gnss_type: GnssType,
    pub number_of_sentences: u16,
    pub sentence_num: u16,
    pub sats_in_view: u16,
    // see SatPack in lib.rs
    pub sats_info: Vec<Option<Satellite>, 4>,
}

fn parse_gsv_sat_info(i: &str) -> IResult<&str, Satellite> {
    let (i, prn) = number::<u32>(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, elevation) = opt(number::<i32>).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, azimuth) = opt(number::<i32>).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, snr) = opt(number::<i32>).parse(i)?;
    let (i, _) = cond(rest_len(i)?.1 > 0, char(',')).parse(i)?;
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

fn do_parse_gsv(i: &str) -> IResult<&str, GsvData> {
    let (i, number_of_sentences) = number::<u16>(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, sentence_num) = number::<u16>(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, sats_in_view) = number::<u16>(i)?;
    let (i, _) = char(',').parse(i)?;
    let sats = Vec::<Option<Satellite>, 4>::new();

    // We loop through the indices and parse the satellite data
    let (i, sats) = (0..4).try_fold((i, sats), |(i, mut sats), sat_index| {
        let (i, sat) = opt(parse_gsv_sat_info).parse(i)?;

        sats.insert(sat_index, sat).unwrap();

        Ok((i, sats))
    })?;

    Ok((
        i,
        GsvData {
            gnss_type: GnssType::Galileo,
            number_of_sentences,
            sentence_num,
            sats_in_view,
            sats_info: sats,
        },
    ))
}

/// # Parse one GSV message
///
/// From gpsd/driver_nmea0183.c:
///
/// `$IDGSV,2,1,08,01,40,083,46,02,17,308,41,12,07,344,39,14,22,228,45*75`
///
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
/// - BD (Beidou),
/// - GA (Galileo),
/// - GB (Beidou),
/// - GI (NavIC - India)
/// - GL (GLONASS),
/// - GN (GLONASS, any combination GNSS),
/// - GP (GPS, SBAS, QZSS),
/// - GQ (QZSS)
/// - PQ (QZSS)
/// - QZ (QZSS)
///
/// GL may be (incorrectly) used when GSVs are mixed containing
/// GLONASS, GN may be (incorrectly) used when GSVs contain GLONASS
/// only.  Usage is inconsistent.
pub fn parse_gsv(sentence: NmeaSentence<'_>) -> Result<GsvData, Error<'_>> {
    if sentence.message_id != SentenceType::GSV {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::GSV,
            found: sentence.message_id,
        })
    } else {
        let gnss_type = match sentence.talker_id {
            "GA" => GnssType::Galileo,
            "GP" => GnssType::Gps,
            "GL" => GnssType::Glonass,
            "BD" | "GB" => GnssType::Beidou,
            "GI" => GnssType::NavIC,
            "GQ" | "PQ" | "QZ" => GnssType::Qzss,
            _ => return Err(Error::UnknownGnssType(sentence.talker_id)),
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
            talker_id: "GP",
            message_id: SentenceType::GSV,
            data: "2,1,08,01,,083,46,02,17,308,,12,07,344,39,14,22,228,",
            checksum: 0,
        })
        .unwrap();
        assert_eq!(data.gnss_type, GnssType::Gps);
        assert_eq!(data.number_of_sentences, 2);
        assert_eq!(data.sentence_num, 1);
        assert_eq!(data.sats_in_view, 8);
        assert_eq!(
            data.sats_info[0].clone().unwrap(),
            Satellite {
                gnss_type: data.gnss_type,
                prn: 1,
                elevation: None,
                azimuth: Some(83.),
                snr: Some(46.),
            }
        );
        assert_eq!(
            data.sats_info[1].clone().unwrap(),
            Satellite {
                gnss_type: data.gnss_type,
                prn: 2,
                elevation: Some(17.),
                azimuth: Some(308.),
                snr: None,
            }
        );
        assert_eq!(
            data.sats_info[2].clone().unwrap(),
            Satellite {
                gnss_type: data.gnss_type,
                prn: 12,
                elevation: Some(7.),
                azimuth: Some(344.),
                snr: Some(39.),
            }
        );
        assert_eq!(
            data.sats_info[3].clone().unwrap(),
            Satellite {
                gnss_type: data.gnss_type,
                prn: 14,
                elevation: Some(22.),
                azimuth: Some(228.),
                snr: None,
            }
        );

        let data = parse_gsv(NmeaSentence {
            talker_id: "GL",
            message_id: SentenceType::GSV,
            data: "3,3,10,72,40,075,43,87,00,000,",
            checksum: 0,
        })
        .unwrap();
        assert_eq!(data.gnss_type, GnssType::Glonass);
        assert_eq!(data.number_of_sentences, 3);
        assert_eq!(data.sentence_num, 3);
        assert_eq!(data.sats_in_view, 10);

        let data = parse_gsv(NmeaSentence {
            talker_id: "GQ",
            message_id: SentenceType::GSV,
            data: "3,3,10,72,40,075,43,87,00,000,",
            checksum: 0,
        })
        .unwrap();
        assert_eq!(data.gnss_type, GnssType::Qzss);
        assert_eq!(data.number_of_sentences, 3);
        assert_eq!(data.sentence_num, 3);
        assert_eq!(data.sats_in_view, 10);
    }
}
