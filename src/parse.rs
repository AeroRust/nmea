use std;
use std::str;

use chrono::NaiveTime;
use nom::{digit, IError};

use GnssType;
use Satellite;
use FixType;

pub struct NmeaSentence<'a> {
    pub talker_id: &'a [u8],
    pub message_id: &'a [u8],
    pub data: &'a [u8],
    pub checksum: u8,
}

impl<'a> NmeaSentence<'a> {
    pub fn calc_checksum(&self) -> u8 {
        checksum(self.talker_id.iter().chain(self.message_id.iter())
                 .chain(&[b',']).chain(self.data.iter()))
    }
}

pub struct GsvData {
    pub gnss_type: GnssType,
    pub number_of_sentences: u16,
    pub sentence_num: u16,
    pub _sats_in_view: u16,
    pub sats_info: [Option<Satellite>; 4],
}

pub fn checksum<'a, I: Iterator<Item=&'a u8>>(bytes: I) -> u8 {
    bytes.fold(0, |c, x| c ^ *x)
}

fn construct_sentence<'a>(data: (&'a [u8], &'a [u8], &'a [u8], u8)) -> std::result::Result<NmeaSentence<'a>, &'static str> {
    Ok(NmeaSentence{ talker_id: data.0, message_id: data.1, data: data.2, checksum: data.3 })
}

fn parse_hex(data: &[u8]) -> std::result::Result<u8, &'static str> {
    u8::from_str_radix(unsafe { str::from_utf8_unchecked(data) }, 16)
        .map_err(|_| "Failed to parse checksum as hex number")
}

named!(parse_checksum<u8>, map_res!(
    do_parse!(
        char!('*') >>
        checksum_bytes: take!(2) >>
            (checksum_bytes)),
    parse_hex));

named!(do_parse_nmea_sentence<NmeaSentence>,
       map_res!(
           do_parse!(
               char!('$') >>
               talker_id: take!(2) >>
               message_id: take!(3) >>
               char!(',') >>
               data: take_until!("*") >>
               cs: parse_checksum >> (talker_id, message_id, data, cs)),
            construct_sentence
       )
);

pub fn parse_nmea_sentence(sentence: &[u8]) -> std::result::Result<NmeaSentence, String> {
    let res: NmeaSentence = do_parse_nmea_sentence(sentence)
        .to_full_result()
        .map_err(|err| match err {
            IError::Incomplete(_) => "Incomplete nmea sentence".to_string(),
            IError::Error(e) => e.description().to_string(),
        }
        )?;
    Ok(res)
}

fn parse_num<I: std::str::FromStr>(data: &[u8]) -> std::result::Result<I, &'static str> {
//    println!("parse num {}", unsafe { str::from_utf8_unchecked(data) });
    str::parse::<I>(unsafe { str::from_utf8_unchecked(data) })
        .map_err(|_| "parse of number failed")
}

fn construct_satellite(data: (u32, Option<i32>, Option<i32>, Option<i32>))
                       -> std::result::Result<Satellite, &'static str> {
//    println!("we construct sat {}", data.0);
    Ok(Satellite {
        gnss_type: GnssType::Galileo,
        prn: data.0,
        elevation: data.1.map(|v| v as f32),
        azimuth: data.2.map(|v| v as f32),
        snr: data.3.map(|v| v as f32),
    })
}

named!(parse_gsv_sat_info<Satellite>,
       map_res!(
           do_parse!(
               prn: map_res!(digit, parse_num::<u32>) >>
               char!(',') >>
               elevation:  opt!(map_res!(digit, parse_num::<i32>)) >>
               char!(',') >>
               azimuth: opt!(map_res!(digit, parse_num::<i32>)) >>
               char!(',') >>
               signal_noise: opt!(map_res!(complete!(digit), parse_num::<i32>)) >>
               dbg!(alt!(eof!() | tag!(","))) >>
               (prn, elevation, azimuth, signal_noise)),
           construct_satellite
       ));


fn construct_gsv_data(data: (u16, u16, u16, Option<Satellite>, Option<Satellite>,
                             Option<Satellite>, Option<Satellite>))
                      -> std::result::Result<GsvData, &'static str> {
    Ok(GsvData {
        gnss_type: GnssType::Galileo,
        number_of_sentences: data.0,
        sentence_num: data.1,
        _sats_in_view: data.2,
        sats_info: [data.3, data.4, data.5, data.6],
    })
}

named!(do_parse_gsv<GsvData>,
       map_res!(
           do_parse!(
               number_of_sentences: map_res!(digit, parse_num::<u16>) >>
               char!(',') >>
               sentence_index: map_res!(digit, parse_num::<u16>) >>
               char!(',') >>
               total_number_of_sats: map_res!(digit, parse_num::<u16>) >>
               char!(',') >>
               sat0: opt!(complete!(parse_gsv_sat_info)) >>
               sat1: opt!(complete!(parse_gsv_sat_info)) >>
               sat2: opt!(complete!(parse_gsv_sat_info)) >>
               sat3: opt!(complete!(parse_gsv_sat_info)) >>
               (number_of_sentences, sentence_index, total_number_of_sats, sat0, sat1, sat2, sat3)),
           construct_gsv_data));

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
        .to_full_result()
        .map_err(|err| match err {
            IError::Incomplete(_) => "Incomplete nmea sentence".to_string(),
            IError::Error(e) => e.description().into(),
        })?;
    res.gnss_type = gnss_type.clone();
    for sat in res.sats_info.iter_mut() {
        (*sat).as_mut().map(|v| v.gnss_type = gnss_type.clone());
    }
    Ok(res)
}

#[test]
fn test_parse_gsv_full() {
    let data = parse_gsv(&NmeaSentence {
        talker_id: b"GP",
        message_id: b"GSV",
        data: b"2,1,08,01,,083,46,02,17,308,,12,07,344,39,14,22,228,",
        checksum: 0,
    }).unwrap();
    assert_eq!(data.gnss_type, GnssType::Gps);
    assert_eq!(data.number_of_sentences, 2);
    assert_eq!(data.sentence_num, 1);
    assert_eq!(data._sats_in_view, 8);
    assert_eq!(data.sats_info[0].clone().unwrap(), Satellite{gnss_type: data.gnss_type.clone(), prn: 1, elevation: None, azimuth: Some(83.), snr: Some(46.)});
    assert_eq!(data.sats_info[1].clone().unwrap(), Satellite{gnss_type: data.gnss_type.clone(), prn: 2, elevation: Some(17.), azimuth: Some(308.), snr: None});
    assert_eq!(data.sats_info[2].clone().unwrap(), Satellite{gnss_type: data.gnss_type.clone(), prn: 12, elevation: Some(7.), azimuth: Some(344.), snr: Some(39.)});
    assert_eq!(data.sats_info[3].clone().unwrap(), Satellite{gnss_type: data.gnss_type.clone(), prn: 14, elevation: Some(22.), azimuth: Some(228.), snr: None});

    let data = parse_gsv(&NmeaSentence {
        talker_id: b"GL",
        message_id: b"GSV",
        data: b"3,3,10,72,40,075,43,87,00,000,",
        checksum: 0,
    }).unwrap();
    assert_eq!(data.gnss_type, GnssType::Glonass);
    assert_eq!(data.number_of_sentences, 3);
    assert_eq!(data.sentence_num, 3);
    assert_eq!(data._sats_in_view, 10);
}

pub struct GgaData {
    pub fix_timestamp_time: Option<NaiveTime>,
    pub fix_type: Option<FixType>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub fix_satellites: Option<u32>,
    pub hdop: Option<f32>,
    pub altitude: Option<f32>,
    pub geoid_height: Option<f32>,
}

fn construct_time(data: (u32, u32, f64)) -> std::result::Result<NaiveTime, &'static str> {
    if data.2.is_sign_negative() {
        return Err("Invalid time: second is negative");
    }
    if data.0 >= 24 {
        return Err("Invalid time: hour >= 24");
    }
    if data.1 >= 60 {
        return Err("Invalid time: min >= 60");
    }
    Ok(NaiveTime::from_hms_nano(data.0, data.1, data.2.trunc() as u32,
                                (data.2.fract() * 1_000_000_000f64).round() as u32))
}

fn parse_float_num<T: str::FromStr>(input: &[u8]) -> std::result::Result<T, &'static str> {
    let s = str::from_utf8(input)
        .map_err(|_| "invalid float number")?;
    str::parse::<T>(s)
        .map_err(|_| "parse of float number failed")
}

named!(parse_hms<NaiveTime>,
       map_res!(
           do_parse!(
               hour: map_res!(take!(2), parse_num::<u32>) >>
               min: map_res!(take!(2), parse_num::<u32>) >>
               sec: map_res!(take_until!(","), parse_float_num::<f64>) >>
               (hour, min, sec)
           ),
           construct_time
       )
);

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

named!(parse_lat_lon<(f64, f64)>,
       map_res!(
           do_parse!(
               lat_deg: map_res!(take!(2), parse_num::<u8>) >>
               lat_min: map_res!(take_until!(","), parse_float_num::<f64>) >>
               char!(',') >>
               lat_dir: one_of!("NS") >>
               char!(',') >>
               lon_deg: map_res!(take!(3), parse_num::<u8>) >>
               lon_min: map_res!(take_until!(","), parse_float_num::<f64>) >>
               char!(',') >>
               lon_dir: one_of!("EW") >>
               (lat_deg, lat_min, lat_dir, lon_deg, lon_min, lon_dir)
           ),
           |data: (u8, f64, char, u8, f64, char)| -> std::result::Result<(f64, f64), &'static str> {
               let mut lat = (data.0 as f64) + data.1 / 60.;
               if data.2 == 'S' {
                   lat = -lat;
               }
               let mut lon = (data.3 as f64) + data.4 / 60.;
               if data.5 == 'W' {
                   lon = -lon;
               }
               Ok((lat, lon))
           }
));

#[test]
fn test_parse_lat_lon() {
    let (_, lat_lon) = parse_lat_lon(b"4807.038,N,01131.324,E").unwrap();
    relative_eq!(lat_lon.0, 48. + 7.038 / 60.);
    relative_eq!(lat_lon.1, 11. + 31.324 / 60.);
}

named!(do_parse_gga<GgaData>,
       map_res!(
           do_parse!(
               time: parse_hms >>
               char!(',') >>
               lat_lon: parse_lat_lon >>
               char!(',') >>
               fix_quality: one_of!("012345678") >>
               char!(',') >>
               tracked_sats: map_res!(dbg!(digit), parse_num::<u32>) >>
               char!(',') >>
               hdop: opt!(map_res!(take_until!(","), parse_float_num::<f32>)) >>
               char!(',') >>
               altitude: opt!(map_res!(take_until!(","), parse_float_num::<f32>)) >>
               char!(',') >>
               char!('M') >>
               char!(',') >>
               geoid_height: opt!(map_res!(take_until!(","), parse_float_num::<f32>)) >>
               char!(',') >>
               char!('M') >>
               (time, lat_lon, fix_quality, tracked_sats, hdop, altitude, geoid_height)),
           |data: (NaiveTime, (f64, f64), char, u32, Option<f32>, Option<f32>, Option<f32>)| -> std::result::Result<GgaData, &'static str> {
               Ok(GgaData {
                   fix_timestamp_time: Some(data.0),
                   fix_type: Some(FixType::from(data.2)),
                   latitude: Some((data.1).0),
                   longitude: Some((data.1).1),
                   fix_satellites: Some(data.3),
                   hdop: data.4,
                   altitude: data.5,
                   geoid_height: data.6,
               })
           }
));

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
        .to_full_result()
        .map_err(|err| match err {
            IError::Incomplete(_) => "Incomplete nmea sentence".to_string(),
            IError::Error(e) => e.description().into(),
        })?;
     Ok(res)
}

#[test]
fn test_parse_gga_full() {
    let data = parse_gga(&NmeaSentence {
        talker_id: b"GP",
        message_id: b"GGA",
        data: b"033745.0,5650.82344,N,03548.9778,E,1,07,1.8,101.2,M,14.7,M,,",
        checksum: 0x57,
    }).unwrap();
    assert_eq!(data.fix_timestamp_time.unwrap(), NaiveTime::from_hms(3, 37, 45));
    assert_eq!(data.fix_type.unwrap(), FixType::Gps);
    relative_eq!(data.latitude.unwrap(), 56. + 50.82344 / 60.);
    relative_eq!(data.longitude.unwrap(), 35. + 48.9778 / 60.);
    assert_eq!(data.fix_satellites.unwrap(), 7);
    relative_eq!(data.hdop.unwrap(), 1.8);
    relative_eq!(data.altitude.unwrap(), 101.2);
    relative_eq!(data.geoid_height.unwrap(), 14.7);
}

#[test]
fn test_parse_gga_with_optional_fields() {
    let sentence = parse_nmea_sentence(b"$GPGGA,133605.0,5521.75946,N,03731.93769,E,0,00,,,M,,M,,*4F").unwrap();
    assert_eq!(sentence.checksum, sentence.calc_checksum());
    assert_eq!(sentence.checksum, 0x4f);
    let data = parse_gga(&sentence).unwrap();
    assert_eq!(data.fix_type.unwrap(), FixType::Invalid);
}
