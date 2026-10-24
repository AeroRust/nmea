use chrono::Timelike as _;
use nmea::sentences::utils;
use quickcheck::QuickCheck;

#[test]
fn clock_fields_preserve_boundaries_and_decimal_precision() {
    for (field, expected) in [
        ("000000", (0, 0, 0, 0)),
        ("120000.1", (12, 0, 0, 100_000_000)),
        ("120000.000000001", (12, 0, 0, 1)),
        ("235959.999999999", (23, 59, 59, 999_999_999)),
        ("235959.999999999999999", (23, 59, 59, 999_999_999)),
        ("120000.1234567899", (12, 0, 0, 123_456_789)),
    ] {
        let input = format!("{field},remaining fields");
        let (remaining, time) = utils::parse_hms(&input).expect("valid clock field");
        assert_eq!(remaining, ",remaining fields");
        assert_eq!(
            (time.hour(), time.minute(), time.second(), time.nanosecond()),
            expected,
        );
        let (remaining, duration) =
            utils::parse_duration_hms(&input).expect("valid duration field");
        assert_eq!(remaining, ",remaining fields");
        let expected_nanoseconds = i64::from(expected.0 * 3_600 + expected.1 * 60 + expected.2)
            * 1_000_000_000
            + i64::from(expected.3);
        assert_eq!(duration.num_nanoseconds(), Some(expected_nanoseconds));
    }
}

#[test]
fn nonfinite_invalid_and_partially_numeric_time_fields_are_rejected() {
    for field in [
        "1200NaN",
        "1200nan",
        "1200inf",
        "1200-inf",
        "1200+inf",
        "120000junk",
        "120000.1junk",
        "120000.1e0",
        "120000e0",
        "120000.",
        "1200+0",
        "1200-0",
        "+10000",
        "-10000",
        "12000",
        "1200000",
        "240000",
        "126000",
        "120060",
        "120000  ",
        "１２0000",
    ] {
        let input = format!("{field},remaining fields");
        assert!(utils::parse_hms(&input).is_err(), "accepted time {field}");
        assert!(
            utils::parse_duration_hms(&input).is_err(),
            "accepted duration {field}"
        );
    }
}

fn generated_components_preserve_time_values(
    hours: u8,
    minutes: u8,
    seconds: u8,
    nanoseconds: u32,
) -> bool {
    let nanoseconds = nanoseconds % 1_000_000_000;
    let input = format!("{hours:02}{minutes:02}{seconds:02}.{nanoseconds:09},suffix");
    let valid = hours < 24 && minutes < 60 && seconds < 60;
    match (utils::parse_hms(&input), utils::parse_duration_hms(&input)) {
        (Ok((remaining, time)), Ok((duration_remaining, duration))) if valid => {
            let expected_nanoseconds =
                (i64::from(hours) * 3_600 + i64::from(minutes) * 60 + i64::from(seconds))
                    * 1_000_000_000
                    + i64::from(nanoseconds);
            remaining == ",suffix"
                && duration_remaining == ",suffix"
                && time.hour() == u32::from(hours)
                && time.minute() == u32::from(minutes)
                && time.second() == u32::from(seconds)
                && time.nanosecond() == nanoseconds
                && duration.num_nanoseconds() == Some(expected_nanoseconds)
        }
        (Err(_), Err(_)) if !valid => true,
        _ => false,
    }
}

#[test]
fn generated_time_components_match_independent_ranges_and_integer_values() {
    QuickCheck::new()
        .tests(10_000)
        .quickcheck(generated_components_preserve_time_values as fn(u8, u8, u8, u32) -> bool);
}

#[test]
fn generated_valid_times_preserve_nanosecond_values() {
    fn valid(hours: u8, minutes: u8, seconds: u8, nanoseconds: u32) -> bool {
        generated_components_preserve_time_values(
            hours % 24,
            minutes % 60,
            seconds % 60,
            nanoseconds,
        )
    }
    QuickCheck::new()
        .tests(10_000)
        .quickcheck(valid as fn(u8, u8, u8, u32) -> bool);
}

#[test]
fn invalid_suffixes_cannot_be_ignored_after_valid_numeric_prefixes() {
    fn rejects(hours: u8, minutes: u8, seconds: u8, suffix: String) -> bool {
        let input = format!(
            "{:02}{:02}{:02}X{suffix},remaining fields",
            hours % 24,
            minutes % 60,
            seconds % 60,
        );
        utils::parse_hms(&input).is_err() && utils::parse_duration_hms(&input).is_err()
    }
    QuickCheck::new()
        .tests(10_000)
        .quickcheck(rejects as fn(u8, u8, u8, String) -> bool);
}

#[cfg(all(
    feature = "GGA",
    feature = "GLL",
    feature = "GNS",
    feature = "RMC",
    feature = "ZDA",
    feature = "ZFO",
    feature = "ZTG"
))]
mod sentences {
    use chrono::NaiveTime;
    use nmea::{Error, ParseResult, parse_str};

    fn sentence(body: &str) -> String {
        let checksum = body.bytes().fold(0_u8, |checksum, byte| checksum ^ byte);
        format!("${body}*{checksum:02X}")
    }

    fn source_time(parsed: ParseResult) -> Option<NaiveTime> {
        match parsed {
            ParseResult::GGA(data) => data.fix_time,
            ParseResult::GLL(data) => data.fix_time,
            ParseResult::GNS(data) => data.fix_time,
            ParseResult::RMC(data) => data.fix_time,
            ParseResult::ZDA(data) => data.utc_time,
            ParseResult::ZFO(data) => data.fix_time,
            ParseResult::ZTG(data) => data.fix_time,
            _ => panic!("timestamped source sentence"),
        }
    }

    #[test]
    fn optional_time_parsers_preserve_absence_and_reject_malformed_present_fields() {
        for body in [
            "GPGGA,120000,5500.0,N,01200.0,E,1,08,1.0,0.0,M,0.0,M,,",
            "GPGLL,5500.0,N,01200.0,E,120000,A,A",
            "GPGNS,120000,5500.0,N,01200.0,E,A,08,1.0,0.0,0.0,,,",
            "GPRMC,120000,A,5500.0,N,01200.0,E,0.0,0.0,231026,,,A",
            "GPZDA,120000,23,10,2026,00,00",
            "GPZFO,120000,010000,ORIGIN",
            "GPZTG,120000,010000,DESTINATION",
        ] {
            let parsed = parse_str(&sentence(body)).expect("valid source time");
            assert_eq!(source_time(parsed), NaiveTime::from_hms_opt(12, 0, 0));
            let parsed =
                parse_str(&sentence(&body.replace("120000", ""))).expect("absent optional time");
            assert_eq!(source_time(parsed), None);
            for invalid in ["1200NaN", "120000junk", "120000.1e0", "1200+0"] {
                let malformed = sentence(&body.replace("120000", invalid));
                assert!(
                    matches!(parse_str(&malformed), Err(Error::ParsingError(_))),
                    "accepted malformed source time {malformed}",
                );
            }
        }
        for body in [
            "GPZFO,120000,010000,ORIGIN",
            "GPZTG,120000,010000,DESTINATION",
        ] {
            for invalid in ["0100NaN", "010000junk", "010000.1e0", "0100+0"] {
                assert!(
                    parse_str(&sentence(&body.replace("010000", invalid))).is_err(),
                    "accepted malformed duration {invalid}",
                );
            }
        }
    }

    #[test]
    fn checksum_valid_nan_time_is_a_recoverable_parser_error() {
        let malformed = "$GPRMC,1200NaN,A,5500.000,N,01200.000,E,1.0,90.0,231026,,,A*2D";
        assert!(matches!(parse_str(malformed), Err(Error::ParsingError(_))));
        let good = sentence("GPRMC,120001,A,5500.0,N,01200.0,E,1.0,90.0,231026,,,A");
        let parsed = parse_str(&good).expect("subsequent valid time");
        assert_eq!(source_time(parsed), NaiveTime::from_hms_opt(12, 0, 1));
    }
}
