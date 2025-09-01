use core::fmt;

use crate::{SentenceType, sentences::GnssType};

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
    ParsingError(
        #[cfg_attr(feature = "defmt", defmt(Debug2Format))] nom::Err<nom::error::Error<&'a str>>,
    ),
    /// The sentence was too long to be parsed, our current limit is `SENTENCE_MAX_LEN` characters.
    SentenceLength(usize),
    /// Parameter was too long to fit into fixed ArrayString.
    ParameterLength {
        max_length: usize,
        parameter_length: usize,
    },
    /// The sentence is recognized but it is not supported by the crate.
    Unsupported(SentenceType),
    /// The sentence type is unknown for this crate.
    Unknown(&'a str),
    /// The provided navigation configuration was empty and thus invalid
    EmptyNavConfig,
    /// An unknown talker ID was found in the NMEA message.
    UnknownTalkerId { expected: &'a str, found: &'a str },
    /// The current sentences is parsable but the feature has been disabled.
    // TODO: Add sentences and data?!
    DisabledSentence,
}

impl<'a> From<nom::Err<nom::error::Error<&'a str>>> for Error<'a> {
    fn from(error: nom::Err<nom::error::Error<&'a str>>) -> Self {
        Self::ParsingError(error)
    }
}

impl fmt::Display for Error<'_> {
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
            Error::ParameterLength {
                max_length,
                parameter_length: _,
            } => write!(
                f,
                "Parameter was too long to fit into string, max length is {}",
                max_length
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
            Error::UnknownTalkerId { expected, found } => write!(
                f,
                "Unknown Talker ID (expected = '{}', found = '{}')",
                expected, found
            ),
            Error::DisabledSentence => {
                write!(f, "Sentence is parsable but it's feature is disabled",)
            }
        }
    }
}

impl core::error::Error for Error<'_> {}
