use heapless::Vec;
use nom::{
    IResult, Parser as _,
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, one_of},
    combinator::{cond, eof, map, opt, peek, rest_len},
    number::complete::float,
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
/// | Field | Description |
/// |------:|-------------|
/// | `1` | total number of GSV sentences to be transmitted in this group |
/// | `2` | Sentence number, 1-9 of this GSV message within current group |
/// | `3` | total number of satellites in view (leading zeros sent) |
/// | `4` | satellite ID or PRN number (leading zeros sent) |
/// | `5` | elevation in degrees (-90 to 90); some receivers emit fractional values |
/// | `6` | azimuth in degrees to true north (0 to 359); some receivers emit fractional values |
/// | `7` | SNR in dB (0 to 99); some receivers emit fractional values |
/// | `4-7` | may repeat for up to four satellites per sentence |
/// | `n-1` | Signal ID (NMEA 4.10+), when present |
/// | `n` | checksum |
///
/// Example:
///
/// `$GPGSV,3,1,11,03,03,111,00,04,15,270,00,06,01,010,00,13,06,292,00*74`
///
/// `$GPGSV,3,2,11,14,25,170,00,16,57,208,39,18,67,296,40,19,40,246,00*74`
///
/// `$GPGSV,3,3,11,22,42,067,42,24,14,311,43,27,05,244,00,,,,*4D`
///
/// Some receivers, including the Quectel BG96, emit fractional elevation,
/// azimuth, and SNR values. These fields are therefore parsed as floating-point
/// values.
///
/// Some GPS receivers may emit more than 12 quadruples (more than three `GPGSV` sentences),
/// even though NMEA-0183 doesn’t allow this. (The extras might be `WAAS` satellites, for example.)
///
/// Receivers may also report quads for satellites they aren’t tracking, in which case the `SNR` field will be null;
/// we don’t know whether this is formally allowed or not.
///
/// Example: `$GLGSV,3,3,09,88,07,028*51`
///
/// Note: NMEA 4.10+ systems may emit an extra `Signal ID` field just before the
/// checksum. This raw hexadecimal ID applies to every satellite in the sentence.
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
    /// Raw NMEA 4.10+ Signal ID, encoded as one hexadecimal digit on the wire.
    /// Returned as `0x0..=0xF` (wire `B` becomes `Some(11)`), without translation
    /// to a u-blox signal ID. `None` means absent or empty, `Some(0)` means
    /// "all signals". Other meanings depend on `gnss_type`.
    pub signal_id: Option<u8>,
    /// Up to four satellite observations from this GSV sentence.
    ///
    /// Empty slots, and observations without a satellite ID, are represented as
    /// `None`. A satellite ID is required to construct a [`Satellite`].
    pub sats_info: Vec<Option<Satellite>, 4>,
}

fn parse_gsv_sat_info(i: &str) -> IResult<&str, Option<Satellite>> {
    let (i, prn) = opt(number::<u32>).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, elevation) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, azimuth) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, snr) = opt(float).parse(i)?;
    let (i, _) = cond(rest_len(i)?.1 > 0, char(',')).parse(i)?;

    Ok((
        i,
        prn.map(|prn| Satellite {
            gnss_type: GnssType::Galileo,
            prn,
            elevation,
            azimuth,
            snr,
            signal_id: None,
        }),
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

        sats.insert(sat_index, sat.flatten()).unwrap();
        Ok((i, sats))
    })?;

    // Validate just this field and preserve the remainder for receiver extensions.
    let (i, signal_id) = if i.is_empty() || i.starts_with(',') {
        (i, None)
    } else {
        let (i, digit) = one_of("0123456789ABCDEFabcdef").parse(i)?;
        let (i, _) = peek(alt((tag(","), eof))).parse(i)?;
        (i, Some(digit.to_digit(16).unwrap() as u8))
    };
    let mut sats = sats;
    for sat in sats.iter_mut().flatten() {
        sat.signal_id = signal_id;
    }

    Ok((
        i,
        GsvData {
            gnss_type: GnssType::Galileo,
            number_of_sentences,
            sentence_num,
            sats_in_view,
            signal_id,
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
        let (tail, mut res) = do_parse_gsv(sentence.data)?;
        let gnss_type = if sentence.talker_id == "PQ" && !tail.is_empty() {
            let (tail, _) = char(',').parse(tail)?;
            let (tail, gnss_type) = map(one_of("45"), |system_id| {
                if system_id == '4' {
                    GnssType::Beidou
                } else {
                    GnssType::Qzss
                }
            })
            .parse(tail)?;
            let (_, _) = peek(alt((tag(","), eof))).parse(tail)?;
            gnss_type
        } else {
            gnss_type
        };
        res.gnss_type = gnss_type;
        for sat in res.sats_info.iter_mut().flatten() {
            sat.gnss_type = gnss_type;
        }
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_nmea_sentence;

    #[test]
    fn test_quectel_system_id_field() {
        // Constructed tails test field validation without whole-body strictness.
        let sentence = NmeaSentence {
            talker_id: "PQ",
            message_id: SentenceType::GSV,
            data: "1,1,01,05,13,030,26,0,4,vendor",
            checksum: 0,
        };
        assert_eq!(parse_gsv(sentence).unwrap().gnss_type, GnssType::Beidou);
        for data in ["1,1,01,05,13,030,26,0,45", "1,1,01,05,13,030,26,0,3"] {
            assert!(
                parse_gsv(NmeaSentence {
                    data,
                    talker_id: "PQ",
                    message_id: SentenceType::GSV,
                    checksum: 0,
                })
                .is_err()
            );
        }
    }

    #[test]
    fn test_quectel_empty_signal_id_with_system_id() {
        // Constructed empty Signal-ID field followed by Quectel System ID 4.
        let data = parse_gsv(NmeaSentence {
            talker_id: "PQ",
            message_id: SentenceType::GSV,
            data: "1,1,01,05,13,030,26,,4",
            checksum: 0,
        })
        .unwrap();
        assert_eq!(data.signal_id, None);
        assert_eq!(data.gnss_type, GnssType::Beidou);
        assert_eq!(data.sats_info.iter().flatten().count(), 1);
        assert!(
            data.sats_info
                .iter()
                .flatten()
                .all(|sat| { sat.gnss_type() == GnssType::Beidou && sat.signal_id().is_none() })
        );
    }

    #[test]
    fn test_quectel_without_system_id() {
        // Constructed Signal-ID-only tail protects the historical PQ fallback.
        let data = parse_gsv(NmeaSentence {
            talker_id: "PQ",
            message_id: SentenceType::GSV,
            data: "1,1,01,05,13,030,26,0",
            checksum: 0,
        })
        .unwrap();
        assert_eq!(data.gnss_type, GnssType::Qzss);
        assert_eq!(data.signal_id, Some(0));
        assert_eq!(data.sats_info.iter().flatten().count(), 1);
        assert!(
            data.sats_info
                .iter()
                .flatten()
                .all(|sat| { sat.gnss_type() == GnssType::Qzss && sat.signal_id() == Some(0) })
        );
    }

    #[test]
    fn test_signal_id_tail() {
        // Constructed variants of the documented padded sentence.
        for (input, expected_id) in [
            (
                "3,3,11,22,42,067,42,24,14,311,43,27,05,244,00,,,,,B",
                Some(11),
            ),
            (
                "3,3,11,22,42,067,42,24,14,311,43,27,05,244,00,,,,,0",
                Some(0),
            ),
        ] {
            let (_, data) = do_parse_gsv(input).unwrap();
            assert_eq!(data.signal_id, expected_id);
            assert_eq!(data.sats_info.iter().flatten().count(), 3);
            assert!(data.sats_info[3].is_none());
            assert!(
                data.sats_info
                    .iter()
                    .flatten()
                    .all(|sat| sat.signal_id() == expected_id)
            );
        }
        // Reject a multi-character Signal ID, without rejecting unknown fields
        // after a valid Signal ID.
        assert!(do_parse_gsv("1,1,01,05,13,030,26,10").is_err());
        let (remaining, data) = do_parse_gsv("1,1,01,05,13,030,26,1,vendor").unwrap();
        assert_eq!(data.signal_id, Some(1));
        assert_eq!(remaining, ",vendor");
    }

    #[test]
    fn test_parse_gsv_full() {
        // Legacy padded sentence from the GSV documentation above.
        let (remaining, padded) =
            do_parse_gsv("3,3,11,22,42,067,42,24,14,311,43,27,05,244,00,,,,").unwrap();
        assert!(remaining.is_empty());
        assert_eq!(padded.signal_id, None);
        assert_eq!(padded.sats_info.iter().flatten().count(), 3);

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
        assert_eq!(data.signal_id, None);
        assert_eq!(
            data.sats_info[0].clone().unwrap(),
            Satellite {
                gnss_type: data.gnss_type,
                prn: 1,
                elevation: None,
                azimuth: Some(83.),
                snr: Some(46.),
                signal_id: None,
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
                signal_id: None,
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
                signal_id: None,
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
                signal_id: None,
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

    #[test]
    fn test_parse_gsv_decimal_satellite_fields() {
        let data = parse_gsv(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::GSV,
            data: "6,1,24,05,13.4,30.9,26.8,16,64.7,258.8,32.2,18,67.5,83.0,31.6,08,7.7,279.8,,1",
            checksum: 0x46,
        })
        .unwrap();

        assert_eq!(data.gnss_type, GnssType::Gps);
        assert_eq!(data.number_of_sentences, 6);
        assert_eq!(data.sentence_num, 1);
        assert_eq!(data.sats_in_view, 24);
        assert_eq!(data.signal_id, Some(1));

        assert_eq!(
            data.sats_info[0].clone().unwrap(),
            Satellite {
                gnss_type: data.gnss_type,
                prn: 5,
                elevation: Some(13.4),
                azimuth: Some(30.9),
                snr: Some(26.8),
                signal_id: Some(1),
            }
        );
        assert_eq!(
            data.sats_info[1].clone().unwrap(),
            Satellite {
                gnss_type: GnssType::Gps,
                prn: 16,
                elevation: Some(64.7),
                azimuth: Some(258.8),
                snr: Some(32.2),
                signal_id: Some(1),
            }
        );
        assert_eq!(
            data.sats_info[2].clone().unwrap(),
            Satellite {
                gnss_type: GnssType::Gps,
                prn: 18,
                elevation: Some(67.5),
                azimuth: Some(83.0),
                snr: Some(31.6),
                signal_id: Some(1),
            }
        );
        assert_eq!(
            data.sats_info[3].clone().unwrap(),
            Satellite {
                gnss_type: GnssType::Gps,
                prn: 8,
                elevation: Some(7.7),
                azimuth: Some(279.8),
                snr: None,
                signal_id: Some(1),
            }
        );
    }

    #[test]
    fn test_bg96_missing_satellite_id_preserves_signal_id() {
        // Captured from a Quectel BG96 while acquiring satellites.
        //
        // $GLGSV,1,1,01,,,,26.9,1*6A
        //              | | | |  |
        //              | | | |  +---- NMEA 4.10 Signal ID = 1
        //              | | | +------- SNR/CN0 = 26.9
        //              | | +--------- azimuth unavailable
        //              | +----------- elevation unavailable
        //              +------------- satellite ID unavailable
        //
        // The BG96 advertises one GLONASS satellite in view but emits an SNR without
        // identifying the satellite. Since the PRN is missing, this observation cannot
        // be represented as a Satellite and is discarded.
        //
        // The malformed satellite fields must still be consumed as one complete GSV
        // satellite tuple. Otherwise parsing loses field alignment and the following
        // sentence-level Signal ID is missed (or could be mistaken for satellite data).
        //
        // This matters beyond this exact sentence: a GSV message may contain other
        // valid satellite observations whose Signal ID should still be preserved.
        //
        // Expected result:
        // - the sentence parses successfully,
        // - no Satellite observation is returned,
        // - Signal ID 1 is preserved.
        let sentence = parse_nmea_sentence("$GLGSV,1,1,01,,,,26.9,1*6A").unwrap();

        let data = parse_gsv(sentence).unwrap();

        assert_eq!(data.gnss_type, GnssType::Glonass);
        assert_eq!(data.number_of_sentences, 1);
        assert_eq!(data.sentence_num, 1);
        assert_eq!(data.sats_in_view, 1);
        assert_eq!(data.signal_id, Some(1));
        assert!(data.sats_info.iter().all(Option::is_none));
    }
}
