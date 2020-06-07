use nom::character::complete::char;

use nom::{bytes::complete::take_while, IResult};

use super::utils::number;
use crate::parse::NmeaSentence;

/// Parse TXT message from u-blox device
///
/// $GNTXT,01,01,02,u-blox AG - www.u-blox.com*4E
/// 1   01  Total number of messages in this transmission, 01..99
/// 2   01  Message number in this transmission, range 01..xx
/// 3   02  Text identifier, u-blox GPS receivers specify the severity of the message with this number. 00 = ERROR, 01 = WARNING, 02 = NOTICE, 07 = USER
/// 4   u-blox AG - www.u-blox.com  Any ASCII text
/// *68        mandatory nmea_checksum
pub fn parse_txt(s: &NmeaSentence) -> Result<TxtData, String> {
    if s.message_id != b"TXT" {
        return Err("TXT message should starts with $..TXT".into());
    }

    let ret = do_parse_txt(s.data)
        .map(|(_, data)| data)
        .map_err(|err| match err {
            nom::Err::Incomplete(_) => "Incomplete nmea sentence".to_string(),
            nom::Err::Error((_, kind)) | nom::Err::Failure((_, kind)) => {
                kind.description().to_string()
            }
        })?;
    Ok(ret)
}

fn txt_str(s: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while(|c| c != b',' && c != b'*')(s)
}

fn do_parse_txt(i: &[u8]) -> IResult<&[u8], TxtData> {
    let (i, count) = number::<u8>(i)?;
    let (i, _) = char(',')(i)?;
    let (i, seq) = number::<u8>(i)?;
    let (i, _) = char(',')(i)?;
    let (i, text_ident) = number::<u8>(i)?;
    let (i, _) = char(',')(i)?;
    let (i, text) = txt_str(i)?;

    Ok((
        i,
        TxtData {
            count,
            seq,
            text_ident,
            text: std::str::from_utf8(text).unwrap().to_owned(),
        },
    ))
}

#[derive(Clone, Debug, PartialEq)]
pub struct TxtData {
    pub count: u8,
    pub seq: u8,
    pub text_ident: u8,
    pub text: String,
}
