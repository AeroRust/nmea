use core::str;

use nom::{
    bytes::complete::{take, take_until},
    character::complete::char,
    combinator::map_res,
    sequence::preceded,
    IResult,
};

use cfg_if::cfg_if;

use crate::{sentences::*, Error, SentenceType};

/// The maximum message length parsable by the crate.
///
/// From `gpsd`:
///
/// > We've had reports that on the Garmin GPS-10 the device sometimes
/// (1:1000 or so) sends garbage packets that have a valid checksum
/// but are like 2 successive NMEA packets merged together in one
/// with some fields lost. Usually these are much longer than the
/// legal limit for NMEA, so we can cope by just tossing out overlong
/// packets.  This may be a generic bug of all Garmin chipsets.
/// NMEA 3.01, Section 5.3 says the max sentence length shall be
/// 82 chars, including the leading $ and terminating \r\n.
///
/// > Some receivers (TN-200, GSW 2.3.2) emit oversized sentences.
/// The Trimble BX-960 receiver emits a 91-character GGA message.
/// The current hog champion is the Skytraq S2525F8 which emits
/// a 100-character PSTI message.
pub const SENTENCE_MAX_LEN: usize = 102;

/// A known and parsable Nmea sentence type.
pub struct NmeaSentence<'a> {
    pub talker_id: &'a str,
    pub message_id: SentenceType,
    pub data: &'a str,
    pub checksum: u8,
}

impl<'a> NmeaSentence<'a> {
    pub fn calc_checksum(&self) -> u8 {
        checksum(
            self.talker_id
                .as_bytes()
                .iter()
                .chain(self.message_id.as_str().as_bytes())
                .chain(&[b','])
                .chain(self.data.as_bytes()),
        )
    }
}

pub(crate) fn checksum<'a, I: Iterator<Item = &'a u8>>(bytes: I) -> u8 {
    bytes.fold(0, |c, x| c ^ *x)
}

fn parse_hex(data: &str) -> Result<u8, &'static str> {
    u8::from_str_radix(data, 16).map_err(|_| "Failed to parse checksum as hex number")
}

fn parse_checksum(i: &str) -> IResult<&str, u8> {
    map_res(preceded(char('*'), take(2usize)), parse_hex)(i)
}

fn parse_sentence_type(i: &str) -> IResult<&str, SentenceType> {
    map_res(take(3usize), |sentence_type: &str| {
        SentenceType::try_from(sentence_type).map_err(|_| "Unknown sentence type")
    })(i)
}

fn do_parse_nmea_sentence(i: &str) -> IResult<&str, NmeaSentence> {
    let (i, talker_id) = preceded(char('$'), take(2usize))(i)?;
    let (i, message_id) = parse_sentence_type(i)?;
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

pub fn parse_nmea_sentence(sentence: &str) -> core::result::Result<NmeaSentence, Error<'_>> {
    if sentence.len() > SENTENCE_MAX_LEN {
        Err(Error::SentenceLength(sentence.len()))
    } else {
        Ok(do_parse_nmea_sentence(sentence)?.1)
    }
}

/// The result of parsing a single NMEA message.
#[derive(Debug, PartialEq)]
pub enum ParseResult {
    AAM(AamData),
    ALM(AlmData),
    BOD(BodData),
    BWC(BwcData),
    BWW(BwwData),
    DBK(DbkData),
    GBS(GbsData),
    GGA(GgaData),
    GLL(GllData),
    GNS(GnsData),
    GSA(GsaData),
    GSV(GsvData),
    HDT(HdtData),
    MDA(MdaData),
    MTW(MtwData),
    MWV(MwvData),
    RMC(RmcData),
    TXT(TxtData),
    VHW(VhwData),
    VTG(VtgData),
    ZDA(ZdaData),
    ZFO(ZfoData),
    ZTG(ZtgData),
    PGRMZ(PgrmzData),
    /// A message that is not supported by the crate and cannot be parsed.
    Unsupported(SentenceType),
}

impl From<&ParseResult> for SentenceType {
    fn from(parse_result: &ParseResult) -> Self {
        match parse_result {
            ParseResult::AAM(_) => SentenceType::AAM,
            ParseResult::ALM(_) => SentenceType::ALM,
            ParseResult::BOD(_) => SentenceType::BOD,
            ParseResult::BWC(_) => SentenceType::BWC,
            ParseResult::BWW(_) => SentenceType::BWW,
            ParseResult::DBK(_) => SentenceType::DBK,
            ParseResult::GBS(_) => SentenceType::GBS,
            ParseResult::GGA(_) => SentenceType::GGA,
            ParseResult::GLL(_) => SentenceType::GLL,
            ParseResult::GNS(_) => SentenceType::GNS,
            ParseResult::GSA(_) => SentenceType::GSA,
            ParseResult::GSV(_) => SentenceType::GSV,
            ParseResult::HDT(_) => SentenceType::HDT,
            ParseResult::MDA(_) => SentenceType::MDA,
            ParseResult::MTW(_) => SentenceType::MTW,
            ParseResult::MWV(_) => SentenceType::MWV,
            ParseResult::RMC(_) => SentenceType::RMC,
            ParseResult::TXT(_) => SentenceType::TXT,
            ParseResult::VHW(_) => SentenceType::VHW,
            ParseResult::VTG(_) => SentenceType::VTG,
            ParseResult::ZFO(_) => SentenceType::ZFO,
            ParseResult::ZTG(_) => SentenceType::ZTG,
            ParseResult::PGRMZ(_) => SentenceType::RMZ,
            ParseResult::ZDA(_) => SentenceType::ZDA,
            ParseResult::Unsupported(sentence_type) => *sentence_type,
        }
    }
}

/// Parse a NMEA 0183 sentence from bytes and extract data from it.
///
/// # Errors
///
/// Apart from errors returned by the message parsing itself, it will return
/// [`Error::Utf8Decoding`] when the bytes are not a valid UTF-8 string.
pub fn parse_bytes(sentence_input: &[u8]) -> Result<ParseResult, Error> {
    let string = core::str::from_utf8(sentence_input).map_err(|_err| Error::Utf8Decoding)?;

    parse_str(string)
}

/// Parse a NMEA 0183 sentence from a string slice and extract data from it.
///
/// Should not contain `\r\n` ending.
///
/// # Errors
///
/// - [`Error::ASCII`] when string contains non-ASCII characters.
pub fn parse_str(sentence_input: &str) -> Result<ParseResult, Error> {
    if !sentence_input.is_ascii() {
        return Err(Error::ASCII);
    }

    let nmea_sentence = parse_nmea_sentence(sentence_input)?;
    let calculated_checksum = nmea_sentence.calc_checksum();

    if nmea_sentence.checksum == calculated_checksum {
        // Ordered alphabetically
        match nmea_sentence.message_id {
            SentenceType::AAM => {
                cfg_if! {
                    if #[cfg(feature = "AAM")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_aam(nmea_sentence).map(ParseResult::AAM)
                    }
                }
            }
            SentenceType::ALM => {
                cfg_if! {
                    if #[cfg(feature = "ALM")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_alm(nmea_sentence).map(ParseResult::ALM)
                    }
                }
            }
            SentenceType::BOD => {
                cfg_if! {
                    if #[cfg(feature = "BOD")] {
                        return Err(Error::DisabledSentence);
                    } else {
                parse_bod(nmea_sentence).map(ParseResult::BOD)
                    }
                }
            }
            SentenceType::BWC => {
                cfg_if! {
                    if #[cfg(feature = "BWC")] {
                        return Err(Error::DisabledSentence);
                    } else {
                    parse_bwc(nmea_sentence).map(ParseResult::BWC)
                    }
                }
            }
            SentenceType::BWW => {
                cfg_if! {
                    if #[cfg(feature = "BWW")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_bww(nmea_sentence).map(ParseResult::BWW)
                    }
                }
            }
            SentenceType::DBK => {
                cfg_if! {
                    if #[cfg(feature = "DBK")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_dbk(nmea_sentence).map(Into::into)
                    }
                }
            }
            SentenceType::GBS => {
                cfg_if! {
                    if #[cfg(feature = "GBS")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_gbs(nmea_sentence).map(ParseResult::GBS)
                    }
                }
            }
            SentenceType::GGA => {
                cfg_if! {
                    if #[cfg(feature = "GGA")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_gga(nmea_sentence).map(ParseResult::GGA)
                    }
                }
            }
            SentenceType::GLL => {
                cfg_if! {
                    if #[cfg(feature = "GLL")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_gll(nmea_sentence).map(ParseResult::GLL)
                    }
                }
            }
            SentenceType::GNS => {
                cfg_if! {
                    if #[cfg(feature = "GNS")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_gns(nmea_sentence).map(ParseResult::GNS)
                    }
                }
            }
            SentenceType::GSA => {
                cfg_if! {
                    if #[cfg(feature = "GSA")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_gsa(nmea_sentence).map(ParseResult::GSA)
                    }
                }
            }
            SentenceType::GSV => {
                cfg_if! {
                    if #[cfg(feature = "GSV")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_gsv(nmea_sentence).map(ParseResult::GSV)
                    }
                }
            }
            SentenceType::HDT => {
                cfg_if! {
                    if #[cfg(feature = "HDT")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_hdt(nmea_sentence).map(ParseResult::HDT)
                    }
                }
            }
            SentenceType::MDA => {
                cfg_if! {
                    if #[cfg(feature = "MDA")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_mda(nmea_sentence).map(ParseResult::MDA)
                    }
                }
            }
            SentenceType::MTW => {
                cfg_if! {
                    if #[cfg(feature = "MTW")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_mtw(nmea_sentence).map(ParseResult::MTW)
                    }
                }
            }
            SentenceType::MWV => {
                cfg_if! {
                    if #[cfg(feature = "MWV")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_mwv(nmea_sentence).map(ParseResult::MWV)
                    }
                }
            }
            SentenceType::RMC => {
                cfg_if! {
                    if #[cfg(feature = "RMC")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_rmc(nmea_sentence).map(ParseResult::RMC)
                    }
                }
            }
            SentenceType::RMZ => {
                cfg_if! {
                    if #[cfg(feature = "RMZ")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_pgrmz(nmea_sentence).map(ParseResult::PGRMZ)
                    }
                }
            }
            SentenceType::TXT => {
                cfg_if! {
                    if #[cfg(feature = "TXT")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_txt(nmea_sentence).map(ParseResult::TXT)
                    }
                }
            }
            SentenceType::VHW => {
                cfg_if! {
                    if #[cfg(feature = "VHW")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_vhw(nmea_sentence).map(ParseResult::VHW)
                    }
                }
            }
            SentenceType::VTG => {
                cfg_if! {
                    if #[cfg(feature = "VTG")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_vtg(nmea_sentence).map(ParseResult::VTG)
                    }
                }
            }
            SentenceType::ZDA => {
                cfg_if! {
                    if #[cfg(feature = "ZDA")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_zda(nmea_sentence).map(ParseResult::ZDA)
                    }
                }
            }
            SentenceType::ZFO => {
                cfg_if! {
                    if #[cfg(feature = "ZFO")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_zfo(nmea_sentence).map(ParseResult::ZFO)
                    }
                }
            }
            SentenceType::ZTG => {
                cfg_if! {
                    if #[cfg(feature = "ZTG")] {
                        return Err(Error::DisabledSentence);
                    } else {
                        parse_ztg(nmea_sentence).map(ParseResult::ZTG)
                    }
                }
            }
            sentence_type => Ok(ParseResult::Unsupported(sentence_type)),
        }
    } else {
        Err(Error::ChecksumMismatch {
            calculated: calculated_checksum,
            found: nmea_sentence.checksum,
        })
    }
}
