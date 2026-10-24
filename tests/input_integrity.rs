use nmea::parse_nmea_sentence;

fn sentence(body: &str) -> String {
    let checksum = body.bytes().fold(0_u8, |checksum, byte| checksum ^ byte);
    format!("${body}*{checksum:02X}")
}

#[test]
fn sentence_framing_accepts_only_one_sentence_and_one_optional_terminator() {
    let input = sentence("GPHDT,12.0,T");
    for ending in ["", "\n", "\r\n", "\r"] {
        assert!(parse_nmea_sentence(&format!("{input}{ending}")).is_ok());
    }
    for trailing in ["junk", " ", "\n\n", "\r\r", "\r\njunk", &input] {
        assert!(
            parse_nmea_sentence(&format!("{input}{trailing}")).is_err(),
            "accepted trailing input {trailing:?}"
        );
    }
    for input in ["$GPHDT,12.0,T*+A", "$GPHDT,12.0,T*-1"] {
        assert!(parse_nmea_sentence(input).is_err());
    }
}

#[cfg(feature = "GGA")]
mod coordinates {
    use nmea::{ParseResult, parse_str};
    use quickcheck::QuickCheck;

    use super::sentence;

    fn coordinates(latitude: &str, longitude: &str) -> String {
        sentence(&format!(
            "GPGGA,120000,{latitude},N,{longitude},E,1,08,1.0,0.0,M,0.0,M,,"
        ))
    }

    #[test]
    fn geographic_boundaries_and_subminute_precision_are_preserved() {
        for (latitude, longitude, expected_latitude, expected_longitude) in [
            ("9000.000000", "18000.000000", 90.0, 180.0),
            ("0000", "00000", 0.0, 0.0),
            ("8959.999999999999999", "17959.999999999999999", 90.0, 180.0),
            (
                "8959.999999999",
                "17959.999999999",
                89.0 + 59.999999999 / 60.0,
                179.0 + 59.999999999 / 60.0,
            ),
        ] {
            let ParseResult::GGA(parsed) =
                parse_str(&coordinates(latitude, longitude)).expect("valid coordinates")
            else {
                panic!("GGA result");
            };
            assert_eq!(parsed.latitude, Some(expected_latitude));
            assert_eq!(parsed.longitude, Some(expected_longitude));
        }
    }

    #[test]
    fn malformed_geographic_fields_fail_in_the_sentence_parser() {
        for malformed in [
            "9160.0", "9000.1", "8960.0", "000.0", "00000.0", "00-1.0", "00+1.0", "0001e1",
            "0001.", "00NaN", "00inf", "-100.0", "+100.0", "ab00.0",
        ] {
            assert!(
                parse_str(&coordinates(malformed, "01200.0")).is_err(),
                "accepted latitude {malformed}"
            );
        }
        for malformed in [
            "18100.0", "18000.1", "17960.0", "0000.0", "000000.0", "000-1.0", "000+1.0", "00001e1",
            "00001.", "000NaN", "000inf", "-0100.0", "+0100.0", "abc00.0",
        ] {
            assert!(
                parse_str(&coordinates("5500.0", malformed)).is_err(),
                "accepted longitude {malformed}"
            );
        }
    }

    fn encoded_components_match_geographic_validity(
        latitude_degrees: u8,
        longitude_degrees: u16,
        latitude_minute_units: u32,
        longitude_minute_units: u32,
        south: bool,
        west: bool,
    ) -> bool {
        const MINUTE_SCALE: u32 = 1_000_000;
        const MINUTE_LIMIT: u32 = 60 * MINUTE_SCALE;
        let latitude_minutes = latitude_minute_units % (100 * MINUTE_SCALE);
        let longitude_minutes = longitude_minute_units % (100 * MINUTE_SCALE);
        let body = format!(
            "GPGGA,120000,{latitude_degrees:02}{:02}.{:06},{},{longitude_degrees:03}{:02}.{:06},{},1,08,1.0,0.0,M,0.0,M,,",
            latitude_minutes / MINUTE_SCALE,
            latitude_minutes % MINUTE_SCALE,
            if south { 'S' } else { 'N' },
            longitude_minutes / MINUTE_SCALE,
            longitude_minutes % MINUTE_SCALE,
            if west { 'W' } else { 'E' },
        );
        let valid = latitude_degrees <= 90
            && longitude_degrees <= 180
            && latitude_minutes < MINUTE_LIMIT
            && longitude_minutes < MINUTE_LIMIT
            && (latitude_degrees != 90 || latitude_minutes == 0)
            && (longitude_degrees != 180 || longitude_minutes == 0);
        match parse_str(&sentence(&body)) {
            Ok(ParseResult::GGA(parsed)) if valid => {
                let latitude = (f64::from(latitude_degrees)
                    + f64::from(latitude_minutes) / f64::from(MINUTE_LIMIT))
                    * if south { -1.0 } else { 1.0 };
                let longitude = (f64::from(longitude_degrees)
                    + f64::from(longitude_minutes) / f64::from(MINUTE_LIMIT))
                    * if west { -1.0 } else { 1.0 };
                parsed.latitude.is_some_and(|value| {
                    value.is_finite()
                        && (-90.0..=90.0).contains(&value)
                        && (value - latitude).abs() < 1e-12
                }) && parsed.longitude.is_some_and(|value| {
                    value.is_finite()
                        && (-180.0..=180.0).contains(&value)
                        && (value - longitude).abs() < 1e-12
                })
            }
            Err(_) if !valid => true,
            _ => false,
        }
    }

    #[test]
    fn generated_coordinate_encodings_match_the_independent_geographic_oracle() {
        QuickCheck::new().tests(10_000).quickcheck(
            encoded_components_match_geographic_validity
                as fn(u8, u16, u32, u32, bool, bool) -> bool,
        );
    }

    #[test]
    fn generated_valid_coordinates_preserve_signed_decimal_degrees() {
        fn valid(
            latitude_degrees: u8,
            longitude_degrees: u16,
            latitude_minutes: u32,
            longitude_minutes: u32,
            south: bool,
            west: bool,
        ) -> bool {
            const MINUTE_LIMIT: u32 = 60_000_000;
            encoded_components_match_geographic_validity(
                latitude_degrees % 90,
                longitude_degrees % 180,
                latitude_minutes % MINUTE_LIMIT,
                longitude_minutes % MINUTE_LIMIT,
                south,
                west,
            )
        }
        QuickCheck::new()
            .tests(10_000)
            .quickcheck(valid as fn(u8, u16, u32, u32, bool, bool) -> bool);
    }

    #[cfg(all(feature = "GLL", feature = "GNS", feature = "RMC"))]
    #[test]
    fn every_navigation_sentence_rejects_invalid_position_fields() {
        for body in [
            "GPGGA,120000,5500.0,N,01200.0,E,1,08,1.0,0.0,M,0.0,M,,",
            "GPGLL,5500.0,N,01200.0,E,120000,A,A",
            "GPGNS,120000,5500.0,N,01200.0,E,A,08,1.0,0.0,0.0,,,",
            "GPRMC,120000,A,5500.0,N,01200.0,E,0.0,0.0,231026,,,A",
        ] {
            assert!(parse_str(&sentence(body)).is_ok(), "valid {body}");
            for invalid in [
                body.replace("5500.0", "5560.0"),
                body.replace("01200.0", "18100.0"),
            ] {
                assert!(
                    parse_str(&sentence(&invalid)).is_err(),
                    "accepted {invalid}"
                );
            }
        }
    }

    #[test]
    fn arbitrary_parsed_coordinate_fields_are_finite_and_within_geographic_bounds() {
        fn bounded(latitude: String, longitude: String) -> bool {
            match parse_str(&coordinates(&latitude, &longitude)) {
                Ok(ParseResult::GGA(parsed)) => {
                    parsed
                        .latitude
                        .is_some_and(|value| value.is_finite() && (-90.0..=90.0).contains(&value))
                        && parsed.longitude.is_some_and(|value| {
                            value.is_finite() && (-180.0..=180.0).contains(&value)
                        })
                }
                Err(_) => true,
                _ => false,
            }
        }
        QuickCheck::new()
            .tests(10_000)
            .quickcheck(bounded as fn(String, String) -> bool);
    }
}

#[cfg(feature = "GSA")]
mod gsa {
    use nmea::sentences::gnss_type::GnssSystemId;
    use nmea::{Error, ParseResult, parse_str};

    use super::sentence;

    fn gsa(talker: &str, suffix: &str) -> String {
        sentence(&format!(
            "{talker}GSA,A,3,01,,,,,,,,,,,,1.0,0.8,0.6{suffix}"
        ))
    }

    #[test]
    fn explicit_system_ids_and_absent_ids_preserve_their_origin() {
        for (talker, system_id, expected) in [
            ("GP", 1, GnssSystemId::Gps),
            ("GL", 2, GnssSystemId::Glonass),
            ("GA", 3, GnssSystemId::Galileo),
            ("GB", 4, GnssSystemId::Beidou),
            ("BD", 4, GnssSystemId::Beidou),
            ("GQ", 5, GnssSystemId::Qzss),
            ("QZ", 5, GnssSystemId::Qzss),
            ("GI", 6, GnssSystemId::NavIC),
        ] {
            for talker in [talker, "GN"] {
                let ParseResult::GSA(data) =
                    parse_str(&gsa(talker, &format!(",{system_id}"))).expect("supported ID")
                else {
                    panic!("GSA result");
                };
                assert_eq!(data.system_id, Some(expected));
                assert_eq!(data.effective_system_id(), Some(expected));
            }
            let ParseResult::GSA(data) = parse_str(&gsa(talker, "")).expect("absent ID") else {
                panic!("GSA result");
            };
            assert_eq!(data.system_id, None);
            assert_eq!(data.effective_system_id(), Some(expected));
        }
        let ParseResult::GSA(data) = parse_str(&gsa("GN", "")).expect("mixed talker") else {
            panic!("GSA result");
        };
        assert_eq!(data.system_id, None);
        assert_eq!(data.effective_system_id(), None);
    }

    #[test]
    fn unknown_system_ids_do_not_fall_back_to_the_talker() {
        for talker in ["GP", "GN", "GI", "GQ", "QZ", "BD"] {
            for system_id in [0, 7, 99, 255] {
                assert_eq!(
                    parse_str(&gsa(talker, &format!(",{system_id}"))),
                    Err(Error::UnknownSystemId(system_id)),
                );
                assert_eq!(
                    parse_str(&sentence(&format!("{talker}GSA,A,1,,,,,{system_id}"))),
                    Err(Error::UnknownSystemId(system_id)),
                );
            }
        }
    }

    #[test]
    fn variable_length_gsa_identity_lists_are_preserved_without_truncation() {
        let prns = (1..=24)
            .map(|prn| prn.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let ParseResult::GSA(data) =
            parse_str(&sentence(&format!("GNGSA,A,3,{prns},1.0,0.8,0.6,1")))
                .expect("extended identity list")
        else {
            panic!("GSA result");
        };
        assert_eq!(data.fix_sats_prn.as_slice(), (1..=24).collect::<Vec<_>>());
    }

    #[test]
    fn supported_talkers_reject_conflicting_explicit_system_ids() {
        let ids = [
            (1, GnssSystemId::Gps),
            (2, GnssSystemId::Glonass),
            (3, GnssSystemId::Galileo),
            (4, GnssSystemId::Beidou),
            (5, GnssSystemId::Qzss),
            (6, GnssSystemId::NavIC),
        ];
        for (talker, expected_talker) in [
            ("GP", GnssSystemId::Gps),
            ("GL", GnssSystemId::Glonass),
            ("GA", GnssSystemId::Galileo),
            ("GB", GnssSystemId::Beidou),
            ("BD", GnssSystemId::Beidou),
            ("GQ", GnssSystemId::Qzss),
            ("QZ", GnssSystemId::Qzss),
            ("GI", GnssSystemId::NavIC),
        ] {
            for (id, expected_tail) in ids {
                if expected_talker != expected_tail {
                    assert_eq!(
                        parse_str(&gsa(talker, &format!(",{id}"))),
                        Err(Error::SystemIdMismatch {
                            talker_sys_id: expected_talker,
                            tail_sys_id: expected_tail,
                        }),
                    );
                }
            }
        }
    }

    #[test]
    fn invalid_system_id_and_additional_tail_fields_are_rejected() {
        for suffix in [",", ",invalid", ",1junk", ",1,2", ",256"] {
            assert!(parse_str(&gsa("GP", suffix)).is_err(), "accepted {suffix}");
        }
    }
}
