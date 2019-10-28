use std::str;

use chrono::{NaiveDate, NaiveTime};
use nom::branch::alt;
use nom::bytes::complete::take_while1;
use nom::character::complete::{char, digit1};
use nom::combinator::{all_consuming, cond, map_res, opt, rest_len};
use nom::error::ErrorKind;
use nom::sequence::tuple;
use nom::{
    alt, char, complete, do_parse, error_position, many0, map_res, named, one_of, opt, tag, take,
    take_until, AsChar, IResult,
};

use crate::{FixType, GnssType, Satellite, SentenceType};

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

pub struct GsvData {
    pub gnss_type: GnssType,
    pub number_of_sentences: u16,
    pub sentence_num: u16,
    pub _sats_in_view: u16,
    pub sats_info: [Option<Satellite>; 4],
}

pub fn checksum<'a, I: Iterator<Item = &'a u8>>(bytes: I) -> u8 {
    bytes.fold(0, |c, x| c ^ *x)
}

fn construct_sentence<'a>(
    data: (&'a [u8], &'a [u8], &'a [u8], u8),
) -> std::result::Result<NmeaSentence<'a>, &'static str> {
    Ok(NmeaSentence {
        talker_id: data.0,
        message_id: data.1,
        data: data.2,
        checksum: data.3,
    })
}

fn parse_hex(data: &[u8]) -> std::result::Result<u8, &'static str> {
    u8::from_str_radix(unsafe { str::from_utf8_unchecked(data) }, 16)
        .map_err(|_| "Failed to parse checksum as hex number")
}

named!(
    parse_checksum<u8>,
    map_res!(
        do_parse!(char!('*') >> checksum_bytes: take!(2) >> (checksum_bytes)),
        parse_hex
    )
);

named!(
    do_parse_nmea_sentence<NmeaSentence>,
    map_res!(
        do_parse!(
            char!('$')
                >> talker_id: take!(2)
                >> message_id: take!(3)
                >> char!(',')
                >> data: take_until!("*")
                >> cs: parse_checksum
                >> (talker_id, message_id, data, cs)
        ),
        construct_sentence
    )
);

pub fn parse_nmea_sentence(sentence: &[u8]) -> std::result::Result<NmeaSentence, String> {
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
        return Err("Too long message".to_string());
    }
    let res: NmeaSentence = do_parse_nmea_sentence(sentence)
        .map_err(|err| match err {
            nom::Err::Incomplete(_) => "Incomplete nmea sentence".to_string(),
            nom::Err::Error((_, kind)) | nom::Err::Failure((_, kind)) => {
                kind.description().to_string()
            }
        })?
        .1;
    Ok(res)
}

fn parse_num<I: std::str::FromStr>(data: &[u8]) -> std::result::Result<I, &'static str> {
    //    println!("parse num {}", unsafe { str::from_utf8_unchecked(data) });
    str::parse::<I>(unsafe { str::from_utf8_unchecked(data) }).map_err(|_| "parse of number failed")
}

fn construct_satellite(
    data: (u32, Option<i32>, Option<i32>, Option<i32>),
) -> std::result::Result<Satellite, &'static str> {
    //    println!("we construct sat {}", data.0);
    Ok(Satellite {
        gnss_type: GnssType::Galileo,
        prn: data.0,
        elevation: data.1.map(|v| v as f32),
        azimuth: data.2.map(|v| v as f32),
        snr: data.3.map(|v| v as f32),
    })
}

fn parse_gsv_sat_info(i: &[u8]) -> IResult<&[u8], Satellite> {
    map_res(
        tuple((
            map_res(digit1, parse_num::<u32>),
            char(','),
            opt(map_res(digit1, parse_num::<i32>)),
            char(','),
            opt(map_res(digit1, parse_num::<i32>)),
            char(','),
            opt(map_res(digit1, parse_num::<i32>)),
            |i| cond(rest_len(i)?.1 > 0, char(','))(i),
        )),
        |(prn, _, elevation, _, azimuth, _, signal_noise, _)| {
            construct_satellite((prn, elevation, azimuth, signal_noise))
        },
    )(i)
}

fn construct_gsv_data(
    data: (
        u16,
        u16,
        u16,
        Option<Satellite>,
        Option<Satellite>,
        Option<Satellite>,
        Option<Satellite>,
    ),
) -> std::result::Result<GsvData, &'static str> {
    Ok(GsvData {
        gnss_type: GnssType::Galileo,
        number_of_sentences: data.0,
        sentence_num: data.1,
        _sats_in_view: data.2,
        sats_info: [data.3, data.4, data.5, data.6],
    })
}

named!(
    do_parse_gsv<GsvData>,
    map_res!(
        do_parse!(
            number_of_sentences: map_res!(digit1, parse_num::<u16>)
                >> char!(',')
                >> sentence_index: map_res!(digit1, parse_num::<u16>)
                >> char!(',')
                >> total_number_of_sats: map_res!(digit1, parse_num::<u16>)
                >> char!(',')
                >> sat0: opt!(complete!(parse_gsv_sat_info))
                >> sat1: opt!(complete!(parse_gsv_sat_info))
                >> sat2: opt!(complete!(parse_gsv_sat_info))
                >> sat3: opt!(complete!(parse_gsv_sat_info))
                >> (
                    number_of_sentences,
                    sentence_index,
                    total_number_of_sats,
                    sat0,
                    sat1,
                    sat2,
                    sat3
                )
        ),
        construct_gsv_data
    )
);

/// Parsin one GSV sentence
/// from gpsd/driver_nmea0183.c:
/// $IDGSV,2,1,08,01,40,083,46,02,17,308,41,12,07,344,39,14,22,228,45*75
/// 2           Number of sentences for full data
/// 1           Sentence 1 of 2
/// 08          Total number of satellites in view
/// 01          Satellite PRN number
/// 40          Elevation, degrees
/// 083         Azimuth, degrees
/// 46          Signal-to-noise ratio in decibels
/// <repeat for up to 4 satellites per sentence>
///
/// Can occur with talker IDs:
///   BD (Beidou),
///   GA (Galileo),
///   GB (Beidou),
///   GL (GLONASS),
///   GN (GLONASS, any combination GNSS),
///   GP (GPS, SBAS, QZSS),
///   QZ (QZSS).
///
/// GL may be (incorrectly) used when GSVs are mixed containing
/// GLONASS, GN may be (incorrectly) used when GSVs contain GLONASS
/// only.  Usage is inconsistent.
pub fn parse_gsv(sentence: &NmeaSentence) -> Result<GsvData, String> {
    if sentence.message_id != b"GSV" {
        return Err("GSV sentence not starts with $..GSV".into());
    }
    let gnss_type = match sentence.talker_id {
        b"GP" => GnssType::Gps,
        b"GL" => GnssType::Glonass,
        _ => return Err("Unknown GNSS type in GSV sentence".into()),
    };
    //    println!("parse: '{}'", str::from_utf8(sentence.data).unwrap());
    let mut res: GsvData = do_parse_gsv(sentence.data)
        .map_err(|err| match err {
            nom::Err::Incomplete(_) => "Incomplete nmea sentence".to_string(),
            nom::Err::Error((_, kind)) | nom::Err::Failure((_, kind)) => {
                kind.description().to_string()
            }
        })?
        .1;
    res.gnss_type = gnss_type.clone();
    for sat in &mut res.sats_info {
        if let Some(v) = (*sat).as_mut() {
            v.gnss_type = gnss_type.clone();
        }
    }
    Ok(res)
}

#[derive(Debug, PartialEq)]
pub struct GgaData {
    pub fix_time: Option<NaiveTime>,
    pub fix_type: Option<FixType>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub fix_satellites: Option<u32>,
    pub hdop: Option<f32>,
    pub altitude: Option<f32>,
    pub geoid_height: Option<f32>,
}

fn parse_float_num<T: str::FromStr>(input: &[u8]) -> std::result::Result<T, &'static str> {
    let s = str::from_utf8(input).map_err(|_| "invalid float number")?;
    str::parse::<T>(s).map_err(|_| "parse of float number failed")
}

named!(
    parse_hms<NaiveTime>,
    map_res!(
        do_parse!(
            hour: map_res!(take!(2), parse_num::<u32>)
                >> min: map_res!(take!(2), parse_num::<u32>)
                >> sec: map_res!(take_until!(","), parse_float_num::<f64>)
                >> (hour, min, sec)
        ),
        |data: (u32, u32, f64)| -> std::result::Result<NaiveTime, &'static str> {
            if data.2.is_sign_negative() {
                return Err("Invalid time: second is negative");
            }
            if data.0 >= 24 {
                return Err("Invalid time: hour >= 24");
            }
            if data.1 >= 60 {
                return Err("Invalid time: min >= 60");
            }
            Ok(NaiveTime::from_hms_nano(
                data.0,
                data.1,
                data.2.trunc() as u32,
                (data.2.fract() * 1_000_000_000f64).round() as u32,
            ))
        }
    )
);

named!(
    do_parse_lat_lon<(f64, f64)>,
    map_res!(
        do_parse!(
            lat_deg: map_res!(take!(2), parse_num::<u8>)
                >> lat_min: map_res!(float_number, parse_float_num::<f64>)
                >> char!(',')
                >> lat_dir: one_of!("NS")
                >> char!(',')
                >> lon_deg: map_res!(take!(3), parse_num::<u8>)
                >> lon_min: map_res!(float_number, parse_float_num::<f64>)
                >> char!(',')
                >> lon_dir: one_of!("EW")
                >> (lat_deg, lat_min, lat_dir, lon_deg, lon_min, lon_dir)
        ),
        |data: (u8, f64, char, u8, f64, char)| -> std::result::Result<(f64, f64), &'static str> {
            let mut lat = f64::from(data.0) + data.1 / 60.;
            if data.2 == 'S' {
                lat = -lat;
            }
            let mut lon = f64::from(data.3) + data.4 / 60.;
            if data.5 == 'W' {
                lon = -lon;
            }
            Ok((lat, lon))
        }
    )
);

named!(
    parse_lat_lon<Option<(f64, f64)>>,
    alt!(
        complete!(map_res!(tag!(",,,"), |_| -> Result<
            Option<(f64, f64)>,
            &'static str,
        > { Ok(None) }))
            | complete!(map_res!(do_parse_lat_lon, |v| -> Result<
                Option<(f64, f64)>,
                &'static str,
            > { Ok(Some(v)) }))
    )
);

named!(
    do_parse_gga<GgaData>,
    map_res!(
        do_parse!(
            time: opt!(complete!(parse_hms))
                >> char!(',')
                >> lat_lon: parse_lat_lon
                >> char!(',')
                >> fix_quality: one_of!("012345678")
                >> char!(',')
                >> tracked_sats: opt!(complete!(map_res!(digit1, parse_num::<u32>)))
                >> char!(',')
                >> hdop: opt!(complete!(map_res!(float_number, parse_float_num::<f32>)))
                >> char!(',')
                >> altitude:
                    opt!(complete!(map_res!(
                        take_until!(","),
                        parse_float_num::<f32>
                    )))
                >> char!(',')
                >> opt!(complete!(char!('M')))
                >> char!(',')
                >> geoid_height:
                    opt!(complete!(map_res!(
                        take_until!(","),
                        parse_float_num::<f32>
                    )))
                >> char!(',')
                >> opt!(complete!(char!('M')))
                >> (
                    time,
                    lat_lon,
                    fix_quality,
                    tracked_sats,
                    hdop,
                    altitude,
                    geoid_height
                )
        ),
        |data: (
            Option<NaiveTime>,
            Option<(f64, f64)>,
            char,
            Option<u32>,
            Option<f32>,
            Option<f32>,
            Option<f32>
        )|
         -> std::result::Result<GgaData, &'static str> {
            Ok(GgaData {
                fix_time: data.0,
                fix_type: Some(FixType::from(data.2)),
                latitude: data.1.map(|v| v.0),
                longitude: data.1.map(|v| v.1),
                fix_satellites: data.3,
                hdop: data.4,
                altitude: data.5,
                geoid_height: data.6,
            })
        }
    )
);

/// Parse GGA message
/// from gpsd/driver_nmea0183.c
/// GGA,123519,4807.038,N,01131.324,E,1,08,0.9,545.4,M,46.9,M, , *42
/// 1     123519       Fix taken at 12:35:19 UTC
/// 2,3   4807.038,N   Latitude 48 deg 07.038' N
/// 4,5   01131.324,E  Longitude 11 deg 31.324' E
/// 6         1            Fix quality: 0 = invalid, 1 = GPS, 2 = DGPS,
/// 3=PPS (Precise Position Service),
/// 4=RTK (Real Time Kinematic) with fixed integers,
/// 5=Float RTK, 6=Estimated, 7=Manual, 8=Simulator
/// 7     08       Number of satellites being tracked
/// 8     0.9              Horizontal dilution of position
/// 9,10  545.4,M      Altitude, Metres above mean sea level
/// 11,12 46.9,M       Height of geoid (mean sea level) above WGS84
/// ellipsoid, in Meters
/// (empty field) time in seconds since last DGPS update
/// (empty field) DGPS station ID number (0000-1023)
pub fn parse_gga(sentence: &NmeaSentence) -> Result<GgaData, String> {
    if sentence.message_id != b"GGA" {
        return Err("GGA sentence not starts with $..GGA".into());
    }
    let res: GgaData = do_parse_gga(sentence.data)
        .map_err(|err| match err {
            nom::Err::Incomplete(_) => "Incomplete nmea sentence".to_string(),
            nom::Err::Error((_, kind)) | nom::Err::Failure((_, kind)) => {
                kind.description().to_string()
            }
        })?
        .1;
    Ok(res)
}

#[derive(Debug, PartialEq)]
pub enum RmcStatusOfFix {
    Autonomous,
    Differential,
    Invalid,
}

#[derive(Debug, PartialEq)]
pub struct RmcData {
    pub fix_time: Option<NaiveTime>,
    pub fix_date: Option<NaiveDate>,
    pub status_of_fix: Option<RmcStatusOfFix>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub speed_over_ground: Option<f32>,
    pub true_course: Option<f32>,
}

named!(
    parse_date<NaiveDate>,
    map_res!(
        do_parse!(
            day: map_res!(take!(2), parse_num::<u8>)
                >> month: map_res!(take!(2), parse_num::<u8>)
                >> year: map_res!(take!(2), parse_num::<u8>)
                >> (day, month, year)
        ),
        |data: (u8, u8, u8)| -> Result<NaiveDate, &'static str> {
            let (day, month, year) = (u32::from(data.0), u32::from(data.1), i32::from(data.2));
            if month < 1 || month > 12 {
                return Err("Invalid month < 1 or > 12");
            }
            if day < 1 || day > 31 {
                return Err("Invalid day < 1 or > 31");
            }
            Ok(NaiveDate::from_ymd(year, month, day))
        }
    )
);

named!(
    do_parse_rmc<RmcData>,
    map_res!(
        do_parse!(
            time: opt!(complete!(parse_hms))
                >> char!(',')
                >> status_of_fix: one_of!("ADV")
                >> char!(',')
                >> lat_lon: parse_lat_lon
                >> char!(',')
                >> speed_over_ground:
                    opt!(complete!(map_res!(float_number, parse_float_num::<f32>)))
                >> char!(',')
                >> true_course: opt!(complete!(map_res!(float_number, parse_float_num::<f32>)))
                >> char!(',')
                >> date: opt!(complete!(parse_date))
                >> char!(',')
                >> (
                    time,
                    status_of_fix,
                    lat_lon,
                    speed_over_ground,
                    true_course,
                    date
                )
        ),
        |data: (
            Option<NaiveTime>,
            char,
            Option<(f64, f64)>,
            Option<f32>,
            Option<f32>,
            Option<NaiveDate>
        )|
         -> Result<RmcData, &'static str> {
            Ok(RmcData {
                fix_time: data.0,
                fix_date: data.5,
                status_of_fix: Some(match data.1 {
                    'A' => RmcStatusOfFix::Autonomous,
                    'D' => RmcStatusOfFix::Differential,
                    'V' => RmcStatusOfFix::Invalid,
                    _ => return Err("do_parse_rmc failed: not A|D|V status of fix"),
                }),
                lat: data.2.map(|v| v.0),
                lon: data.2.map(|v| v.1),
                speed_over_ground: data.3,
                true_course: data.4,
            })
        }
    )
);

/// Parse RMC message
/// From gpsd:
/// RMC,225446.33,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E,A*68
/// 1     225446.33    Time of fix 22:54:46 UTC
/// 2     A          Status of Fix: A = Autonomous, valid;
/// D = Differential, valid; V = invalid
/// 3,4   4916.45,N    Latitude 49 deg. 16.45 min North
/// 5,6   12311.12,W   Longitude 123 deg. 11.12 min West
/// 7     000.5      Speed over ground, Knots
/// 8     054.7      Course Made Good, True north
/// 9     181194       Date of fix  18 November 1994
/// 10,11 020.3,E      Magnetic variation 20.3 deg East
/// 12    A      FAA mode indicator (NMEA 2.3 and later)
/// A=autonomous, D=differential, E=Estimated,
/// N=not valid, S=Simulator, M=Manual input mode
/// *68        mandatory nmea_checksum
///
/// SiRF chipsets don't return either Mode Indicator or magnetic variation.
pub fn parse_rmc(sentence: &NmeaSentence) -> Result<RmcData, String> {
    if sentence.message_id != b"RMC" {
        return Err("RMC message should starts with $..RMC".into());
    }
    do_parse_rmc(sentence.data)
        .map(|(_, data)| data)
        .map_err(|err| match err {
            nom::Err::Incomplete(_) => "Incomplete nmea sentence".to_string(),
            nom::Err::Error((_, kind)) | nom::Err::Failure((_, kind)) => {
                kind.description().to_string()
            }
        })
}

#[derive(PartialEq, Debug)]
pub enum GsaMode1 {
    Manual,
    Automatic,
}

#[derive(Debug, PartialEq)]
pub enum GsaMode2 {
    NoFix,
    Fix2D,
    Fix3D,
}

#[derive(Debug, PartialEq)]
pub struct GsaData {
    pub mode1: GsaMode1,
    pub mode2: GsaMode2,
    pub fix_sats_prn: Vec<u32>,
    pub pdop: Option<f32>,
    pub hdop: Option<f32>,
    pub vdop: Option<f32>,
}

named!(gsa_prn_fields_parse<&[u8], Vec<Option<u32>>>, many0!(complete!(map_res!(do_parse!(
    prn: opt!(map_res!(complete!(digit1), parse_num::<u32>)) >>
    char!(',') >> (prn)),
    |prn: Option<u32>| -> Result<Option<u32>, String> {
        Ok(prn)
    }
))));

type GsaTail = (Vec<Option<u32>>, Option<f32>, Option<f32>, Option<f32>);
named!(
    do_parse_gsa_tail<GsaTail>,
    do_parse!(
        prns: gsa_prn_fields_parse
            >> pdop: map_res!(float_number, parse_float_num::<f32>)
            >> char!(',')
            >> hdop: map_res!(float_number, parse_float_num::<f32>)
            >> char!(',')
            >> vdop: map_res!(float_number, parse_float_num::<f32>)
            >> (prns, Some(pdop), Some(hdop), Some(vdop))
    )
);

fn is_comma(x: u8) -> bool {
    x == b','
}

fn do_parse_empty_gsa_tail(input: &[u8]) -> IResult<&[u8], GsaTail> {
    map_res(
        all_consuming(take_while1(is_comma)),
        |_| -> Result<GsaTail, String> { Ok((Vec::new(), None, None, None)) },
    )(input)
}

named!(
    do_parse_gsa<GsaData>,
    map_res!(
        do_parse!(
            mode1: one_of!("MA")
                >> char!(',')
                >> mode2: one_of!("123")
                >> char!(',')
                >> tail: alt!(complete!(do_parse_empty_gsa_tail) | complete!(do_parse_gsa_tail))
                >> (mode1, mode2, tail)
        ),
        |mut data: (char, char, GsaTail)| -> Result<GsaData, String> {
            Ok(GsaData {
                mode1: match data.0 {
                    'M' => GsaMode1::Manual,
                    'A' => GsaMode1::Automatic,
                    _ => unreachable!(),
                },
                mode2: match data.1 {
                    '1' => GsaMode2::NoFix,
                    '2' => GsaMode2::Fix2D,
                    '3' => GsaMode2::Fix3D,
                    _ => unreachable!(),
                },
                fix_sats_prn: (data.2).0.drain(..).filter_map(|v| v).collect(),
                pdop: (data.2).1,
                hdop: (data.2).2,
                vdop: (data.2).3,
            })
        }
    )
);

/// Parse GSA
/// from gpsd:
/// eg1. $GPGSA,A,3,,,,,,16,18,,22,24,,,3.6,2.1,2.2*3C
/// eg2. $GPGSA,A,3,19,28,14,18,27,22,31,39,,,,,1.7,1.0,1.3*35
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
/// may be variable.  Most doc that specifies says 12 PRNs.
///
/// the CH-4701 ourputs 24 PRNs!
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
/// $GNGSA,A,3,31,26,21,,,,,,,,,,3.77,2.55,2.77*1A
/// $GNGSA,A,3,75,86,87,,,,,,,,,,3.77,2.55,2.77*1C
/// seems like the first is GNSS and the second GLONASS
///
/// One chipset called the i.Trek M3 issues GPGSA lines that look like
/// this: "$GPGSA,A,1,,,,*32" when it has no fix.  This is broken
/// in at least two ways: it's got the wrong number of fields, and
/// it claims to be a valid sentence (A flag) when it isn't.
/// Alarmingly, it's possible this error may be generic to SiRFstarIII
fn parse_gsa(s: &NmeaSentence) -> Result<GsaData, String> {
    if s.message_id != b"GSA" {
        return Err("GSA message should starts with $..GSA".into());
    }
    let ret: GsaData = do_parse_gsa(s.data)
        .map(|(_, data)| data)
        .map_err(|err| match err {
            nom::Err::Incomplete(_) => "Incomplete nmea sentence".to_string(),
            nom::Err::Error((_, kind)) | nom::Err::Failure((_, kind)) => {
                kind.description().to_string()
            }
        })?;
    Ok(ret)
}

#[derive(Debug, PartialEq)]
pub struct VtgData {
    pub true_course: Option<f32>,
    pub speed_over_ground: Option<f32>,
}

fn float_number(input: &[u8]) -> IResult<&[u8], &[u8]> {
    use nom::{InputIter, InputLength};

    let input_length = input.input_len();
    if input_length == 0 {
        return Err(nom::Err::Incomplete(nom::Needed::Unknown));
    }

    #[derive(PartialEq)]
    enum State {
        BeforePoint,
        Point,
        AfterPoint,
    }
    let mut state = State::BeforePoint;

    for (idx, item) in input.iter_indices() {
        match state {
            State::BeforePoint => {
                if item.as_char() == '.' {
                    state = State::Point;
                } else if !item.is_dec_digit() {
                    if idx == 0 {
                        return Err(nom::Err::Error(error_position!(input, ErrorKind::Digit)));
                    } else {
                        return Ok((&input[idx..], &input[..idx]));
                    }
                }
            }
            State::Point => {
                if !item.is_dec_digit() {
                    return Err(nom::Err::Error(error_position!(input, ErrorKind::Digit)));
                }
                state = State::AfterPoint;
            }
            State::AfterPoint => {
                if !item.is_dec_digit() {
                    return Ok((&input[idx..], &input[..idx]));
                }
            }
        }
    }
    Ok((&input[input_length..], input))
}

named!(
    do_parse_vtg<VtgData>,
    map_res!(
        do_parse!(
            true_course: opt!(map_res!(complete!(float_number), parse_float_num::<f32>))
                >> char!(',')
                >> opt!(complete!(char!('T')))
                >> char!(',')
                >> magn_course: opt!(map_res!(complete!(float_number), parse_float_num::<f32>))
                >> char!(',')
                >> opt!(complete!(char!('M')))
                >> char!(',')
                >> knots_ground_speed:
                    opt!(map_res!(complete!(float_number), parse_float_num::<f32>))
                >> char!(',')
                >> opt!(complete!(char!('N')))
                >> kph_ground_speed:
                    opt!(complete!(map_res!(float_number, parse_float_num::<f32>)))
                >> char!(',')
                >> opt!(complete!(char!('K')))
                >> (true_course, knots_ground_speed, kph_ground_speed)
        ),
        |data: (Option<f32>, Option<f32>, Option<f32>)| -> Result<VtgData, String> {
            //println!("data: {:?}", data);
            Ok(VtgData {
                true_course: data.0,
                speed_over_ground: match (data.1, data.2) {
                    (Some(val), _) => Some(val),
                    (_, Some(val)) => Some(val / 1.852),
                    (None, None) => None,
                },
            })
        }
    )
);

/// parse VTG
/// from http://aprs.gids.nl/nmea/#vtg
/// Track Made Good and Ground Speed.
///
/// eg1. $GPVTG,360.0,T,348.7,M,000.0,N,000.0,K*43
/// eg2. $GPVTG,054.7,T,034.4,M,005.5,N,010.2,K
///
///
/// 054.7,T      True track made good
/// 034.4,M      Magnetic track made good
/// 005.5,N      Ground speed, knots
/// 010.2,K      Ground speed, Kilometers per hour
///
///
/// eg3. $GPVTG,t,T,,,s.ss,N,s.ss,K*hh
/// 1    = Track made good
/// 2    = Fixed text 'T' indicates that track made good is relative to true north
/// 3    = not used
/// 4    = not used
/// 5    = Speed over ground in knots
/// 6    = Fixed text 'N' indicates that speed over ground in in knots
/// 7    = Speed over ground in kilometers/hour
/// 8    = Fixed text 'K' indicates that speed over ground is in kilometers/hour
/// 9    = Checksum
/// The actual track made good and speed relative to the ground.
///
/// $--VTG,x.x,T,x.x,M,x.x,N,x.x,K
/// x.x,T = Track, degrees True
/// x.x,M = Track, degrees Magnetic
/// x.x,N = Speed, knots
/// x.x,K = Speed, Km/hr
fn parse_vtg(s: &NmeaSentence) -> Result<VtgData, String> {
    if s.message_id != b"VTG" {
        return Err("VTG message should starts with $..VTG".into());
    }
    let ret: VtgData = do_parse_vtg(s.data)
        .map(|(_, data)| data)
        .map_err(|err| match err {
            nom::Err::Incomplete(_) => "Incomplete nmea sentence".to_string(),
            nom::Err::Error((_, kind)) | nom::Err::Failure((_, kind)) => {
                kind.description().to_string()
            }
        })?;
    Ok(ret)
}

pub enum ParseResult {
    GGA(GgaData),
    RMC(RmcData),
    GSV(GsvData),
    GSA(GsaData),
    VTG(VtgData),
    Unsupported(SentenceType),
}

/// parse nmea 0183 sentence and extract data from it
pub fn parse(xs: &[u8]) -> Result<ParseResult, String> {
    let nmea_sentence = parse_nmea_sentence(xs)?;

    if nmea_sentence.checksum == nmea_sentence.calc_checksum() {
        match SentenceType::try_from(nmea_sentence.message_id)? {
            SentenceType::GGA => {
                let data = parse_gga(&nmea_sentence)?;
                Ok(ParseResult::GGA(data))
            }
            SentenceType::GSV => {
                let data = parse_gsv(&nmea_sentence)?;
                Ok(ParseResult::GSV(data))
            }
            SentenceType::RMC => {
                let data = parse_rmc(&nmea_sentence)?;
                Ok(ParseResult::RMC(data))
            }
            SentenceType::GSA => Ok(ParseResult::GSA(parse_gsa(&nmea_sentence)?)),
            SentenceType::VTG => Ok(ParseResult::VTG(parse_vtg(&nmea_sentence)?)),
            msg_id => Ok(ParseResult::Unsupported(msg_id)),
        }
    } else {
        Err("Checksum mismatch".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::relative_eq;

    #[test]
    fn test_do_parse_lat_lon() {
        let (_, lat_lon) = do_parse_lat_lon(b"4807.038,N,01131.324,E").unwrap();
        relative_eq!(lat_lon.0, 48. + 7.038 / 60.);
        relative_eq!(lat_lon.1, 11. + 31.324 / 60.);
    }

    #[test]
    fn test_parse_gga_full() {
        let data = parse_gga(&NmeaSentence {
            talker_id: b"GP",
            message_id: b"GGA",
            data: b"033745.0,5650.82344,N,03548.9778,E,1,07,1.8,101.2,M,14.7,M,,",
            checksum: 0x57,
        })
        .unwrap();
        assert_eq!(data.fix_time.unwrap(), NaiveTime::from_hms(3, 37, 45));
        assert_eq!(data.fix_type.unwrap(), FixType::Gps);
        relative_eq!(data.latitude.unwrap(), 56. + 50.82344 / 60.);
        relative_eq!(data.longitude.unwrap(), 35. + 48.9778 / 60.);
        assert_eq!(data.fix_satellites.unwrap(), 7);
        relative_eq!(data.hdop.unwrap(), 1.8);
        relative_eq!(data.altitude.unwrap(), 101.2);
        relative_eq!(data.geoid_height.unwrap(), 14.7);

        let s = parse_nmea_sentence(b"$GPGGA,,,,,,0,,,,,,,,*66").unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        let data = parse_gga(&s).unwrap();
        assert_eq!(
            GgaData {
                fix_time: None,
                fix_type: Some(FixType::Invalid),
                latitude: None,
                longitude: None,
                fix_satellites: None,
                hdop: None,
                altitude: None,
                geoid_height: None,
            },
            data
        );
    }

    #[test]
    fn test_parse_gga_with_optional_fields() {
        let sentence =
            parse_nmea_sentence(b"$GPGGA,133605.0,5521.75946,N,03731.93769,E,0,00,,,M,,M,,*4F")
                .unwrap();
        assert_eq!(sentence.checksum, sentence.calc_checksum());
        assert_eq!(sentence.checksum, 0x4f);
        let data = parse_gga(&sentence).unwrap();
        assert_eq!(data.fix_type.unwrap(), FixType::Invalid);
    }

    #[test]
    fn test_parse_rmc() {
        let s = parse_nmea_sentence(
            b"$GPRMC,225446.33,A,4916.45,N,12311.12,W,\
                                  000.5,054.7,191194,020.3,E,A*2B",
        )
        .unwrap();
        assert_eq!(s.checksum, s.calc_checksum());
        assert_eq!(s.checksum, 0x2b);
        let rmc_data = parse_rmc(&s).unwrap();
        assert_eq!(
            rmc_data.fix_time.unwrap(),
            NaiveTime::from_hms_milli(22, 54, 46, 330)
        );
        assert_eq!(rmc_data.fix_date.unwrap(), NaiveDate::from_ymd(94, 11, 19));

        println!("lat: {}", rmc_data.lat.unwrap());
        relative_eq!(rmc_data.lat.unwrap(), 49.0 + 16.45 / 60.);
        println!(
            "lon: {}, diff {}",
            rmc_data.lon.unwrap(),
            (rmc_data.lon.unwrap() + (123.0 + 11.12 / 60.)).abs()
        );
        relative_eq!(rmc_data.lon.unwrap(), -(123.0 + 11.12 / 60.));

        relative_eq!(rmc_data.speed_over_ground.unwrap(), 0.5);
        relative_eq!(rmc_data.true_course.unwrap(), 54.7);

        let s = parse_nmea_sentence(b"$GPRMC,,V,,,,,,,,,,N*53").unwrap();
        let rmc = parse_rmc(&s).unwrap();
        assert_eq!(
            RmcData {
                fix_time: None,
                fix_date: None,
                status_of_fix: Some(RmcStatusOfFix::Invalid),
                lat: None,
                lon: None,
                speed_over_ground: None,
                true_course: None,
            },
            rmc
        );
    }

    #[test]
    fn test_parse_gsv_full() {
        let data = parse_gsv(&NmeaSentence {
            talker_id: b"GP",
            message_id: b"GSV",
            data: b"2,1,08,01,,083,46,02,17,308,,12,07,344,39,14,22,228,",
            checksum: 0,
        })
        .unwrap();
        assert_eq!(data.gnss_type, GnssType::Gps);
        assert_eq!(data.number_of_sentences, 2);
        assert_eq!(data.sentence_num, 1);
        assert_eq!(data._sats_in_view, 8);
        assert_eq!(
            data.sats_info[0].clone().unwrap(),
            Satellite {
                gnss_type: data.gnss_type.clone(),
                prn: 1,
                elevation: None,
                azimuth: Some(83.),
                snr: Some(46.),
            }
        );
        assert_eq!(
            data.sats_info[1].clone().unwrap(),
            Satellite {
                gnss_type: data.gnss_type.clone(),
                prn: 2,
                elevation: Some(17.),
                azimuth: Some(308.),
                snr: None,
            }
        );
        assert_eq!(
            data.sats_info[2].clone().unwrap(),
            Satellite {
                gnss_type: data.gnss_type.clone(),
                prn: 12,
                elevation: Some(7.),
                azimuth: Some(344.),
                snr: Some(39.),
            }
        );
        assert_eq!(
            data.sats_info[3].clone().unwrap(),
            Satellite {
                gnss_type: data.gnss_type.clone(),
                prn: 14,
                elevation: Some(22.),
                azimuth: Some(228.),
                snr: None,
            }
        );

        let data = parse_gsv(&NmeaSentence {
            talker_id: b"GL",
            message_id: b"GSV",
            data: b"3,3,10,72,40,075,43,87,00,000,",
            checksum: 0,
        })
        .unwrap();
        assert_eq!(data.gnss_type, GnssType::Glonass);
        assert_eq!(data.number_of_sentences, 3);
        assert_eq!(data.sentence_num, 3);
        assert_eq!(data._sats_in_view, 10);
    }

    #[test]
    fn test_parse_hms() {
        use chrono::Timelike;
        let (_, time) = parse_hms(b"125619,").unwrap();
        assert_eq!(time.hour(), 12);
        assert_eq!(time.minute(), 56);
        assert_eq!(time.second(), 19);
        assert_eq!(time.nanosecond(), 0);
        let (_, time) = parse_hms(b"125619.5,").unwrap();
        assert_eq!(time.hour(), 12);
        assert_eq!(time.minute(), 56);
        assert_eq!(time.second(), 19);
        assert_eq!(time.nanosecond(), 5_00_000_000);
    }

    #[test]
    fn test_gsa_prn_fields_parse() {
        let (_, ret) = gsa_prn_fields_parse(b"5,").unwrap();
        assert_eq!(vec![Some(5)], ret);
        let (_, ret) = gsa_prn_fields_parse(b",").unwrap();
        assert_eq!(vec![None], ret);

        let (_, ret) = gsa_prn_fields_parse(b",,5,6,").unwrap();
        assert_eq!(vec![None, None, Some(5), Some(6)], ret);
    }

    #[test]
    fn smoke_test_parse_gsa() {
        let s = parse_nmea_sentence(b"$GPGSA,A,3,,,,,,16,18,,22,24,,,3.6,2.1,2.2*3C").unwrap();
        let gsa = parse_gsa(&s).unwrap();
        assert_eq!(
            GsaData {
                mode1: GsaMode1::Automatic,
                mode2: GsaMode2::Fix3D,
                fix_sats_prn: vec![16, 18, 22, 24],
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
            parse_gsa(&s).unwrap();
        }
    }

    #[test]
    fn test_float_number() {
        assert_eq!(Ok((&b""[..], &b"12.3"[..])), float_number(&b"12.3"[..]));
        assert_eq!(Ok((&b"a"[..], &b"12.3"[..])), float_number(&b"12.3a"[..]));
        assert_eq!(Ok((&b"a"[..], &b"12"[..])), float_number(&b"12a"[..]));
        assert_eq!(
            Err(nom::Err::Error((&b"a12a"[..], ErrorKind::Digit))),
            float_number(&b"a12a"[..])
        );
    }

    #[test]
    fn test_parse_vtg() {
        let run_parse_vtg = |line: &str| -> Result<VtgData, String> {
            let s =
                parse_nmea_sentence(line.as_bytes()).expect("VTG sentence initial parse failed");
            assert_eq!(s.checksum, s.calc_checksum());
            parse_vtg(&s)
        };
        assert_eq!(
            VtgData {
                true_course: None,
                speed_over_ground: None,
            },
            run_parse_vtg("$GPVTG,,T,,M,,N,,K,N*2C").unwrap()
        );
        assert_eq!(
            VtgData {
                true_course: Some(360.),
                speed_over_ground: Some(0.),
            },
            run_parse_vtg("$GPVTG,360.0,T,348.7,M,000.0,N,000.0,K*43").unwrap()
        );
        assert_eq!(
            VtgData {
                true_course: Some(54.7),
                speed_over_ground: Some(5.5),
            },
            run_parse_vtg("$GPVTG,054.7,T,034.4,M,005.5,N,010.2,K*48").unwrap()
        );
    }
}
