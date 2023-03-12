use chrono::NaiveTime;
use nom::{
    bytes::complete::take,
    character::complete::char,
    combinator::{map_res, opt},
    IResult,
};

use crate::{parse::NmeaSentence, sentences::utils::parse_hms, Error, SentenceType};

use super::utils::{parse_num, parse_number_in_range};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZdaData {
    pub utc_time: Option<NaiveTime>,
    pub day: Option<u8>,
    pub month: Option<u8>,
    pub year: Option<u16>,
    pub local_zone: Option<i8>,
    pub local_zone_minutes_description: Option<i8>,
}

fn do_parse_zda(i: &str) -> IResult<&str, ZdaData> {
    let comma = char(',');
    let (i, utc_time) = opt(parse_hms)(i)?;
    let (i, _) = comma(i)?;
    let (i, day) = opt(|i| parse_number_in_range::<u8>(i, 1, 31))(i)?;
    let (i, _) = comma(i)?;
    let (i, month) = opt(|i| parse_number_in_range::<u8>(i, 1, 12))(i)?;
    let (i, _) = comma(i)?;
    let (i, year) = opt(map_res(take(4usize), parse_num::<u16>))(i)?;
    let (i, _) = comma(i)?;
    let (i, minus) = opt(char('-'))(i)?;
    let signum = minus.map(|_| -1).unwrap_or(1);
    let (i, local_zone) = opt(|i| parse_number_in_range::<i8>(i, 0, 13))(i)?;
    let local_zone = local_zone.map(|z| z * signum);
    let (i, _) = comma(i)?;
    let (i, local_zone_minutes_description) = opt(|i| parse_number_in_range::<i8>(i, -59, 59))(i)?;
    let local_zone_minutes_description = local_zone_minutes_description.map(|m| m * signum);

    Ok((
        i,
        ZdaData {
            utc_time,
            day,
            month,
            year,
            local_zone,
            local_zone_minutes_description,
        },
    ))
}

pub fn parse_zda(sentence: NmeaSentence) -> Result<ZdaData, Error> {
    if sentence.message_id != SentenceType::ZDA {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::ZDA,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_zda(sentence.data)?.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_nmea_sentence;

    fn assert_zda_sentence(sentence: &str, checksum: u8, expected: ZdaData) {
        let s = parse_nmea_sentence(sentence).unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, checksum);
        let zda_data = parse_zda(s).unwrap();
        assert_eq!(zda_data, expected);
    }

    #[test]
    fn test_parse_zda() {
        assert_zda_sentence(
            "$GPZDA,160012.71,11,03,2004,-1,00*7D",
            0x7d,
            ZdaData {
                utc_time: Some(NaiveTime::from_hms_milli_opt(16, 00, 12, 710).unwrap()),
                day: Some(11),
                month: Some(3),
                year: Some(2004),
                local_zone: Some(-1),
                local_zone_minutes_description: Some(0),
            },
        );

        assert_zda_sentence(
            "$GPZDA,,,,,,*48",
            0x48,
            ZdaData {
                utc_time: None,
                day: None,
                month: None,
                year: None,
                local_zone: None,
                local_zone_minutes_description: None,
            },
        );

        assert_zda_sentence(
            "$GPZDA,,,,,-1,5*61",
            0x61,
            ZdaData {
                utc_time: None,
                day: None,
                month: None,
                year: None,
                local_zone: Some(-1),
                local_zone_minutes_description: Some(-5),
            },
        );

        assert_zda_sentence(
            "$GPZDA,,,,,,21*4B",
            0x4b,
            ZdaData {
                utc_time: None,
                day: None,
                month: None,
                year: None,
                local_zone: None,
                local_zone_minutes_description: Some(21),
            },
        );
    }
}
