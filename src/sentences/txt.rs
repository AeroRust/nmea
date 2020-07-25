use arrayvec::ArrayString;
use nom::character::complete::char;
use nom::{bytes::complete::take_while, IResult};

use super::utils::number;
use crate::{NmeaError, parse::NmeaSentence};

const MAX_LEN: usize = 64;

/// Parse TXT message from u-blox device
///
/// $GNTXT,01,01,02,u-blox AG - www.u-blox.com*4E
/// 1   01  Total number of messages in this transmission, 01..99
/// 2   01  Message number in this transmission, range 01..xx
/// 3   02  Text identifier, u-blox GPS receivers specify the severity of the message with this number. 00 = ERROR, 01 = WARNING, 02 = NOTICE, 07 = USER
/// 4   u-blox AG - www.u-blox.com  Any ASCII text
/// *68        mandatory nmea_checksum
pub fn parse_txt(s: NmeaSentence) -> Result<TxtData, NmeaError> {
    if s.message_id != b"TXT" {
        return Err(NmeaError::WrongSentenceHeader{expected: b"TXT", found: s.message_id});
    }

    let ret = do_parse_txt(s.data).map_err(|err| NmeaError::ParsingError(err))?.1;

    let text_str = core::str::from_utf8(ret.text).map_err(|_e| NmeaError::Utf8DecodingError)?;

    let text = ArrayString::from(text_str).map_err(|_e| NmeaError::SentenceLength(text_str.len()))?;

    Ok(TxtData {
        count: ret.count,
        seq: ret.seq,
        text_ident: ret.text_ident,
        text,
    })
}

fn txt_str(s: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while(|c| c != b',' && c != b'*')(s)
}

fn do_parse_txt<'a>(i: &'a [u8]) -> IResult<&'a [u8], TxtData0<'a>> {
    let (i, count) = number::<u8>(i)?;
    let (i, _) = char(',')(i)?;
    let (i, seq) = number::<u8>(i)?;
    let (i, _) = char(',')(i)?;
    let (i, text_ident) = number::<u8>(i)?;
    let (i, _) = char(',')(i)?;
    let (i, text) = txt_str(i)?;

    Ok((
        i,
        TxtData0 {
            count,
            seq,
            text_ident,
            text,
        },
    ))
}

#[derive(Clone, Debug, PartialEq)]
pub struct TxtData {
    pub count: u8,
    pub seq: u8,
    pub text_ident: u8,
    pub text: ArrayString<[u8; MAX_LEN]>,
}

struct TxtData0<'a> {
    pub count: u8,
    pub seq: u8,
    pub text_ident: u8,
    pub text: &'a [u8],
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::parse::parse_nmea_sentence;

    #[test]
    fn smoke_test_parse_txt() {
        let s = parse_nmea_sentence(b"$GNTXT,01,01,02,u-blox AG - www.u-blox.com*4E").unwrap();
        let txt = parse_txt(s).unwrap();
        assert_eq!(
            TxtData {
                count: 1,
                seq: 1,
                text_ident: 2,
                text: ArrayString::from("u-blox AG - www.u-blox.com").unwrap(),
            },
            txt
        );

        let gsa_examples = [
            "$GPTXT,01,01,02,u-blox ag - www.u-blox.com*50",
            "$GPTXT,01,01,02,HW  UBX-G70xx   00070000 FF7FFFFFo*69",
            "$GPTXT,01,01,02,ROM CORE 1.00 (59842) Jun 27 2012 17:43:52*59",
            "$GPTXT,01,01,02,PROTVER 14.00*1E",
            "$GPTXT,01,01,02,ANTSUPERV=AC SD PDoS SR*20",
            "$GPTXT,01,01,02,ANTSTATUS=OK*3B",
            "$GPTXT,01,01,02,LLC FFFFFFFF-FFFFFFFF-FFFFFFFF-FFFFFFFF-FFFFFFFD*2C",
        ];
        for line in &gsa_examples {
            println!("we parse line '{}'", line);
            let s = parse_nmea_sentence(line.as_bytes()).unwrap();
            parse_txt(s).unwrap();
        }
    }
}
