// Copyright (C) 2016 Felix Obenhuber
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate time;
#[cfg(test)]
extern crate quickcheck;

use regex::Regex;
use std::collections::HashMap;
use std::fmt;
use std::vec::Vec;
use ::time::Tm;
use std::ops::Range;
use std::iter::Iterator;

error_chain! {
    foreign_links {
        Regex(::regex::Error);
    }
}

/// NMEA parser
#[derive(Default)]
pub struct Nmea {
    pub fix_timestamp: Option<Tm>,
    pub fix_type: Option<FixType>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f32>,
    pub fix_satellites: Option<u32>,
    pub hdop: Option<f32>,
    pub geoid_height: Option<f32>,
    pub satellites: Vec<Satellite>,
    satellites_scan: HashMap<GnssType, Vec<Vec<Satellite>>>,
}

impl<'a> Nmea {
    /// Constructs a new `Nmea`.
    /// This struct parses NMEA sentences, including checksum checks and sentence
    /// validation.
    ///
    /// # Examples
    ///
    /// ```
    /// use nmea::Nmea;
    ///
    /// let mut nmea= Nmea::new();
    /// let gga = "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76";
    /// nmea.parse(gga).ok();
    /// println!("{}", nmea);
    /// ```
    pub fn new() -> Nmea {
        // TODO: This looks ugly.
        let mut n = Nmea::default();
        n.satellites_scan.insert(GnssType::Galileo, vec![]);
        n.satellites_scan.insert(GnssType::Gps, vec![]);
        n.satellites_scan.insert(GnssType::Glonass, vec![]);
        n
    }

    /// Returns fix type
    pub fn fix_timestamp(&self) -> Option<Tm> {
        self.fix_timestamp
    }

    /// Returns fix type
    pub fn fix_type(&self) -> Option<FixType> {
        self.fix_type.clone()
    }

    /// Returns last fixed latitude in degress. None if not fixed.
    pub fn latitude(&self) -> Option<f64> {
        self.latitude
    }

    /// Returns last fixed longitude in degress. None if not fixed.
    pub fn longitude(&self) -> Option<f64> {
        self.longitude
    }

    /// Returns latitude from last fix. None if not available.
    pub fn altitude(&self) -> Option<f32> {
        self.altitude
    }

    /// Returns the number of satellites use for fix.
    pub fn fix_satellites(&self) -> Option<u32> {
        self.fix_satellites
    }

    /// Returns the number fix HDOP
    pub fn hdop(&self) -> Option<f32> {
        self.hdop
    }

    /// Returns the height of geoid above WGS84
    pub fn geoid_height(&self) -> Option<f32> {
        self.geoid_height
    }

    /// Returns the height of geoid above WGS84
    pub fn satellites(&self) -> Vec<Satellite> {
        self.satellites.clone()
    }

    /// Returns the NMEA sentence type.
    pub fn sentence_type(&self, s: &'a str) -> Result<SentenceType> {
        let error_msg = "Failed to parse type";
        REGEX_TYPE.captures(s)
            .ok_or(error_msg.into())
            .and_then(|caps| {
                caps.name("type")
                    .ok_or(error_msg.into())
            })
            .and_then(|t| Ok(SentenceType::from(t.as_str())))
    }

    /// Parse a HHMMSS string into todays UTC datetime
    fn parse_hms(s: &'a str) -> Result<Tm> {
        REGEX_HMS.captures(s)
            .ok_or("Failed to parse time".into())
            .and_then(|c| {
                // TODO: Think about defining a struct for the fix time. NMEA
                // is quite poor regarding the time info...
                let mut t = ::time::now();
                t.tm_sec = Self::parse_numeric::<u32>(c.get(3).unwrap().as_str(), 1)? as i32;
                t.tm_min = Self::parse_numeric::<u32>(c.get(2).unwrap().as_str(), 1)? as i32;
                t.tm_hour = Self::parse_numeric::<u32>(c.get(1).unwrap().as_str(), 1)? as i32;
                Ok(t)
            })
    }

    fn parse_gga(&mut self, sentence: &'a str) -> Result<SentenceType> {
        match REGEX_GGA.captures(sentence) {
            Some(caps) => {
                self.fix_timestamp = caps.name("timestamp")
                    .and_then(|t| Self::parse_hms(t.as_str()).ok());
                self.fix_type = caps.name("fix_type").and_then(|t| Some(FixType::from(t.as_str())));
                self.latitude =
                    caps.name("lat_dir").and_then(|s| {
                        match s.as_str() {
                                "N" => {
                                    caps.name("lat")
                                        .and_then(|l| {
                                            Self::parse_numeric::<f64>(l.as_str(), 0.01).ok()
                                        })
                                }
                                "S" => {
                                    caps.name("lat")
                                        .and_then(|l| {
                                            Self::parse_numeric::<f64>(l.as_str(), -0.01).ok()
                                        })
                                }
                                _ => None,
                            }
                            .map(|v| v.trunc() + v.fract() * 100. / 60.)
                    });
                self.longitude = caps.name("lon_dir").and_then(|s| {
                    match s.as_str() {
                            "W" => {
                            caps.name("lon")
                                .and_then(|l| Self::parse_numeric::<f64>(l.as_str(), -0.01).ok())
                        }
                            "E" => {
                                caps.name("lon")
                                    .and_then(|l| Self::parse_numeric::<f64>(l.as_str(), 0.01).ok())
                            }
                            _ => None,
                        }
                        .map(|v| v.trunc() + v.fract() * 100. / 60.)
                });
                self.altitude = caps.name("alt")
                    .and_then(|a| Self::parse_numeric::<f32>(a.as_str(), 1.0).ok());
                self.fix_satellites = caps.name("fix_satellites")
                    .and_then(|a| Self::parse_numeric::<u32>(a.as_str(), 1).ok());
                self.hdop = caps.name("hdop")
                    .and_then(|a| Self::parse_numeric::<f32>(a.as_str(), 1.0).ok());
                self.geoid_height = caps.name("geoid_height")
                    .and_then(|g| Self::parse_numeric::<f32>(g.as_str(), 1.0).ok());
                Ok(SentenceType::GGA)
            }
            None => Err("Failed to parse GGA sentence".into()),
        }
    }

    fn merge_gsv_data(&mut self, data: GsvData) -> Result<()> {
        {
            let d = self.satellites_scan.get_mut(&data.gnss_type)
                .ok_or("Invalid GNSS type")?;
            // Adjust size to this scan
            d.resize(data.number_of_sentences as usize, vec![]);
            // Replace data at index with new scan data
            d.push(data.sats_info.iter().filter(|v| v.is_some()).map(|v| v.clone().unwrap()).collect());
            d.swap_remove(data.sentence_num as usize - 1);
        }
        self.satellites.clear();
        for (_, v) in &self.satellites_scan {
            for v1 in v {
                for v2 in v1 {
                    self.satellites.push(v2.clone());
                }
            }
        }

        Ok(())
    }

    /// Parse any NMEA sentence and stores the result. The type of sentence
    /// is returnd if implemented and valid.
    pub fn parse(&mut self, s: &'a str) -> Result<SentenceType> {
        let caps = REGEX_CHECKSUM.captures(s).ok_or("Failed to parse sentence")?;
        let sentence = caps.name(&"sentence").ok_or("Failed to parse sentence")?;
        let sentence = sentence.as_str();
        let checksum =
            caps.name(&"checksum")
                .ok_or("Failed to parse checksum")
                .and_then(|c| {
                    u8::from_str_radix(c.as_str(), 16).map_err(|_| "Failed to parse checksun")
                })?;

        if Nmea::checksum(sentence) == checksum {
            match self.sentence_type(sentence)? {
                SentenceType::GGA => self.parse_gga(sentence),
                SentenceType::GSV => {
                    let data = parse_gsv(sentence)?;
                    self.merge_gsv_data(data)?;
                    Ok(SentenceType::GSV)
                }
                _ => Err("Unknown or implemented sentence type".into()),
            }
        } else {
            Err("Checksum mismatch".into())
        }
    }

    fn checksum(sentence: &str) -> u8 {
        sentence.bytes().fold(0, |c, x| c ^ x)
    }

    fn parse_numeric<T>(input: &'a str, factor: T) -> Result<T>
        where T: std::str::FromStr + std::ops::Mul<Output = T> + Copy
    {
        input.parse::<T>()
            .map_err(|_| "Failed to parse numeric value".into())
            .map(|v| v * factor)
    }
}

struct GsvData {
    gnss_type: GnssType,
    number_of_sentences: u16,
    sentence_num: u16,
    _sats_in_view: u16,
    sats_info: [Option<Satellite>; 4],
}

fn str_slice<'a>(s: &'a str, range: Range<usize>) -> Option<&'a str> {
    assert!(range.start <= range.end);
    if range.start <= s.len() && range.end <= s.len() {
        Some(unsafe { s.slice_unchecked(range.start, range.end) })
    } else {
        None
    }
}

macro_rules! map_not_empty {
    ($StrName: ident, $Expr: expr) => {
        if !$StrName.is_empty() {
            Some($Expr)
        } else {
            None
        }
    }
}

/// parse one GSV sentence
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
fn parse_gsv(sentence: &str) -> Result<GsvData> {
    if str_slice(sentence, 0..5).map_or(true, |head| &head[2..5] != "GSV") {
        return Err("GSV sentence not starts with $..GSV".into());
    }

    let mut field = sentence.split(",");
    let gnss_type = match &(field.next().ok_or("no header")?[0..2]) {
        "GP" => GnssType::Gps,
        "GL" => GnssType::Glonass,
        _ => return Err("Unknown GNSS type in GSV sentence".into()),
    };

    let number_of_sentences = Nmea::parse_numeric::<u16>(field.next().ok_or("no sentence amount")?, 1)?;
    let sentence_num = Nmea::parse_numeric::<u16>(field.next().ok_or("no sentence number")?, 1)?;
    let sats_in_view = Nmea::parse_numeric::<u16>(field.next().ok_or("no number of satellites in view")?, 1)?;

    let rest_field_num = field.clone().count();
    let nsats_info = rest_field_num / 4;
    if rest_field_num % 4 != 0 || nsats_info > 4 {
        return Err("mailformed sattilite info in GSV".into());
    }
    let mut sats_info = [None, None, None, None];
    for idx in 0..nsats_info {
        let prn = Nmea::parse_numeric::<u32>(field.next().ok_or("no prn")?, 1)?;
        let elevation = field.next().ok_or("no elevation")?;
        let elevation = map_not_empty!(elevation, Nmea::parse_numeric::<f32>(elevation, 1.0)?);
        let azimuth = field.next().ok_or("no aizmuth")?;
        let azimuth = map_not_empty!(azimuth, Nmea::parse_numeric::<f32>(azimuth, 1.0)?);
        let snr = field.next().ok_or("no SNR")?;
        let snr = map_not_empty!(snr, Nmea::parse_numeric::<f32>(snr, 1.0)?);
        sats_info[idx] = Some(Satellite{gnss_type: gnss_type.clone(), prn: prn, elevation: elevation, azimuth: azimuth, snr: snr});
    }
    Ok(GsvData {
        gnss_type: gnss_type,
        number_of_sentences: number_of_sentences,
        sentence_num: sentence_num,
        _sats_in_view: sats_in_view,
        sats_info: sats_info,
    })
}

#[test]
fn test_parse_gsv_full() {
    let data = parse_gsv("GPGSV,2,1,08,01,40,083,46,02,17,308,41,12,07,344,39,14,22,228,45").unwrap();
    assert_eq!(data.gnss_type, GnssType::Gps);
    assert_eq!(data.number_of_sentences, 2);
    assert_eq!(data.sentence_num, 1);
    assert_eq!(data._sats_in_view, 8);
    assert_eq!(data.sats_info[0].clone().unwrap(), Satellite{gnss_type: data.gnss_type.clone(), prn: 1, elevation: Some(40.), azimuth: Some(83.), snr: Some(46.)});
    assert_eq!(data.sats_info[1].clone().unwrap(), Satellite{gnss_type: data.gnss_type.clone(), prn: 2, elevation: Some(17.), azimuth: Some(308.), snr: Some(41.)});
    assert_eq!(data.sats_info[2].clone().unwrap(), Satellite{gnss_type: data.gnss_type.clone(), prn: 12, elevation: Some(7.), azimuth: Some(344.), snr: Some(39.)});
    assert_eq!(data.sats_info[3].clone().unwrap(), Satellite{gnss_type: data.gnss_type.clone(), prn: 14, elevation: Some(22.), azimuth: Some(228.), snr: Some(45.)});
}

impl fmt::Debug for Nmea {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Display for Nmea {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{}: lat: {} lon: {} alt: {} {:?}",
               self.fix_timestamp.map(|l| format!("{:?}", l)).unwrap_or("None".to_owned()),
               self.latitude.map(|l| format!("{:3.8}", l)).unwrap_or("None".to_owned()),
               self.longitude.map(|l| format!("{:3.8}", l)).unwrap_or("None".to_owned()),
               self.altitude.map(|l| format!("{:.3}", l)).unwrap_or("None".to_owned()),
               self.satellites())
    }
}

#[derive(Clone, PartialEq)]
/// ! A Satellite
pub struct Satellite {
    gnss_type: GnssType,
    prn: u32,
    elevation: Option<f32>,
    azimuth: Option<f32>,
    snr: Option<f32>,
}

impl Satellite {
    pub fn gnss_type(&self) -> GnssType {
        self.gnss_type.clone()
    }

    pub fn prn(&self) -> u32 {
        self.prn
    }

    pub fn elevation(&self) -> Option<f32> {
        self.elevation
    }

    pub fn azimuth(&self) -> Option<f32> {
        self.azimuth
    }

    pub fn snr(&self) -> Option<f32> {
        self.snr
    }
}

impl fmt::Display for Satellite {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{}: {} elv: {} ath: {} snr: {}",
               self.gnss_type,
               self.prn,
               self.elevation.map(|e| format!("{}", e)).unwrap_or("--".to_owned()),
               self.azimuth.map(|e| format!("{}", e)).unwrap_or("--".to_owned()),
               self.snr.map(|e| format!("{}", e)).unwrap_or("--".to_owned()))
    }
}

impl fmt::Debug for Satellite {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "[{:?},{:?},{:?},{:?},{:?}]",
               self.gnss_type,
               self.prn,
               self.elevation,
               self.azimuth,
               self.snr)
    }
}

macro_rules! define_sentence_type_enum {
    ($Name:ident { $($Variant:ident),* }) => {
        #[derive(PartialEq, Debug)]
        pub enum $Name {
            None,
            $($Variant),*,
        }

        impl<'a> From<&'a str> for $Name {
            fn from(s: &str) -> Self {
                match s {
                    $(stringify!($Variant) => $Name::$Variant,)*
                    _ => $Name::None,
                }
            }
        }
    }
}

#[test]
fn test_define_sentence_type_enum() {
    define_sentence_type_enum!( TestEnum {
        AAA,
        BBB
    }
    );

    let a = TestEnum::AAA;
    let b = TestEnum::BBB;
    let n = TestEnum::None;
    assert_eq!(TestEnum::from("AAA"), a);
    assert_eq!(TestEnum::from("BBB"), b);
    assert_eq!(TestEnum::from("fdafa"), n);
}

/// ! NMEA sentence type
/// ! General: OSD |
/// ! Autopilot: APA | APB | ASD |
/// ! Decca: DCN |
/// ! D-GPS: MSK
/// ! Echo: DBK | DBS | DBT |
/// ! Radio: FSI | SFI | TLL
/// ! Speed: VBW | VHW | VLW |
/// ! GPS: ALM | GBS | GGA | GNS | GSA | GSV |
/// ! Course: DPT | HDG | HDM | HDT | HSC | ROT | VDR |
/// ! Loran-C: GLC | LCD |
/// ! Machine: RPM |
/// ! Navigation: RMA | RMB | RMC |
/// ! Omega: OLN |
/// ! Position: GLL | DTM
/// ! Radar: RSD | TLL | TTM |
/// ! Rudder: RSA |
/// ! Temperature: MTW |
/// ! Transit: GXA | RTF |
/// ! Waypoints and tacks: AAM | BEC | BOD | BWC | BWR | BWW | ROO | RTE | VTG | WCV | WNC | WPL | XDR | XTE | XTR |
/// ! Wind: MWV | VPW | VWR |
/// ! Date and Time: GDT | ZDA | ZFO | ZTG |
define_sentence_type_enum!(SentenceType {
    AAM,
    ABK,
    ACA,
    ACK,
    ACS,
    AIR,
    ALM,
    ALR,
    APA,
    APB,
    ASD,
    BEC,
    BOD,
    BWC,
    BWR,
    BWW,
    CUR,
    DBK,
    DBS,
    DBT,
    DCN,
    DPT,
    DSC,
    DSE,
    DSI,
    DSR,
    DTM,
    FSI,
    GBS,
    GGA,
    GLC,
    GLL,
    GMP,
    GNS,
    GRS,
    GSA,
    GST,
    GSV,
    GTD,
    GXA,
    HDG,
    HDM,
    HDT,
    HMR,
    HMS,
    HSC,
    HTC,
    HTD,
    LCD,
    LRF,
    LRI,
    LR1,
    LR2,
    LR3,
    MLA,
    MSK,
    MSS,
    MWD,
    MTW,
    MWV,
    OLN,
    OSD,
    ROO,
    RMA,
    RMB,
    RMC,
    ROT,
    RPM,
    RSA,
    RSD,
    RTE,
    SFI,
    SSD,
    STN,
    TLB,
    TLL,
    TRF,
    TTM,
    TUT,
    TXT,
    VBW,
    VDM,
    VDO,
    VDR,
    VHW,
    VLW,
    VPW,
    VSD,
    VTG,
    VWR,
    WCV,
    WNC,
    WPL,
    XDR,
    XTE,
    XTR,
    ZDA,
    ZDL,
    ZFO,
    ZTG
});

/// ! Fix type
#[derive(Clone, PartialEq, Debug)]
pub enum FixType {
    Invalid,
    Gps,
    DGps,
    Pps,
    Rtk,
    FloatRtk,
    Estimated,
    Manual,
    Simulation,
}

/// ! GNSS type
#[derive (Debug, Clone, Hash, Eq, PartialEq)]
pub enum GnssType {
    Galileo,
    Gps,
    Glonass,
}

impl fmt::Display for GnssType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GnssType::Galileo => write!(f, "Galileo"),
            GnssType::Gps => write!(f, "GPS"),
            GnssType::Glonass => write!(f, "GLONASS"),
        }
    }
}

impl<'a> From<&'a str> for FixType {
    fn from(s: &'a str) -> Self {
        match s {
            "Invaild" => FixType::Invalid,
            "Gps" => FixType::Gps,
            "DGps" => FixType::DGps,
            "Pps" => FixType::Pps,
            "Rtk" => FixType::Rtk,
            "FloatRtk" => FixType::FloatRtk,
            "Estimated" => FixType::Estimated,
            "Manual" => FixType::Manual,
            "Simulation" => FixType::Simulation,
            _ => {
                match Nmea::parse_numeric::<u8>(s, 1) {
                    Ok(n) => {
                        match n {
                            0 => FixType::Invalid,
                            1 => FixType::Gps,
                            2 => FixType::DGps,
                            3 => FixType::Pps,
                            4 => FixType::Rtk,
                            5 => FixType::FloatRtk,
                            6 => FixType::Estimated,
                            7 => FixType::Manual,
                            8 => FixType::Simulation,
                            _ => FixType::Invalid,
                        }
                    }
                    Err(_) => FixType::Invalid,
                }
            }
        }

    }
}

lazy_static! {
    static ref REGEX_CHECKSUM: Regex = {
        Regex::new(r"^\$(?P<sentence>.*)\*(?P<checksum>..)$").unwrap()
    };
    static ref REGEX_TYPE: Regex = {
        Regex::new(r"^\D{2}(?P<type>\D{3}).*$").unwrap()
    };

    static ref REGEX_GGA: Regex = {
        Regex::new(r"^\D\DGGA,(?P<timestamp>\d{6})\.?\d*,(?P<lat>\d+\.\d+),(?P<lat_dir>[NS]),(?P<lon>\d+\.\d+),(?P<lon_dir>[WE]),(?P<fix_type>\d),(?P<fix_satellites>\d+),(?P<hdop>\d+\.\d+),(?P<alt>\d+\.\d+),\D,(?P<geoid_height>\d+\.\d+),\D,,").unwrap()
    };
    static ref REGEX_HMS: Regex = {
        Regex::new(r"^(?P<hour>\d\d)(?P<minute>\d\d)(?P<second>\d\d)$").unwrap()
    };
}


#[test]
fn test_fix_type() {
    assert_eq!(FixType::from(""), FixType::Invalid);
    assert_eq!(FixType::from("0"), FixType::Invalid);
    assert_eq!(FixType::from("1"), FixType::Gps);
    assert_eq!(FixType::from("2"), FixType::DGps);
    assert_eq!(FixType::from("3"), FixType::Pps);
    assert_eq!(FixType::from("4"), FixType::Rtk);
    assert_eq!(FixType::from("5"), FixType::FloatRtk);
    assert_eq!(FixType::from("6"), FixType::Estimated);
    assert_eq!(FixType::from("7"), FixType::Manual);
    assert_eq!(FixType::from("8"), FixType::Simulation);
}

#[test]
fn test_parse_numeric() {
    assert_eq!(Nmea::parse_numeric::<f32>("123.1", 1.0).unwrap(), 123.1);
    assert!(Nmea::parse_numeric::<f32>("123.a", 0.0).is_err());
    assert_eq!(Nmea::parse_numeric::<f32>("100.1", 2.0).unwrap(), 200.2);
    assert_eq!(Nmea::parse_numeric::<f32>("-10.0", 1.0).unwrap(), -10.0);
    assert_eq!(Nmea::parse_numeric::<f64>("123.1", 1.0).unwrap(), 123.1);
    assert!(Nmea::parse_numeric::<f64>("123.a", 0.0).is_err());
    assert_eq!(Nmea::parse_numeric::<f64>("100.1", 2.0).unwrap(), 200.2);
    assert_eq!(Nmea::parse_numeric::<f64>("-10.0", 1.0).unwrap(), -10.0);
    assert_eq!(Nmea::parse_numeric::<i32>("0", 0).unwrap(), 0);
    assert_eq!(Nmea::parse_numeric::<i32>("-10", 1).unwrap(), -10);
    assert_eq!(Nmea::parse_numeric::<u32>("0", 0).unwrap(), 0);
    assert!(Nmea::parse_numeric::<u32>("-1", 0).is_err());
    assert_eq!(Nmea::parse_numeric::<i8>("0", 0).unwrap(), 0);
    assert_eq!(Nmea::parse_numeric::<i8>("-10", 1).unwrap(), -10);
    assert_eq!(Nmea::parse_numeric::<u8>("0", 0).unwrap(), 0);
    assert!(Nmea::parse_numeric::<u8>("-1", 0).is_err());
}

#[test]
fn test_checksum() {
    let valid = "$GNGSA,A,1,,,,,,,,,,,,,99.99,99.99,99.99*2E";
    let invalid = "$GNZDA,165118.00,13,05,2016,00,00*71";
    assert_eq!(Nmea::checksum(&valid[1..valid.len() - 3]), 0x2E);
    assert_ne!(Nmea::checksum(&invalid[1..invalid.len() - 3]), 0x71);
}

#[test]
fn test_message_type() {
    let nmea = Nmea::new();
    let gga = "GPGGA,A,1,,,,,,,,,,,,,99.99,99.99,99.99";
    let fail = "GPXXX,A,1,,,,,,,,,,,,,99.99,99.99,99.99";
    assert_eq!(nmea.sentence_type(gga).unwrap(), SentenceType::GGA);
    assert_eq!(nmea.sentence_type(fail).unwrap(), SentenceType::None);
}

#[test]
fn test_gga_north_west() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76").unwrap();
    assert_eq!(nmea.fix_timestamp().unwrap().tm_sec, 50);
    assert_eq!(nmea.fix_timestamp().unwrap().tm_min, 27);
    assert_eq!(nmea.fix_timestamp().unwrap().tm_hour, 9);
    assert_eq!(nmea.latitude().unwrap(), 53. + 21.6802 / 60.);
    assert_eq!(nmea.longitude().unwrap(), -(6. + 30.3372 / 60.));
    assert_eq!(nmea.fix_type().unwrap(), FixType::Gps);
    assert_eq!(nmea.fix_satellites().unwrap(), 8);
    assert_eq!(nmea.hdop().unwrap(), 1.03);
    assert_eq!(nmea.geoid_height().unwrap(), 55.2);
}

#[test]
fn test_gga_north_east() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,N,00630.3372,E,1,8,1.03,61.7,M,55.2,M,,*64").unwrap();
    assert_eq!(nmea.latitude().unwrap(), 53. + 21.6802 / 60.);
    assert_eq!(nmea.longitude().unwrap(), 6. + 30.3372 / 60.);
}

#[test]
fn test_gga_south_west() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*6B").unwrap();
    assert_eq!(nmea.latitude().unwrap(), -(53. + 21.6802 / 60.));
    assert_eq!(nmea.longitude().unwrap(), -(6. + 30.3372 / 60.));
}

#[test]
fn test_gga_south_east() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,E,1,8,1.03,61.7,M,55.2,M,,*79").unwrap();
    assert_eq!(nmea.latitude().unwrap(), -(53. + 21.6802 / 60.));
    assert_eq!(nmea.longitude().unwrap(), 6. + 30.3372 / 60.);
}

#[test]
fn test_gga_invalid() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,E,0,8,1.03,61.7,M,55.2,M,,*7B")
        .unwrap_err();
    assert_eq!(nmea.fix_type(), None);
}

#[test]
fn test_gga_gps() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,E,1,8,1.03,61.7,M,55.2,M,,*79").unwrap();
    assert_eq!(nmea.fix_timestamp().unwrap().tm_sec, 50);
    assert_eq!(nmea.fix_timestamp().unwrap().tm_min, 27);
    assert_eq!(nmea.fix_timestamp().unwrap().tm_hour, 9);
    assert_eq!(-(53. + 21.6802 / 60.), nmea.latitude.unwrap());
    assert_eq!(6. + 30.3372 / 60., nmea.longitude.unwrap());
    assert_eq!(nmea.fix_type(), Some(FixType::Gps));
    assert_eq!(8, nmea.fix_satellites.unwrap());
    assert_eq!(1.03, nmea.hdop.unwrap());
    assert_eq!(61.7, nmea.altitude.unwrap());
    assert_eq!(55.2, nmea.geoid_height.unwrap());
}

#[test]
fn test_gsv() {
    let mut nmea = Nmea::new();
    //                        10           07           05           08
    nmea.parse("$GPGSV,3,1,11,10,63,137,17,07,61,098,15,05,59,290,20,08,54,157,30*70").unwrap();
    //                        02           13           26         04
    nmea.parse("$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79").unwrap();
    //                        29           16         36
    nmea.parse("$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76").unwrap();
    assert_eq!(nmea.satellites().len(), 11);

    let sat: &Satellite = &(nmea.satellites()[0]);
    assert_eq!(sat.gnss_type, GnssType::Gps);
    assert_eq!(sat.prn, 10);
    assert_eq!(sat.elevation, Some(63.0));
    assert_eq!(sat.azimuth, Some(137.0));
    assert_eq!(sat.snr, Some(17.0));
}

#[test]
fn test_gsv_real_data() {
    let mut nmea = Nmea::new();
    static REAL_DATA: [&'static str; 7] = [
        "$GPGSV,3,1,12,01,49,196,41,03,71,278,32,06,02,323,27,11,21,196,39*72",
        "$GPGSV,3,2,12,14,39,063,33,17,21,292,30,19,20,310,31,22,82,181,36*73",
        "$GPGSV,3,3,12,23,34,232,42,25,11,045,33,31,45,092,38,32,14,061,39*75",
        "$GLGSV,3,1,10,74,40,078,43,66,23,275,31,82,10,347,36,73,15,015,38*6B",
        "$GLGSV,3,2,10,75,19,135,36,65,76,333,31,88,32,233,33,81,40,302,38*6A",
        "$GLGSV,3,3,10,72,40,075,43,87,00,000,*6F",

        "$GPGSV,4,4,15,26,02,112,,31,45,071,,32,01,066,*4C"
    ];
    for line in &REAL_DATA {
        assert_eq!(nmea.parse(line).unwrap(), SentenceType::GSV);
    }
}

#[test]
fn test_gsv_order() {
    let mut nmea = Nmea::new();
    //                         2           13           26         04
    nmea.parse("$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79").unwrap();
    //                        29           16         36
    nmea.parse("$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76").unwrap();
    //                        10           07           05           08
    nmea.parse("$GPGSV,3,1,11,10,63,137,17,07,61,098,15,05,59,290,20,08,54,157,30*70").unwrap();
    assert_eq!(nmea.satellites().len(), 11);

    let sat: &Satellite = &(nmea.satellites()[0]);
    assert_eq!(sat.gnss_type, GnssType::Gps);
    assert_eq!(sat.prn, 10);
    assert_eq!(sat.elevation, Some(63.0));
    assert_eq!(sat.azimuth, Some(137.0));
    assert_eq!(sat.snr, Some(17.0));
}

#[test]
fn test_gsv_two_of_three() {
    let mut nmea = Nmea::new();
    //                         2           13           26          4
    nmea.parse("$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79").unwrap();
    //                        29           16         36
    nmea.parse("$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76").unwrap();
    assert_eq!(nmea.satellites().len(), 7);
}

#[test]
fn test_parse() {
    let sentences = ["$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76",
                     "$GPGSA,A,3,10,07,05,02,29,04,08,13,,,,,1.72,1.03,1.38*0A",
                     "$GPGSV,3,1,11,10,63,137,17,07,61,098,15,05,59,290,20,08,54,157,30*70",
                     "$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79",
                     "$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76",
                     "$GPRMC,092750.000,A,5321.6802,N,00630.3372,W,0.02,31.66,280511,,,A*43"];

    let mut nmea = Nmea::new();
    for s in &sentences {
        let res = nmea.parse(s);
        if s.starts_with("$GPGSA") || s.starts_with("$GPRMC") {
            res.unwrap_err();
        } else {
            res.unwrap();
        }
    }

    assert_eq!(nmea.latitude().unwrap(), 53. + 21.6802 / 60.);
    assert_eq!(nmea.longitude().unwrap(), -(6. + 30.3372 / 60.));
    assert_eq!(nmea.altitude().unwrap(), 61.7);
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::QuickCheck;

    fn check_parsing_lat_lon_in_gga(lat: f64, lon: f64) -> bool {
        let lat_min = (lat.abs() * 60.0) % 60.0;
        let lon_min = (lon.abs() * 60.0) % 60.0;
        let mut nmea = Nmea::new();
        nmea.parse_gga(
            &format!("GPGGA,092750.000,{lat_deg:02}{lat_min:09.6},{lat_dir},{lon_deg:03}{lon_min:09.6},{lon_dir},1,8,1.03,61.7,M,55.2,M,,",
                     lat_deg = lat.abs().floor() as u8, lon_deg = lon.abs().floor() as u8, lat_min = lat_min, lon_min = lon_min,
                     lat_dir = if lat.is_sign_positive() { 'N' } else { 'S' },
                     lon_dir = if lon.is_sign_positive() { 'E' } else { 'W' },
            )).unwrap();
        let (new_lat, new_lon) = (nmea.latitude.unwrap(), nmea.longitude.unwrap());
        const MAX_COOR_DIFF: f64 = 1e-7;
        (new_lat - lat).abs() < MAX_COOR_DIFF && (new_lon - lon).abs() < MAX_COOR_DIFF
    }

    #[test]
    fn test_parsing_lat_lon_in_gga() {
        // regressions found by quickcheck,
        // explicit because of quickcheck use random gen
        assert!(check_parsing_lat_lon_in_gga(0., 57.89528));
        assert!(check_parsing_lat_lon_in_gga(0., -43.33031));
        QuickCheck::new()
            .tests(10_000_000_000)
            .quickcheck(check_parsing_lat_lon_in_gga as fn(f64, f64) -> bool);
    }
}
