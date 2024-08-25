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
// Field Number:

// 1. Water depth relative to transducer, meters

// 2. Offset from transducer, meters positive means distance from transducer to water line negative means distance from transducer to keel

// 3. Maximum range scale in use (NMEA 3.0 and above)

// 4. Checksum

// Example: $INDPT,2.3,0.0*46
// $SDDPT,15.2,0.5*68 - $SDDPT is the sentence identifier (SD for the talker ID, DPT for Depth)
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
        ParseResult::DPT_(value)
    }
}

pub fn parse_dpt_(sentence: crate::NmeaSentence) -> Result<DptData, crate::Error> {
    if sentence.message_id != crate::SentenceType::DPT_ {
        return Err(Error::WrongSentenceHeader {
            expected: SentenceType::DPT_,
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
    let (i, _) = char(',')(i)?;
    let (i, max_range_scale) = opt(double)(i)?;
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

    #[test]
    fn test_parse_dpt() {
        let correct_dpt_messages: [&str; 10] = [
            "15.2,0.5*68",
            "15.5,0.5*6D",
            "15.8,0.5*62",
            "16.1,0.5*66",
            "16.4,0.5*61",
            "16.7,0.5*64",
            "17.0,0.5*64",
            "17.3,0.5*61",
            "17.9,0.5*63",
            "18.7,0.5,2.0*70", // Extra field (NMEA 2.3 DPT has only 2 fields before checksum)
        ];

        let incorrect_dpt_messages: [&str; 10] = [
            "15.2,0.5,*68",        // Extra comma before the checksum
            "-12.3,0.5*6A",        // negative water depth
            "ABC,0.5*41",          // non-numeric water depth
            "20.1,XYZ*55",         // non-numeric offset
            "22.3*31",             // missing offset
            "19.8,0.5*ZZ",         // Invalid checksum (not hexadecimal)
            "16.5,0.5,3.0,4.0*6B", // Too many fields
            "21.0,-1.5*65",        // negative offset
            "17.2 0.5*60",         // missing comma
            "18.3,0.5*XX",         // Invalid checksum (not hexadecimal)
        ];

        for msg in correct_dpt_messages.iter() {
            let seq = format!("$SDDPT,{}", msg);
            println!("{}", seq); // $SDDPT,15.2,0.5*68
            let s = crate::parse::parse_nmea_sentence(&seq);
            assert!(s.is_ok());
            let s = s.unwrap();
            println!("{:?}", s.checksum); // 104 VS calculated 100, but 104 must be correct
            assert_eq!(s.checksum, s.calc_checksum());
            let dpt_data = parse_dpt_(s);
            // asset the result is ok
            assert!(dpt_data.is_ok());
        }

        for msg in incorrect_dpt_messages.iter() {
            let seq = format!("$SDDPT,{}", msg);
            let s = crate::parse::parse_nmea_sentence(&seq);
            match s {
                Ok(sentence) => {
                    assert_eq!(sentence.checksum, sentence.calc_checksum());
                    let dpt_data = parse_dpt_(sentence);
                    // asset the result is an error
                    assert!(dpt_data.is_err());
                }
                Err(_) => {
                    // checksum error
                    assert!(true);
                }
            }
        }
    }
}
