use chrono::{Duration, NaiveTime};
use nmea::sentences::utils;
use nom::error::{Error, ErrorKind};
use quickcheck::QuickCheck;

#[test]
fn nonfinite_seconds_return_the_exact_range_error() {
    for (input, error_input, kind) in [
        ("1200NaN,tail", "NaN", ErrorKind::Float),
        ("1200inf,tail", "inf", ErrorKind::Float),
        ("1200-inf,tail", "-inf", ErrorKind::Float),
    ] {
        assert_eq!(
            utils::parse_hms(input),
            Err(nom::Err::Error(Error::new(error_input, kind)))
        );
        assert_eq!(
            utils::parse_duration_hms(input),
            Err(nom::Err::Error(Error::new(error_input, kind)))
        );
    }
}

#[test]
fn trailing_seconds_content_returns_the_exact_unconsumed_suffix() {
    for (input, suffix) in [
        ("120000junk,tail", "junk"),
        ("120000.5junk,tail", "junk"),
        ("120000  ,tail", "  "),
    ] {
        assert_eq!(
            utils::parse_hms(input),
            Err(nom::Err::Error(Error::new(suffix, ErrorKind::Eof)))
        );
        assert_eq!(
            utils::parse_duration_hms(input),
            Err(nom::Err::Error(Error::new(suffix, ErrorKind::Eof)))
        );
    }
}

#[test]
fn valid_time_rounding_and_duration_millisecond_precision_are_unchanged() {
    assert_eq!(
        utils::parse_hms("125619.1234567899,tail"),
        Ok((
            ",tail",
            NaiveTime::from_hms_nano_opt(12, 56, 19, 123_456_790).unwrap()
        ))
    );
    assert_eq!(
        utils::parse_duration_hms("125619.1234567899,tail"),
        Ok((
            ",tail",
            Duration::milliseconds((12 * 3_600 + 56 * 60 + 19) * 1_000 + 123)
        ))
    );
}

#[test]
fn generated_times_preserve_exact_quarter_seconds() {
    fn property(hours: u8, minutes: u8, seconds: u8, quarter: u8) -> bool {
        let (hours, minutes, seconds, quarter) =
            (hours % 24, minutes % 60, seconds % 60, quarter % 4);
        let input = format!(
            "{hours:02}{minutes:02}{seconds:02}.{:02},tail",
            quarter * 25
        );
        assert_eq!(
            utils::parse_hms(&input),
            Ok((
                ",tail",
                NaiveTime::from_hms_nano_opt(
                    u32::from(hours),
                    u32::from(minutes),
                    u32::from(seconds),
                    u32::from(quarter) * 250_000_000
                )
                .unwrap()
            ))
        );
        let millis = (i64::from(hours) * 3_600 + i64::from(minutes) * 60 + i64::from(seconds))
            * 1_000
            + i64::from(quarter) * 250;
        assert_eq!(
            utils::parse_duration_hms(&input),
            Ok((",tail", Duration::milliseconds(millis)))
        );
        true
    }
    QuickCheck::new()
        .tests(10_000)
        .quickcheck(property as fn(u8, u8, u8, u8) -> bool);
}

#[cfg(feature = "RMC")]
#[test]
fn checksum_valid_nan_time_cannot_become_a_receiver_timestamp() {
    let line = "$GPRMC,1200NaN,A,5500.000,N,01200.000,E,1.0,90.0,231026,,,A*2D";
    assert_eq!(
        nmea::parse_str(line),
        Err(nmea::Error::ParsingError(nom::Err::Error(Error::new(
            "1200NaN,A,5500.000,N,01200.000,E,1.0,90.0,231026,,,A",
            ErrorKind::Char,
        ))))
    );
}
