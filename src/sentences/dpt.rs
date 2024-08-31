use nom::character::complete::char;
use nom::combinator::opt;
use nom::number::complete::double;
use nom::IResult;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::Error;
use crate::ParseResult;
use crate::SentenceType;

// DPT - Depth of Water
//         1   2   3   4
//         |   |   |   |
//  $--DPT,x.x,x.x,x.x*hh<CR><LF>
/// ```
///
/// Field Number:

// 1. Water depth relative to transducer, meters

// 2. Offset from transducer, meters positive means distance from transducer to water line negative means distance from transducer to keel

// 3. Maximum range scale in use (NMEA 3.0 and above)

// 4. Checksum

/// Example: `$INDPT,2.3,0.0*46`
/// `$SDDPT,15.2,0.5*68` - `$SDDPT` is the sentence identifier (`SD` for the talker ID, `DPT` for Depth)
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DptData {
    pub water_depth: Option<f64>,
    pub offset: Option<f64>,
    pub max_range_scale: Option<f64>,
}

impl From<DptData> for ParseResult {
    fn from(value: DptData) -> Self {
        ParseResult::DPT(value)
    }
}

pub fn parse_dpt_(sentence: crate::NmeaSentence) -> Result<DptData, crate::Error> {
    if sentence.message_id != crate::SentenceType::DPT {
        return Err(Error::WrongSentenceHeader {
            expected: SentenceType::DPT,
            found: sentence.message_id,
        });
    } else {
        match do_parse_dbt(sentence.data) {
            Ok((_, data)) => Ok(data),
            Err(err) => Err(Error::ParsingError(err)),
        }
    }
}

fn do_parse_dbt(i: &str) -> IResult<&str, DptData> {
    let (i, water_depth) = opt(double)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, offset) = opt(double)(i)?;
    // this comma is optional in NMEA 2.3
    let (i, comma) = opt(char(','))(i)?;
    let (i, max_range_scale) = if comma.is_some() {
        let (i, max_range_scale) = opt(double)(i)?;
        (i, max_range_scale)
    } else {
        (i, None)
    };

    Ok((
        i,
        DptData {
            water_depth,
            offset,
            max_range_scale,
        },
    ))
}

#[cfg(test)]
mod tests {

    use super::*;

    struct TestExpectation(&'static str, DptData);
    struct FailedTestExpectation(&'static str);

    fn test_check_valid_message(
        message: &str,
        expected: DptData,
    ) -> std::result::Result<(), String> {
        let result = crate::parse::parse_nmea_sentence(message);
        match result {
            Ok(sentence) => {
                let dpt_data = parse_dpt_(sentence);
                match dpt_data {
                    Ok(data) => {
                        if data != expected {
                            return Err(format!(
                                "DPT parse result is different from expectations. Expected: {:?}, got {:?}",
                                expected, data
                            ));
                        }
                        Ok(())
                    }
                    Err(_) => Err(format!("Failed to parse DPT sentence: {}", message)),
                }
            }
            Err(_) => Err(format!(
                "NMEA sentence is constructed incorrectly: {}",
                message
            )),
        }
    }

    fn test_invalid_message(message: &str) -> std::result::Result<(), String> {
        let result = crate::parse::parse_nmea_sentence(message);
        match result {
            Ok(sentence) => {
                let dpt_data = parse_dpt_(sentence);
                match dpt_data {
                    Ok(_) => Err(format!(
                        "Parsing should have failed for message: {}",
                        message
                    )),
                    Err(e) => {
                        println!("{:?}", e);
                        Ok(())
                    }
                }
            }
            Err(_) => Err(format!(
                "NMEA sentence is constructed incorrectly: {}",
                message
            )),
        }
    }

    #[test]
    fn test_parse_dpt() -> std::result::Result<(), String> {
        let correct_dpt_messages: [TestExpectation; 11] = [
            TestExpectation(
                "$SDDPT,2.4,,*53",
                DptData {
                    water_depth: Some(2.4),
                    offset: None,
                    max_range_scale: None,
                },
            ), // checksum fails
            TestExpectation(
                "$SDDPT,15.2,0.5*64",
                DptData {
                    water_depth: Some(15.2),
                    offset: Some(0.5),
                    max_range_scale: None,
                },
            ),
            TestExpectation(
                "$SDDPT,15.5,0.5*63",
                DptData {
                    water_depth: Some(15.5),
                    offset: Some(0.5),
                    max_range_scale: None,
                },
            ),
            TestExpectation(
                "$SDDPT,15.8,0.5*6E",
                DptData {
                    water_depth: Some(15.8),
                    offset: Some(0.5),
                    max_range_scale: None,
                },
            ),
            TestExpectation(
                "$SDDPT,16.1,0.5*64",
                DptData {
                    water_depth: Some(16.1),
                    offset: Some(0.5),
                    max_range_scale: None,
                },
            ),
            TestExpectation(
                "$SDDPT,16.4,0.5*61",
                DptData {
                    water_depth: Some(16.4),
                    offset: Some(0.5),
                    max_range_scale: None,
                },
            ),
            TestExpectation(
                "$SDDPT,16.7,0.5*62",
                DptData {
                    water_depth: Some(16.7),
                    offset: Some(0.5),
                    max_range_scale: None,
                },
            ),
            TestExpectation(
                "$SDDPT,17.0,0.5*64",
                DptData {
                    water_depth: Some(17.0),
                    offset: Some(0.5),
                    max_range_scale: None,
                },
            ),
            TestExpectation(
                "$SDDPT,17.3,0.5*67",
                DptData {
                    water_depth: Some(17.3),
                    offset: Some(0.5),
                    max_range_scale: None,
                },
            ),
            TestExpectation(
                "$SDDPT,17.9,0.5*6D",
                DptData {
                    water_depth: Some(17.9),
                    offset: Some(0.5),
                    max_range_scale: None,
                },
            ),
            TestExpectation(
                "$SDDPT,18.7,0.5,2.0*6C",
                DptData {
                    water_depth: Some(18.7),
                    offset: Some(0.5),
                    max_range_scale: Some(2.0),
                },
            ), // Extra field (NMEA 2.3 DPT has only 2 fields before checksum)
        ];

        let incorrect_dpt_messages: [FailedTestExpectation; 11] = [
            FailedTestExpectation("$SDDPT,15.2,0.5,*6C"),
            FailedTestExpectation(
                "$SDDPT,15.2,0.5,*68", // Extra comma before the checksum
            ),
            FailedTestExpectation("$SDDPT,-12.3,0.5,*6A"),
            FailedTestExpectation(
                "$SDDPT,ABC,0.5*41", // non-numeric water depth
            ),
            FailedTestExpectation(
                "$SDDPT,20.1,XYZ*55", // non-numeric offset
            ),
            FailedTestExpectation(
                "$SDDPT,22.3*31", // missing offset
            ),
            FailedTestExpectation(
                "$SDDPT,19.8,0.5*ZZ", // Invalid checksum (not hexadecimal)
            ),
            FailedTestExpectation(
                "$SDDPT,16.5,0.5,3.0,4.0*6B", // Too many fields
            ),
            FailedTestExpectation(
                "$SDDPT,21.0,-1.5*65", // negative offset
            ),
            FailedTestExpectation(
                "$SDDPT,17.2 0.5*60", // missing comma
            ),
            FailedTestExpectation(
                "$SDDPT,18.3,0.5*XX", // Invalid checksum (not hexadecimal)
            ),
        ];

        correct_dpt_messages
            .iter()
            .try_for_each(|test_expectation| {
                test_check_valid_message(test_expectation.0, test_expectation.1)
            })?;

        incorrect_dpt_messages
            .iter()
            .try_for_each(|test_expectation| test_invalid_message(test_expectation.0))?;

        Ok(())
    }
}
