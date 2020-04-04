use core::str;

use nom::bytes::complete::{take, take_until};
use nom::character::complete::char;
use nom::combinator::map_res;

use nom::sequence::preceded;
use nom::IResult;

pub use crate::sentences::*;
use crate::SentenceType;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NmeaSentence<'a> {
    pub talker_id: &'a [u8],
    pub message_id: &'a [u8],
    pub data: &'a [u8],
    pub checksum: u8,
}

impl<'a> NmeaSentence<'a> {
    pub fn calc_checksum(&self) -> u8 {
        checksum(
            self.talker_id
                .iter()
                .chain(self.message_id.iter())
                .chain(&[b','])
                .chain(self.data.iter()),
        )
    }
}

pub fn checksum<'a, I: Iterator<Item = &'a u8>>(bytes: I) -> u8 {
    bytes.fold(0, |c, x| c ^ *x)
}

fn parse_hex(data: &[u8]) -> Result<u8, &'static str> {
    u8::from_str_radix(unsafe { str::from_utf8_unchecked(data) }, 16)
        .map_err(|_| "Failed to parse checksum as hex number")
}

fn parse_checksum(i: &[u8]) -> IResult<&[u8], u8> {
    map_res(preceded(char('*'), take(2usize)), parse_hex)(i)
}

fn do_parse_nmea_sentence(i: &[u8]) -> IResult<&[u8], NmeaSentence> {
    let (i, talker_id) = preceded(char('$'), take(2usize))(i)?;
    let (i, message_id) = take(3usize)(i)?;
    let (i, _) = char(',')(i)?;
    let (i, data) = take_until("*")(i)?;
    let (i, checksum) = parse_checksum(i)?;

    Ok((
        i,
        NmeaSentence {
            talker_id,
            message_id,
            data,
            checksum,
        },
    ))
}

pub fn parse_nmea_sentence<'a>(sentence: &'a [u8]) -> Result<NmeaSentence, NmeaError<'a>> {
    /*
     * From gpsd:
     * We've had reports that on the Garmin GPS-10 the device sometimes
     * (1:1000 or so) sends garbage packets that have a valid checksum
     * but are like 2 successive NMEA packets merged together in one
     * with some fields lost.  Usually these are much longer than the
     * legal limit for NMEA, so we can cope by just tossing out overlong
     * packets.  This may be a generic bug of all Garmin chipsets.
     * NMEA 3.01, Section 5.3 says the max sentence length shall be
     * 82 chars, including the leading $ and terminating \r\n.
     *
     * Some receivers (TN-200, GSW 2.3.2) emit oversized sentences.
     * The Trimble BX-960 receiver emits a 91-character GGA message.
     * The current hog champion is the Skytraq S2525F8 which emits
     * a 100-character PSTI message.
     */
    if sentence.len() > 102 {
        Err(NmeaError::SentenceLength(sentence.len()))
    } else {
        Ok(do_parse_nmea_sentence(sentence)?.1)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseResult {
    GGA(GgaData),
    RMC(RmcData),
    GSV(GsvData),
    GSA(GsaData),
    VTG(VtgData),
    GLL(GllData),
    Unsupported(SentenceType),
}

#[derive(Debug, Clone, PartialEq)]
pub enum NmeaError<'a> {
    /// The provided input was not a proper UTF-8 string
    Utf8DecodingError,
    /// The checksum of the sentence was corrupt or wrong
    ChecksumMismatch,
    /// For some reason a sentence was passed to the wrong sentence specific parser, this error
    /// should never happen. First slice is the expected header, second is the found one
    WrongSentenceHeader(&'a [u8], &'a [u8]),
    /// The sentence could not be parsed because its format was invalid
    ParsingError(nom::Err<(&'a [u8], nom::error::ErrorKind)>),
    /// The sentence was too long to be parsed, our current limit is 102 characters
    SentenceLength(usize),
    /// The type of a GSV sentence was not a valid Gnss type
    InvalidGnssType,
    /// The sentence has and maybe will never be implemented
    Unsupported(SentenceType),
    /// The provided navigation configuration was empty and thus invalid
    EmptyNavConfig,
}

impl<'a> From<nom::Err<(&'a [u8], nom::error::ErrorKind)>> for NmeaError<'a> {
    fn from(error: nom::Err<(&'a [u8], nom::error::ErrorKind)>) -> Self {
        Self::ParsingError(error)
    }
}

/// parse nmea 0183 sentence and extract data from it
pub fn parse(xs: &[u8]) -> Result<ParseResult, NmeaError> {
    let nmea_sentence = parse_nmea_sentence(xs)?;

    if nmea_sentence.checksum == nmea_sentence.calc_checksum() {
        match SentenceType::try_from(nmea_sentence.message_id)? {
            SentenceType::GGA => Ok(ParseResult::GGA(parse_gga(nmea_sentence)?)),
            SentenceType::GSV => Ok(ParseResult::GSV(parse_gsv(nmea_sentence)?)),
            SentenceType::RMC => Ok(ParseResult::RMC(parse_rmc(nmea_sentence)?)),
            SentenceType::GSA => Ok(ParseResult::GSA(parse_gsa(nmea_sentence)?)),
            SentenceType::VTG => Ok(ParseResult::VTG(parse_vtg(nmea_sentence)?)),
            SentenceType::GLL => Ok(ParseResult::GLL(parse_gll(nmea_sentence)?)),
            msg_id => Ok(ParseResult::Unsupported(msg_id)),
        }
    } else {
        Err(NmeaError::ChecksumMismatch)
    }
}
