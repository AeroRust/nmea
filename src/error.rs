use core::fmt;

use crate::{sentences::GnssType, SentenceType};

#[derive(Debug, PartialEq)]
pub enum Error<'a> {
    /// The provided input was not a proper UTF-8 string
    Utf8Decoding,
    /// The provided string message contains other apart from ASCII.
    ASCII,
    /// The checksum of the sentence was corrupt or wrong
    ChecksumMismatch { calculated: u8, found: u8 },
    /// For some reason a sentence was passed to the wrong sentence specific parser, this error
    /// should never happen. First slice is the expected header, second is the found one
    WrongSentenceHeader {
        expected: SentenceType,
        found: SentenceType,
    },
    /// An unknown [`GnssType`] was found in the NMEA message.
    UnknownGnssType(&'a str),
    /// The sentence could not be parsed because its format was invalid.
    ParsingError(nom::Err<nom::error::Error<&'a str>>),
    /// The sentence was too long to be parsed, our current limit is `SENTENCE_MAX_LEN` characters.
    SentenceLength(usize),
    /// The sentence is recognized but it is not supported by the crate.
    Unsupported(SentenceType),
    /// The sentence type is unknown for this crate.
    Unknown(&'a str),
    /// The provided navigation configuration was empty and thus invalid
    EmptyNavConfig,
    /// Invalid sentence number field in nmea sentence of type GSV
    InvalidGsvSentenceNum,
    /// An unknown talker ID was found in the NMEA message.
    UnknownTalkerId { expected: &'a str, found: &'a str },
}

impl<'a> From<nom::Err<nom::error::Error<&'a str>>> for Error<'a> {
    fn from(error: nom::Err<nom::error::Error<&'a str>>) -> Self {
        Self::ParsingError(error)
    }
}

impl<'a> fmt::Display for Error<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Utf8Decoding => {
                write!(f, "The provided input was not a valid UTF-8 string")
            }
            Error::ASCII => write!(f, "Provided input includes non-ASCII characters"),
            Error::ChecksumMismatch { calculated, found } => write!(
                f,
                "Checksum Mismatch(calculated = {}, found = {})",
                calculated, found
            ),
            Error::WrongSentenceHeader { expected, found } => write!(
                f,
                "Wrong Sentence Header (expected = '{}', found = '{}')",
                expected, found
            ),
            Error::UnknownGnssType(found) => write!(
                f,
                "Unknown GNSS type (expected one of '{:?}', found = '{}')",
                GnssType::ALL_TYPES,
                found
            ),
            Error::ParsingError(e) => write!(f, "Parse error: {}", e),
            Error::SentenceLength(size) => write!(
                f,
                "The sentence was too long to be parsed, current limit is {} characters",
                size
            ),
            Error::Unsupported(sentence) => {
                write!(f, "Unsupported NMEA sentence '{}'", sentence)
            }
            Error::Unknown(sentence) => {
                write!(f, "Unknown for the crate NMEA sentence '{}'", sentence)
            }
            Error::EmptyNavConfig => write!(
                f,
                "The provided navigation configuration was empty and thus invalid"
            ),
            Error::InvalidGsvSentenceNum => write!(
                f,
                "Invalid senetence number field in nmea sentence of type GSV"
            ),
            Error::UnknownTalkerId { expected, found } => write!(
                f,
                "Unknown Talker ID (expected = '{}', found = '{}')",
                expected, found
            ),
        }
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl<'a> std::error::Error for Error<'a> {}
