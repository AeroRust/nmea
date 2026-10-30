//! Decimal-only float parsing for NMEA fields.
//!
//! NMEA numeric fields use plain decimal notation. This parser deliberately
//! avoids `core::str::FromStr` for floats, which pulls `dec2flt` and the large
//! `POWER_OF_FIVE_128` table into embedded images. Exponent notation is not
//! part of the NMEA fields used by this crate.

use nom::{
    IResult,
    error::{Error, ErrorKind},
};

/// Parse a plain decimal number into an `f64`, returning the number and the
/// number of input bytes consumed.
fn parse_f64_inner(input: &str) -> Result<(f64, usize), &'static str> {
    let bytes = input.as_bytes();
    if bytes.is_empty() {
        return Err("empty number");
    }

    let mut at = 0;
    let negative = bytes[0] == b'-';
    if negative || bytes[0] == b'+' {
        at += 1;
    }

    let mut value = 0.0f64;
    let mut seen_digit = false;
    while at < bytes.len() && bytes[at].is_ascii_digit() {
        value = value * 10.0 + f64::from(bytes[at] - b'0');
        at += 1;
        seen_digit = true;
    }
    if at < bytes.len() && bytes[at] == b'.' {
        at += 1;
        let mut scale = 0.1f64;
        while at < bytes.len() && bytes[at].is_ascii_digit() {
            value += f64::from(bytes[at] - b'0') * scale;
            scale *= 0.1;
            at += 1;
            seen_digit = true;
        }
    }
    if !seen_digit {
        return Err("invalid number");
    }

    Ok((if negative { -value } else { value }, at))
}

/// Parse a complete, plain decimal number into an `f64`.
pub(crate) fn parse_f64(input: &str) -> Result<f64, &'static str> {
    let (value, len) = parse_f64_inner(input)?;
    if len != input.len() {
        return Err("invalid number");
    }
    Ok(value)
}

/// Parse a complete, plain decimal number into an `f32`.
pub(crate) fn parse_f32(input: &str) -> Result<f32, &'static str> {
    parse_f64(input).map(|value| value as f32)
}

/// Parse one comma-delimited NMEA field as `f64`.
pub(crate) fn float64(input: &str) -> IResult<&str, f64> {
    match parse_f64_inner(input) {
        Ok((value, len)) => Ok((&input[len..], value)),
        Err(_) => Err(nom::Err::Error(Error::new(input, ErrorKind::Float))),
    }
}

/// Parse one comma-delimited NMEA field as `f32`.
pub(crate) fn float32(input: &str) -> IResult<&str, f32> {
    float64(input).map(|(rest, value)| (rest, value as f32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn digits(min: usize, max: usize) -> impl Strategy<Value = String> {
        proptest::collection::vec(0u8..=9, min..=max).prop_map(|digits| {
            digits
                .into_iter()
                .map(|digit| char::from(b'0' + digit))
                .collect()
        })
    }

    fn decimal_string() -> impl Strategy<Value = String> {
        (0u8..=2, digits(0, 12), digits(0, 12)).prop_filter_map(
            "`.` alone is not a valid input",
            |(sign, integer, fraction)| {
                if integer.is_empty() && fraction.is_empty() {
                    return None;
                }
                let sign = match sign {
                    0 => "",
                    1 => "-",
                    _ => "+",
                };
                Some(format!("{sign}{integer}.{fraction}"))
            },
        )
    }

    fn assert_close_f64(actual: f64, expected: f64) {
        let scale = actual.abs().max(expected.abs()).max(1.0);
        assert!(
            (actual - expected).abs() <= 1e-12 * scale,
            "actual={actual:?}, expected={expected:?}"
        );
    }

    fn assert_close_f32(actual: f32, expected: f32) {
        let scale = actual.abs().max(expected.abs()).max(1.0);
        assert!(
            (actual - expected).abs() <= 1e-5 * scale,
            "actual={actual:?}, expected={expected:?}"
        );
    }

    proptest! {
        #[test]
        fn float64_matches_std(input in decimal_string()) {
            let expected = input.parse::<f64>().unwrap();
            let actual = parse_f64(&input).unwrap();
            assert_close_f64(actual, expected);
        }

        #[test]
        fn float32_matches_std(input in decimal_string()) {
            let expected = input.parse::<f32>().unwrap();
            let actual = parse_f32(&input).unwrap();
            assert_close_f32(actual, expected);
        }
    }

    #[test]
    fn parses_plain_decimals() {
        assert_eq!(parse_f64("12.5"), Ok(12.5));
        assert_eq!(parse_f32("12.5"), Ok(12.5));
        assert_eq!(parse_f64("+12.5"), Ok(12.5));
        assert_eq!(parse_f64("-12.5"), Ok(-12.5));
        assert_eq!(parse_f64(".5"), Ok(0.5));
        assert_eq!(parse_f64("12."), Ok(12.0));
    }

    #[test]
    fn rejects_malformed_decimals() {
        for input in ["", "-", "+", ".", "1.2.3", "1e3", "12x", "--1", "1,"] {
            assert!(parse_f64(input).is_err(), "accepted {input:?}");
        }
    }
}
