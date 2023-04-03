use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};
use nom::{
    bytes::complete::take,
    character::complete::char,
    combinator::{map_res, opt},
    IResult,
};

use crate::{parse::NmeaSentence, sentences::utils::parse_hms, Error, SentenceType};

use super::utils::{parse_num, parse_number_in_range};

/// ZDA - Time & Date - UTC, day, month, year and local time zone
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_zda_time_date_utc_day_month_year_and_local_time_zone>
///
/// ```text
///        1         2  3  4    5  6  7
///        |         |  |  |    |  |  |
/// $--ZDA,hhmmss.ss,xx,xx,xxxx,xx,xx*hh<CR><LF>
/// ```
///
/// 1. UTC time (hours, minutes, seconds, may have fractional subseconds)
/// 2. Day, 01 to 31
/// 3. Month, 01 to 12
/// 4. Year (4 digits)
/// 5. Local zone description, 00 to +- 13 hours
/// 6. Local zone minutes description, 00 to 59, apply same sign as local hours
/// 7. Checksum
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZdaData {
    pub utc_time: Option<NaiveTime>,
    pub day: Option<u8>,
    pub month: Option<u8>,
    pub year: Option<u16>,
    pub local_zone_hours: Option<i8>,
    pub local_zone_minutes: Option<i8>,
}

impl ZdaData {
    /// Get UTC date by `day`, `month` and `year` fields.
    /// Returns `None` if any field is `None`.
    pub fn utc_date(&self) -> Option<NaiveDate> {
        if let Some(((day, month), year)) = self.day.zip(self.month).zip(self.year) {
            NaiveDate::from_ymd_opt(year.into(), month.into(), day.into())
        } else {
            None
        }
    }

    /// Get UTC date time by `utc_time`, `day`, `month`, and `year` fields.
    /// Returns `None` if any field is `None`.
    pub fn utc_date_time(&self) -> Option<NaiveDateTime> {
        self.utc_time.and_then(|utc_time| {
            self.utc_date()
                .map(|utc_date| NaiveDateTime::new(utc_date, utc_time))
        })
    }

    /// Get `chrono::FixedOffset` by `local_zone_hours` and `local_zone_minutes` fields.
    /// Return `Some` if either `local_zone_hours` or `local_zone_minutes` is `Some`.
    pub fn offset(&self) -> Option<FixedOffset> {
        let hours = self.local_zone_hours.map(i32::from);
        let minutes = self.local_zone_minutes.map(i32::from);
        match (hours, minutes) {
            (Some(h), Some(m)) => FixedOffset::east_opt(((h * 60) + m) * 60),
            (Some(h), None) => FixedOffset::east_opt(h * 60 * 60),
            (None, Some(m)) => FixedOffset::east_opt(m * 60),
            (None, None) => None,
        }
    }

    /// Caluculate local datetime
    /// Returns `None` if any field is `None`.
    pub fn local_date_time(&self) -> Option<DateTime<FixedOffset>> {
        self.utc_date_time()
            .zip(self.offset())
            .and_then(|(date_time, offset)| date_time.and_local_timezone(offset).single())
    }
}

/// # Parse ZDA message
///
/// From gpsd/driver_nmea0183.c
///
/// ```text
/// $GPZDA,160012.71,11,03,2004,-1,00*7D
/// ```
///
/// 1) UTC time (hours, minutes, seconds, may have fractional subsecond)
/// 2) Day, 01 to 31
/// 3) Month, 01 to 12
/// 4) Year (4 digits)
/// 5) Local zone description, 00 to +- 13 hours
/// 6) Local zone minutes description, apply same sign as local hours
/// 7) Checksum
///
/// Note: some devices, like the u-blox ANTARIS 4h, are known to ship ZDAs
/// with some fields blank under poorly-understood circumstances (probably
/// when they don't have satellite lock yet).
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
    let (i, local_zone_hours) = opt(|i| parse_number_in_range::<i8>(i, 0, 13))(i)?;
    let local_zone_hours = local_zone_hours.map(|z| z * signum);
    let (i, _) = comma(i)?;
    let (i, local_zone_minutes) = opt(|i| parse_number_in_range::<i8>(i, -59, 59))(i)?;
    let local_zone_minutes = local_zone_minutes.map(|m| m * signum);

    Ok((
        i,
        ZdaData {
            utc_time,
            day,
            month,
            year,
            local_zone_hours,
            local_zone_minutes,
        },
    ))
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
                local_zone_hours: Some(-1),
                local_zone_minutes: Some(0),
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
                local_zone_hours: None,
                local_zone_minutes: None,
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
                local_zone_hours: Some(-1),
                local_zone_minutes: Some(-5),
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
                local_zone_hours: None,
                local_zone_minutes: Some(21),
            },
        );
    }

    #[test]
    fn test_wrong_sentence() {
        let invalid_aam_sentence = NmeaSentence {
            message_id: SentenceType::AAM,
            data: "",
            talker_id: "GP",
            checksum: 0,
        };
        assert_eq!(
            Err(Error::WrongSentenceHeader {
                expected: SentenceType::ZDA,
                found: SentenceType::AAM
            }),
            parse_zda(invalid_aam_sentence)
        );
    }

    #[test]
    fn test_parse_zda_datetime() {
        let s = parse_nmea_sentence("$GPZDA,160012.71,11,03,2004,-1,00*7D").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x7d);
        let zda_data = parse_zda(s).unwrap();
        assert_eq!(
            zda_data.utc_date(),
            Some(NaiveDate::from_ymd_opt(2004, 3, 11).unwrap())
        );
        assert_eq!(
            zda_data.utc_date_time(),
            Some(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2004, 3, 11).unwrap(),
                NaiveTime::from_hms_milli_opt(16, 00, 12, 710).unwrap()
            ))
        );
        assert_eq!(
            zda_data.offset(),
            Some(FixedOffset::east_opt(-1 * 60 * 60).unwrap())
        );
        assert_eq!(
            zda_data.local_date_time(),
            Some(DateTime::from_local(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(2004, 3, 11).unwrap(),
                    NaiveTime::from_hms_milli_opt(16, 00, 12, 710).unwrap()
                ),
                FixedOffset::east_opt(-1 * 60 * 60).unwrap()
            ))
        );

        assert_eq!(
            ZdaData {
                utc_time: None,
                day: None,
                month: None,
                year: None,
                local_zone_hours: None,
                local_zone_minutes: None,
            }
            .utc_date(),
            None,
        );

        assert_eq!(
            ZdaData {
                utc_time: None,
                day: None,
                month: None,
                year: None,
                local_zone_hours: None,
                local_zone_minutes: None,
            }
            .offset(),
            None
        );
        assert_eq!(
            ZdaData {
                utc_time: None,
                day: None,
                month: None,
                year: None,
                local_zone_hours: Some(9),
                local_zone_minutes: None,
            }
            .offset(),
            Some(FixedOffset::east_opt(9 * 60 * 60).unwrap()),
        );
        assert_eq!(
            ZdaData {
                utc_time: None,
                day: None,
                month: None,
                year: None,
                local_zone_hours: None,
                local_zone_minutes: Some(20),
            }
            .offset(),
            Some(FixedOffset::east_opt(20 * 60).unwrap()),
        );
        assert_eq!(
            ZdaData {
                utc_time: None,
                day: None,
                month: None,
                year: None,
                local_zone_hours: Some(9),
                local_zone_minutes: Some(20),
            }
            .offset(),
            Some(FixedOffset::east_opt((9 * 60 + 20) * 60).unwrap()),
        );
    }
}
