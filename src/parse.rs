use core::{fmt, str};

use nom::{
    bytes::complete::{take, take_until},
    character::complete::char,
    combinator::map_res,
    sequence::preceded,
    IResult,
};

pub use crate::sentences::*;
use crate::SentenceType;

pub const SENTENCE_MAX_LEN: usize = 102;

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

fn parse_hex(data: &[u8]) -> core::result::Result<u8, &'static str> {
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

pub fn parse_nmea_sentence<'a>(
    sentence: &'a [u8],
) -> core::result::Result<NmeaSentence, NmeaError<'a>> {
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
    if sentence.len() > SENTENCE_MAX_LEN {
        Err(NmeaError::SentenceLength(sentence.len()))
    } else {
        Ok(do_parse_nmea_sentence(sentence)?.1)
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseResult {
    BWC(BwcData),
    GGA(GgaData),
    RMC(RmcData),
    GSV(GsvData),
    GSA(GsaData),
    VTG(VtgData),
    GLL(GllData),
    TXT(TxtData),
    GNS(GnsData),
    Unsupported(SentenceType),
}

#[derive(Debug, PartialEq)]
pub enum NmeaError<'a> {
    /// The provided input was not a proper UTF-8 string
    Utf8DecodingError,
    /// The checksum of the sentence was corrupt or wrong
    ChecksumMismatch { calculated: u8, found: u8 },
    /// For some reason a sentence was passed to the wrong sentence specific parser, this error
    /// should never happen. First slice is the expected header, second is the found one
    WrongSentenceHeader { expected: &'a [u8], found: &'a [u8] },
    /// The sentence could not be parsed because its format was invalid
    ParsingError(nom::Err<nom::error::Error<&'a [u8]>>),
    /// The sentence was too long to be parsed, our current limit is `SENTENCE_MAX_LEN` characters
    SentenceLength(usize),
    /// The sentence has and maybe will never be implemented
    Unsupported(SentenceType),
    /// The provided navigation configuration was empty and thus invalid
    EmptyNavConfig,
    /// invalid senetence number field in nmea sentence of type GSV
    InvalidGsvSentenceNum,
}

impl<'a> From<nom::Err<nom::error::Error<&'a [u8]>>> for NmeaError<'a> {
    fn from(error: nom::Err<nom::error::Error<&'a [u8]>>) -> Self {
        Self::ParsingError(error)
    }
}

impl<'a> fmt::Display for NmeaError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NmeaError::Utf8DecodingError => {
                write!(f, "The provided input was not a valid UTF-8 string")
            }
            NmeaError::ChecksumMismatch { calculated, found } => write!(
                f,
                "Checksum Mismatch(calculated = {}, found = {})",
                calculated, found
            ),
            NmeaError::WrongSentenceHeader { expected, found } => write!(
                f,
                "Wrong Sentence Header (expected = {:?}, found = {:?})",
                expected, found
            ),
            NmeaError::ParsingError(e) => write!(f, "Parse error: {}", e),
            NmeaError::SentenceLength(size) => write!(
                f,
                "The sentence was too long to be parsed, current limit is {} characters",
                size
            ),
            NmeaError::Unsupported(sentence) => {
                write!(f, "Unsupported NMEA sentence {:?}", sentence)
            }
            NmeaError::EmptyNavConfig => write!(
                f,
                "The provided navigation configuration was empty and thus invalid"
            ),
            NmeaError::InvalidGsvSentenceNum => write!(
                f,
                "Invalid senetence number field in nmea sentence of type GSV"
            ),
        }
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl<'a> std::error::Error for NmeaError<'a> {}

/// Parse NMEA 0183 sentence and extract data from it
pub fn parse(xs: &[u8]) -> Result<ParseResult, NmeaError> {
    let nmea_sentence = parse_nmea_sentence(xs)?;
    let calculated_checksum = nmea_sentence.calc_checksum();

    if nmea_sentence.checksum == calculated_checksum {
        match SentenceType::from_slice(nmea_sentence.message_id) {
            SentenceType::BWC => {
                let data = parse_bwc(nmea_sentence)?;
                Ok(ParseResult::BWC(data))
            }
            SentenceType::GGA => {
                let data = parse_gga(nmea_sentence)?;
                Ok(ParseResult::GGA(data))
            }
            SentenceType::GSV => {
                let data = parse_gsv(nmea_sentence)?;
                Ok(ParseResult::GSV(data))
            }
            SentenceType::RMC => {
                let data = parse_rmc(nmea_sentence)?;
                Ok(ParseResult::RMC(data))
            }
            SentenceType::GSA => Ok(ParseResult::GSA(parse_gsa(nmea_sentence)?)),
            SentenceType::VTG => Ok(ParseResult::VTG(parse_vtg(nmea_sentence)?)),
            SentenceType::GLL => Ok(ParseResult::GLL(parse_gll(nmea_sentence)?)),
            SentenceType::TXT => Ok(ParseResult::TXT(parse_txt(nmea_sentence)?)),
            SentenceType::GNS => Ok(ParseResult::GNS(parse_gns(nmea_sentence)?)),
            msg_id => Ok(ParseResult::Unsupported(msg_id)),
        }
    } else {
        Err(NmeaError::ChecksumMismatch {
            calculated: calculated_checksum,
            found: nmea_sentence.checksum,
        })
    }
}
