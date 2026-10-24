#![cfg(feature = "GSA")]

use nmea::sentences::gnss_type::GnssSystemId;
use nmea::sentences::gsa::{GsaData, GsaMode1, GsaMode2};
use nmea::{Error, ParseResult};
use nom::error::{Error as NomError, ErrorKind};

fn sentence(body: &str) -> String {
    let checksum = body.bytes().fold(0, |value, byte| value ^ byte);
    format!("${body}*{checksum:02X}")
}

#[test]
fn supported_and_absent_system_ids_keep_existing_talker_inference() {
    for (talker, suffix, expected_id) in [
        ("GP", "", Some(GnssSystemId::Gps)),
        ("GP", ",1", Some(GnssSystemId::Gps)),
        ("GN", "", None),
        ("GN", ",1", Some(GnssSystemId::Gps)),
        ("GN", ",5", Some(GnssSystemId::Qzss)),
        ("GN", ",6", Some(GnssSystemId::NavIC)),
        ("PQ", ",5", Some(GnssSystemId::Qzss)),
        ("PQ", ",6", Some(GnssSystemId::NavIC)),
        ("BD", "", Some(GnssSystemId::Beidou)),
        ("BD", ",4", Some(GnssSystemId::Beidou)),
    ] {
        let line = sentence(&format!(
            "{talker}GSA,A,3,01,,,,,,,,,,,,1.0,0.8,0.6{suffix}"
        ));
        assert_eq!(
            nmea::parse_str(&line),
            Ok(ParseResult::GSA(GsaData {
                mode1: GsaMode1::Automatic,
                mode2: GsaMode2::Fix3D,
                fix_sats_prn: heapless::Vec::from_slice(&[1]).unwrap(),
                pdop: Some(1.0),
                hdop: Some(0.8),
                vdop: Some(0.6),
                system_id: expected_id,
                talker_id: heapless::String::try_from(talker).unwrap(),
            })),
        );
    }
}

#[test]
fn unsupported_and_malformed_ids_cannot_silently_fall_back_to_the_talker() {
    for (suffix, remaining) in [
        (",0", ",0"),
        (",7", ",7"),
        (",255", ",255"),
        (",256", ",256"),
        (",invalid", ",invalid"),
        (",1junk", "junk"),
        (",1,2", ",2"),
        (",", ","),
    ] {
        for talker in ["GP", "GN"] {
            let line = sentence(&format!(
                "{talker}GSA,A,3,01,,,,,,,,,,,,1.0,0.8,0.6{suffix}"
            ));
            assert_eq!(
                nmea::parse_str(&line),
                Err(Error::ParsingError(nom::Err::Error(NomError::new(
                    remaining,
                    ErrorKind::Eof
                )))),
            );
        }
    }
}

#[test]
fn unsupported_ids_with_empty_dilution_fields_are_rejected() {
    for id in [0, 7, 255] {
        let line = sentence(&format!("GPGSA,A,1,,,,,{id}"));
        assert_eq!(
            nmea::parse_str(&line),
            Err(Error::ParsingError(nom::Err::Error(NomError::new(
                "",
                ErrorKind::Char
            )))),
        );
    }
}

#[test]
fn recognized_conflicting_ids_keep_the_existing_typed_error() {
    let line = sentence("GPGSA,A,3,01,,,,,,,,,,,,1.0,0.8,0.6,3");
    assert_eq!(
        nmea::parse_str(&line),
        Err(Error::SystemIdMismatch {
            talker_sys_id: GnssSystemId::Gps,
            tail_sys_id: GnssSystemId::Galileo,
        }),
    );
}

#[test]
fn new_explicit_system_ids_are_preserved_without_dilution_fields() {
    for (id, expected_id) in [(5, GnssSystemId::Qzss), (6, GnssSystemId::NavIC)] {
        let line = sentence(&format!("GNGSA,A,1,,,,,{id}"));
        assert_eq!(
            nmea::parse_str(&line),
            Ok(ParseResult::GSA(GsaData {
                mode1: GsaMode1::Automatic,
                mode2: GsaMode2::NoFix,
                fix_sats_prn: heapless::Vec::new(),
                pdop: None,
                hdop: None,
                vdop: None,
                system_id: Some(expected_id),
                talker_id: heapless::String::try_from("GN").unwrap(),
            })),
        );
    }
}

#[test]
fn new_explicit_system_ids_still_conflict_with_existing_constellation_talkers() {
    for (id, expected_id) in [(5, GnssSystemId::Qzss), (6, GnssSystemId::NavIC)] {
        let line = sentence(&format!("GPGSA,A,3,01,,,,,,,,,,,,1.0,0.8,0.6,{id}"));
        assert_eq!(
            nmea::parse_str(&line),
            Err(Error::SystemIdMismatch {
                talker_sys_id: GnssSystemId::Gps,
                tail_sys_id: expected_id,
            }),
        );
    }
}
