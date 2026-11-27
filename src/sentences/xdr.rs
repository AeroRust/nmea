use heapless::Vec;
use nom::{
    IResult, Parser as _,
    bytes::complete::take_while,
    character::complete::{anychar, char},
    combinator::opt,
    number::complete::double,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{Error, SentenceType, parse::NmeaSentence};

/// Maximum number of transducer measurements in a single XDR sentence.
/// Real instruments send up to 5 measurements (e.g., B&G H5000, compass devices).
const MAX_MEASUREMENTS: usize = 8;

/// A single transducer measurement from an XDR sentence.
///
/// Each measurement consists of four fields:
/// - Transducer type (e.g., 'P' for pressure, 'C' for temperature)
/// - Measurement value
/// - Unit of measurement (e.g., 'B' for bars, 'C' for Celsius)
/// - Transducer name (e.g., "Barometer", "AirTemp")
///
/// Some instruments emit empty unit or name fields; these are represented
/// as empty strings / null chars rather than Options, matching the NMEA
/// convention of "field present but empty".
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, PartialEq)]
pub struct XdrMeasurement {
    /// Transducer type indicator:
    /// - 'A' = Angular displacement (degrees)
    /// - 'C' = Temperature
    /// - 'D' = Depth
    /// - 'E' = Fluid level (v4.11)
    /// - 'G' = Generic (magnetic field, engine hours)
    /// - 'H' = Humidity
    /// - 'I' = Current (amperes)
    /// - 'P' = Pressure
    /// - 'T' = Tachometer (RPM)
    /// - 'U' = Voltage
    pub transducer_type: char,
    /// Measurement value.
    pub value: f64,
    /// Unit of measurement indicator, or '\0' if the field is empty.
    pub units: char,
    /// Transducer name (instrument-defined). May be empty.
    pub name: arrayvec::ArrayString<16>,
}

/// XDR - Transducer Measurements
///
/// Generic transducer measurement sentence used to convey data from various
/// on-board sensors: barometric pressure, air temperature, humidity, rudder
/// angle, battery voltage, engine data, and more.
///
/// Contains one or more measurement groups (typically 1-5).
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_xdr_transducer_measurement>
///
/// ```text
///        1 2   3 4       1 2   3 4
///        | |   | |       | |   | |
/// $--XDR,a,x.x,a,c--c[,a,x.x,a,c--c]*hh<CR><LF>
/// ```
///
/// Groups of 4 fields, repeatable:
/// 1. Transducer type
/// 2. Measurement value
/// 3. Unit of measurement (may be empty)
/// 4. Transducer name (may be empty)
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, PartialEq)]
pub struct XdrData {
    /// One or more transducer measurements.
    pub measurements: Vec<XdrMeasurement, MAX_MEASUREMENTS>,
}

/// # Parse XDR message
///
/// Transducer Measurements — a generic container for sensor data.
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_xdr_transducer_measurement>
///
/// ## Examples:
/// ```text
/// $IIXDR,P,1.0154,B,Barometer*16
/// $IIXDR,P,1.015,B,Baro,C,22.0,C,AirTemp*21
/// $WIXDR,C,022.0,C,,*52
/// ```
pub fn parse_xdr(sentence: NmeaSentence<'_>) -> Result<XdrData, Error<'_>> {
    if sentence.message_id != SentenceType::XDR {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::XDR,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_xdr(sentence.data)?.1)
    }
}

fn do_parse_xdr(i: &str) -> IResult<&str, XdrData> {
    let mut measurements = Vec::new();
    let mut remaining = i;

    while !remaining.is_empty() {
        // Transducer type (single char)
        let (i, transducer_type) = anychar.parse(remaining)?;
        let (i, _) = char(',').parse(i)?;

        // Value (floating point)
        let value_result: IResult<&str, f64> = double.parse(i);
        let (i, value) = match value_result {
            Ok((i, v)) => (i, v),
            Err(_) => break, // incomplete quadruplet — stop parsing
        };
        let (i, _) = char(',').parse(i)?;

        // Units — may be empty (e.g., engine hours: "G,200,,ENGINE#0")
        let (i, units) = opt(|i| {
            let (i, c) = anychar.parse(i)?;
            if c == ',' {
                Err(nom::Err::Error(nom::error::Error::new(
                    i,
                    nom::error::ErrorKind::Char,
                )))
            } else {
                Ok((i, c))
            }
        })
        .parse(i)?;
        let units = units.unwrap_or('\0');
        let (i, _) = char(',').parse(i)?;

        // Name — may be empty (e.g., "$WIXDR,C,022.0,C,,*52")
        let (i, name_str) = take_while(|c| c != ',').parse(i)?;
        let mut name = arrayvec::ArrayString::<16>::new();
        let truncated = if name_str.len() > 16 {
            &name_str[..16]
        } else {
            name_str
        };
        let _ = name.try_push_str(truncated);

        if measurements.is_full() {
            break;
        }
        measurements
            .push(XdrMeasurement {
                transducer_type,
                value,
                units,
                name,
            })
            .ok();

        // Optional comma before next group
        let (i, _) = opt(char(',')).parse(i)?;
        remaining = i;
    }

    Ok(("", XdrData { measurements }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_parse_xdr_single_pressure() {
        let s = parse_nmea_sentence("$IIXDR,P,1.0154,B,Barometer*16").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        let xdr = parse_xdr(s).unwrap();
        assert_eq!(xdr.measurements.len(), 1);
        let m = &xdr.measurements[0];
        assert_eq!(m.transducer_type, 'P');
        assert!((m.value - 1.0154).abs() < 0.0001);
        assert_eq!(m.units, 'B');
        assert_eq!(m.name.as_str(), "Barometer");
    }

    #[test]
    fn test_parse_xdr_single_temperature() {
        let s = parse_nmea_sentence("$IIXDR,C,23.5,C,AirTemp*22").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        let xdr = parse_xdr(s).unwrap();
        assert_eq!(xdr.measurements.len(), 1);
        assert_eq!(xdr.measurements[0].transducer_type, 'C');
        assert!((xdr.measurements[0].value - 23.5).abs() < 0.1);
        assert_eq!(xdr.measurements[0].name.as_str(), "AirTemp");
    }

    #[test]
    fn test_parse_xdr_angular_displacement() {
        let s = parse_nmea_sentence("$IIXDR,A,-2.3,D,RUDDER*59").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        let xdr = parse_xdr(s).unwrap();
        assert_eq!(xdr.measurements.len(), 1);
        let m = &xdr.measurements[0];
        assert_eq!(m.transducer_type, 'A');
        assert!((m.value - (-2.3)).abs() < 0.1);
        assert_eq!(m.units, 'D');
        assert_eq!(m.name.as_str(), "RUDDER");
    }

    #[test]
    fn test_parse_xdr_two_measurements() {
        let s = parse_nmea_sentence("$IIXDR,P,1.015,B,Baro,C,22.0,C,AirTemp*21").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        let xdr = parse_xdr(s).unwrap();
        assert_eq!(xdr.measurements.len(), 2);
        assert_eq!(xdr.measurements[0].transducer_type, 'P');
        assert_eq!(xdr.measurements[0].name.as_str(), "Baro");
        assert_eq!(xdr.measurements[1].transducer_type, 'C');
        assert_eq!(xdr.measurements[1].name.as_str(), "AirTemp");
    }

    #[test]
    fn test_parse_xdr_four_measurements() {
        let s = parse_nmea_sentence(
            "$IIXDR,P,1.013,B,Baro,C,19.5,C,TempAir,H,65.2,P,Humidity,A,-1.5,D,RUDDER*06",
        )
        .unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        let xdr = parse_xdr(s).unwrap();
        assert_eq!(xdr.measurements.len(), 4);
        assert_eq!(xdr.measurements[0].transducer_type, 'P');
        assert_eq!(xdr.measurements[1].transducer_type, 'C');
        assert_eq!(xdr.measurements[2].transducer_type, 'H');
        assert_eq!(xdr.measurements[3].transducer_type, 'A');
    }

    #[test]
    fn test_parse_xdr_empty_name_field() {
        // Real-world: Calypso anemometer sends empty name
        let s = parse_nmea_sentence("$WIXDR,C,022.0,C,,*52").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        let xdr = parse_xdr(s).unwrap();
        assert_eq!(xdr.measurements.len(), 1);
        assert_eq!(xdr.measurements[0].name.as_str(), "");
        assert!((xdr.measurements[0].value - 22.0).abs() < 0.1);
    }

    #[test]
    fn test_parse_xdr_empty_units_field() {
        // Real-world: engine hours often have empty unit field
        let s = parse_nmea_sentence("$IIXDR,G,200,,ENGHRS*3E").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        let xdr = parse_xdr(s).unwrap();
        assert_eq!(xdr.measurements.len(), 1);
        assert_eq!(xdr.measurements[0].units, '\0');
        assert!((xdr.measurements[0].value - 200.0).abs() < 0.1);
        assert_eq!(xdr.measurements[0].name.as_str(), "ENGHRS");
    }

    #[test]
    fn test_parse_xdr_wrong_sentence_type() {
        let s = parse_nmea_sentence("$INMTW,17.9,C*1B").unwrap();
        assert!(parse_xdr(s).is_err());
    }
}
