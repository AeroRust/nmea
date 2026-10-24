use nmea::sentences::gnss_type::{GnssSystemId, GnssType};

#[test]
fn system_ids_preserve_existing_values_and_add_qzss_and_navic() {
    for (id, system, constellation, name) in [
        (1, GnssSystemId::Gps, GnssType::Gps, "GPS"),
        (2, GnssSystemId::Glonass, GnssType::Glonass, "GLONASS"),
        (3, GnssSystemId::Galileo, GnssType::Galileo, "Galileo"),
        (4, GnssSystemId::Beidou, GnssType::Beidou, "Beidou"),
        (5, GnssSystemId::Qzss, GnssType::Qzss, "QZSS"),
        (6, GnssSystemId::NavIC, GnssType::NavIC, "NavIC"),
    ] {
        assert_eq!(GnssSystemId::try_from(id), Ok(system));
        assert_eq!(GnssType::from(system), constellation);
        assert_eq!(system.to_string(), name);
    }
    assert_eq!(
        GnssSystemId::ALL_TYPES,
        [
            GnssSystemId::Gps,
            GnssSystemId::Glonass,
            GnssSystemId::Galileo,
            GnssSystemId::Beidou,
            GnssSystemId::Qzss,
            GnssSystemId::NavIC
        ],
    );
    for id in [0, 7, 255] {
        assert_eq!(GnssSystemId::try_from(id), Err(()));
    }
}

#[cfg(feature = "serde")]
#[test]
fn new_system_ids_roundtrip_with_their_enum_names() {
    for (system, encoded) in [
        (GnssSystemId::Qzss, "\"Qzss\""),
        (GnssSystemId::NavIC, "\"NavIC\""),
    ] {
        assert_eq!(serde_json::to_string(&system).unwrap(), encoded);
        assert_eq!(
            serde_json::from_str::<GnssSystemId>(encoded).unwrap(),
            system
        );
    }
}
