#![cfg(feature = "GSA")]

use heapless::Vec;
use nmea::sentences::gnss_type::GnssSystemId;
use nmea::sentences::gsa::{self, GsaData, GsaMode1, GsaMode2};
use nom::Parser as _;

#[test]
fn extended_gsa_reports_preserve_all_identities_and_dilution_values() {
    for count in [18, 19, 24] {
        let prns = (1..=count)
            .map(|prn| prn.to_string())
            .collect::<std::vec::Vec<_>>()
            .join(",");
        let body = format!("GPGSA,A,3,{prns},1.0,0.8,0.6");
        let checksum = body.bytes().fold(0, |value, byte| value ^ byte);
        let sentence = format!("${body}*{checksum:02X}");
        assert_eq!(
            nmea::parse_str(&sentence),
            Ok(nmea::ParseResult::GSA(GsaData {
                mode1: GsaMode1::Automatic,
                mode2: GsaMode2::Fix3D,
                fix_sats_prn: (1..=count).collect(),
                pdop: Some(1.0),
                hdop: Some(0.8),
                vdop: Some(0.6),
                system_id: Some(GnssSystemId::Gps),
                talker_id: heapless::String::try_from("GP").unwrap(),
            })),
        );
    }
}

#[test]
fn public_many0_keeps_its_existing_output_type() {
    let result: nom::IResult<&str, Vec<char, 18>> =
        gsa::many0(nom::character::complete::char('x')).parse("xx,tail");
    assert_eq!(result, Ok((",tail", Vec::from_slice(&['x', 'x']).unwrap())));
}
