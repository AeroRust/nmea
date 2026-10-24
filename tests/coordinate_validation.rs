use nmea::sentences::utils;
use nom::error::{Error, ErrorKind};
use quickcheck::QuickCheck;

#[test]
fn invalid_minutes_and_geographic_endpoints_have_exact_errors() {
    for input in [
        "1234.567,N,09876.543,W",
        "9000.000001,N,01200.000,E",
        "9100.000,N,01200.000,E",
        "5560.000,N,01200.000,E",
    ] {
        assert_eq!(
            utils::do_parse_lat_lon(input),
            Err(nom::Err::Error(Error::new(
                if input.starts_with("1234") {
                    "09876.543,W"
                } else {
                    input
                },
                ErrorKind::MapRes,
            )))
        );
    }
    for longitude in ["18000.000001", "18100.000", "17960.000"] {
        let input = format!("5500.000,N,{longitude},E");
        let remainder = format!("{longitude},E");
        assert_eq!(
            utils::do_parse_lat_lon(&input),
            Err(nom::Err::Error(Error::new(
                remainder.as_str(),
                ErrorKind::MapRes
            )))
        );
    }
}

#[test]
fn valid_coordinates_preserve_values_and_remaining_input() {
    for (input, expected) in [
        ("9000.000,N,18000.000,E,tail", (90.0, 180.0)),
        ("9000.000,S,18000.000,W,tail", (-90.0, -180.0)),
        ("0000,S,00000,E,tail", (-0.0, 0.0)),
        (
            "8959.999999999999999,N,17959.999999999999999,E,tail",
            (90.0, 180.0),
        ),
        (
            "1234.567,N,09846.543,W,tail",
            (12.0 + 34.567 / 60.0, -(98.0 + 46.543 / 60.0)),
        ),
    ] {
        assert_eq!(utils::do_parse_lat_lon(input), Ok((",tail", expected)));
    }
}

#[test]
fn generated_exact_quarter_degree_coordinates_preserve_hemispheres() {
    fn property(latitude: u8, longitude: u8, quarter: u8, south: bool, west: bool) -> bool {
        let latitude = latitude % 90;
        let longitude = longitude % 180;
        let quarter = quarter % 4;
        let input = format!(
            "{latitude:02}{:02},{},{longitude:03}{:02},{},tail",
            quarter * 15,
            if south { 'S' } else { 'N' },
            quarter * 15,
            if west { 'W' } else { 'E' },
        );
        let expected_latitude =
            (f64::from(latitude) + f64::from(quarter) / 4.0) * if south { -1.0 } else { 1.0 };
        let expected_longitude =
            (f64::from(longitude) + f64::from(quarter) / 4.0) * if west { -1.0 } else { 1.0 };
        assert_eq!(
            utils::do_parse_lat_lon(&input),
            Ok((",tail", (expected_latitude, expected_longitude))),
        );
        true
    }
    QuickCheck::new()
        .tests(10_000)
        .quickcheck(property as fn(u8, u8, u8, bool, bool) -> bool);
}

#[cfg(all(feature = "GGA", feature = "GLL", feature = "GNS", feature = "RMC"))]
#[test]
fn complete_navigation_sentences_reject_invalid_positions() {
    for (kind, data) in [
        ("GGA", "120000,5560.0,N,01200.0,E,1,08,1.0,0.0,M,0.0,M,,"),
        ("GLL", "5560.0,N,01200.0,E,120000,A,A"),
        ("GNS", "120000,5560.0,N,01200.0,E,A,08,1.0,0.0,0.0,,,"),
        ("RMC", "120000,A,5560.0,N,01200.0,E,0.0,0.0,231026,,,A"),
    ] {
        let body = format!("GP{kind},{data}");
        let checksum = body.bytes().fold(0, |value, byte| value ^ byte);
        let line = format!("${body}*{checksum:02X}");
        let coordinate = &data[data.find("5560").unwrap()..];
        assert_eq!(
            nmea::parse_str(&line),
            Err(nmea::Error::ParsingError(nom::Err::Error(Error::new(
                coordinate,
                ErrorKind::MapRes
            )))),
        );
    }
}
