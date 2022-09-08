use heapless::Vec;
use nom::{
    branch::alt,
    bytes::complete::take_while1,
    character::complete::{char, one_of},
    combinator::{all_consuming, opt, value},
    error::{ErrorKind, ParseError},
    number::complete::float,
    sequence::terminated,
    Err, IResult, InputLength, Parser,
};

use crate::{parse::NmeaSentence, sentences::utils::number, NmeaError};

#[derive(PartialEq, Eq, Debug)]
pub enum GsaMode1 {
    Manual,
    Automatic,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GsaMode2 {
    NoFix,
    Fix2D,
    Fix3D,
}

#[derive(Debug, PartialEq)]
pub struct GsaData {
    pub mode1: GsaMode1,
    pub mode2: GsaMode2,
    pub fix_sats_prn: Vec<u32, 12>,
    pub pdop: Option<f32>,
    pub hdop: Option<f32>,
    pub vdop: Option<f32>,
}

/// This function is take from `nom`, see `nom::multi::many0`
/// with one difference - we use a [`heapless::Vec`]
/// because we want `no_std` & no `alloc`
fn many0<I, O, E, F>(mut f: F) -> impl FnMut(I) -> IResult<I, Vec<O, 12>, E>
where
    I: Clone + InputLength,
    F: Parser<I, O, E>,
    E: ParseError<I>,
    O: core::fmt::Debug,
{
    move |mut i: I| {
        let mut acc = Vec::<_, 12>::new();
        loop {
            let len = i.input_len();
            match f.parse(i.clone()) {
                Err(Err::Error(_)) => return Ok((i, acc)),
                Err(e) => return Err(e),
                Ok((i1, o)) => {
                    // infinite loop check: the parser must always consume
                    if i1.input_len() == len {
                        return Err(Err::Error(E::from_error_kind(i, ErrorKind::Many0)));
                    }

                    i = i1;
                    acc.push(o).unwrap();
                }
            }
        }
    }
}

fn gsa_prn_fields_parse(i: &[u8]) -> IResult<&[u8], Vec<Option<u32>, 12>> {
    many0(terminated(opt(number::<u32>), char(',')))(i)
}

type GsaTail = (Vec<Option<u32>, 12>, Option<f32>, Option<f32>, Option<f32>);

fn do_parse_gsa_tail(i: &[u8]) -> IResult<&[u8], GsaTail> {
    let (i, prns) = gsa_prn_fields_parse(i)?;
    let (i, pdop) = float(i)?;
    let (i, _) = char(',')(i)?;
    let (i, hdop) = float(i)?;
    let (i, _) = char(',')(i)?;
    let (i, vdop) = float(i)?;
    Ok((i, (prns, Some(pdop), Some(hdop), Some(vdop))))
}

fn is_comma(x: u8) -> bool {
    x == b','
}

fn do_parse_empty_gsa_tail(i: &[u8]) -> IResult<&[u8], GsaTail> {
    value(
        (Vec::new(), None, None, None),
        all_consuming(take_while1(is_comma)),
    )(i)
}

fn do_parse_gsa(i: &[u8]) -> IResult<&[u8], GsaData> {
    let (i, mode1) = one_of("MA")(i)?;
    let (i, _) = char(',')(i)?;
    let (i, mode2) = one_of("123")(i)?;
    let (i, _) = char(',')(i)?;
    let (i, mut tail) = alt((do_parse_empty_gsa_tail, do_parse_gsa_tail))(i)?;
    Ok((
        i,
        GsaData {
            mode1: match mode1 {
                'M' => GsaMode1::Manual,
                'A' => GsaMode1::Automatic,
                _ => unreachable!(),
            },
            mode2: match mode2 {
                '1' => GsaMode2::NoFix,
                '2' => GsaMode2::Fix2D,
                '3' => GsaMode2::Fix3D,
                _ => unreachable!(),
            },
            fix_sats_prn: {
                let mut fix_sats_prn = Vec::<u32, 12>::new();
                for sat in tail.0.iter().flatten() {
                    fix_sats_prn.push(*sat).unwrap()
                }
                // now that we don't have `drain()` from `std::Vec`,
                // we clear the `heapless::Vec`'s tail manually
                tail.0.clear();
                fix_sats_prn
            },
            pdop: tail.1,
            hdop: tail.2,
            vdop: tail.3,
        },
    ))
}

/// # Parse GSA message
///
/// From gpsd:
///
/// eg1. `$GPGSA,A,3,,,,,,16,18,,22,24,,,3.6,2.1,2.2*3C`
/// eg2. `$GPGSA,A,3,19,28,14,18,27,22,31,39,,,,,1.7,1.0,1.3*35`
/// 1    = Mode:
/// M=Manual, forced to operate in 2D or 3D
/// A=Automatic, 3D/2D
/// 2    = Mode: 1=Fix not available, 2=2D, 3=3D
/// 3-14 = PRNs of satellites used in position fix (null for unused fields)
/// 15   = PDOP
/// 16   = HDOP
/// 17   = VDOP
///
/// Not all documentation specifies the number of PRN fields, it
/// may be variable. Most doc that specifies says 12 PRNs.
///
/// The CH-4701 outputs 24 PRNs!
///
/// The Skytraq S2525F8-BD-RTK output both GPGSA and BDGSA in the
/// same cycle:
/// $GPGSA,A,3,23,31,22,16,03,07,,,,,,,1.8,1.1,1.4*3E
/// $BDGSA,A,3,214,,,,,,,,,,,,1.8,1.1,1.4*18
/// These need to be combined like GPGSV and BDGSV
///
/// Some GPS emit GNGSA.  So far we have not seen a GPS emit GNGSA
/// and then another flavor of xxGSA
///
/// Some Skytraq will emit all GPS in one GNGSA, Then follow with
/// another GNGSA with the BeiDou birds.
///
/// SEANEXX and others also do it:
///
/// ```text
/// $GNGSA,A,3,31,26,21,,,,,,,,,,3.77,2.55,2.77*1A
/// $GNGSA,A,3,75,86,87,,,,,,,,,,3.77,2.55,2.77*1C
/// ```
/// seems like the first is GNSS and the second GLONASS
///
/// One chipset called the i.Trek M3 issues GPGSA lines that look like
/// this: "$GPGSA,A,1,,,,*32" when it has no fix. This is broken
/// in at least two ways:
/// - It's got the wrong number of fields
/// - it claims to be a valid sentence (A flag) when it isn't
///
/// Alarmingly, it's possible this error may be generic to SiRFstarIII
pub fn parse_gsa(sentence: NmeaSentence) -> Result<GsaData, NmeaError> {
    if sentence.message_id != b"GSA" {
        Err(NmeaError::WrongSentenceHeader {
            expected: b"GSA",
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_gsa(sentence.data)?.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_nmea_sentence;

    #[test]
    fn test_gsa_prn_fields_parse() {
        let (_, ret) = gsa_prn_fields_parse(b"5,").unwrap();
        assert_eq!(ret, &[Some(5)]);

        let (_, ret) = gsa_prn_fields_parse(b",").unwrap();
        assert_eq!(ret, &[None]);

        let (_, ret) = gsa_prn_fields_parse(b",,5,6,").unwrap();
        assert_eq!(ret, &[None, None, Some(5), Some(6)],);
    }

    #[test]
    fn smoke_test_parse_gsa() {
        let s = parse_nmea_sentence(b"$GPGSA,A,3,,,,,,16,18,,22,24,,,3.6,2.1,2.2*3C").unwrap();
        let gsa = parse_gsa(s).unwrap();
        assert_eq!(
            GsaData {
                mode1: GsaMode1::Automatic,
                mode2: GsaMode2::Fix3D,
                fix_sats_prn: Vec::<_, 12>::from_slice(&[16, 18, 22, 24]).unwrap(),
                pdop: Some(3.6),
                hdop: Some(2.1),
                vdop: Some(2.2),
            },
            gsa
        );
        let gsa_examples = [
            "$GPGSA,A,3,19,28,14,18,27,22,31,39,,,,,1.7,1.0,1.3*35",
            "$GPGSA,A,3,23,31,22,16,03,07,,,,,,,1.8,1.1,1.4*3E",
            "$BDGSA,A,3,214,,,,,,,,,,,,1.8,1.1,1.4*18",
            "$GNGSA,A,3,31,26,21,,,,,,,,,,3.77,2.55,2.77*1A",
            "$GNGSA,A,3,75,86,87,,,,,,,,,,3.77,2.55,2.77*1C",
            "$GPGSA,A,1,,,,*32",
        ];
        for line in &gsa_examples {
            println!("we parse line '{}'", line);
            let s = parse_nmea_sentence(line.as_bytes()).unwrap();
            parse_gsa(s).unwrap();
        }
    }
}
