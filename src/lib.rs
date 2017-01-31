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
extern crate lazy_static;
extern crate regex;
extern crate chrono;

use regex::Regex;
use std::fmt;
use std::vec::Vec;
use std::collections::HashMap;
use chrono::{DateTime, UTC, Timelike};

/// ! NMEA parser
#[derive(Default)]
pub struct Nmea {
    fix_timestamp: Option<DateTime<UTC>>,
    fix_type: Option<FixType>,
    latitude: Option<f32>,
    longitude: Option<f32>,
    altitude: Option<f32>,
    fix_satellites: Option<u32>,
    hdop: Option<f32>,
    geoid_height: Option<f32>,
    satellites: Vec<Satellite>,
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
        let mut n = Nmea::default();
        n.satellites_scan.insert(GnssType::Galileo, vec![]);
        n.satellites_scan.insert(GnssType::Gps, vec![]);
        n.satellites_scan.insert(GnssType::Glonass, vec![]);
        n
    }

    /// Returns fix type
    pub fn fix_timestamp(&self) -> Option<DateTime<UTC>> {
        self.fix_timestamp
    }

    /// Returns fix type
    pub fn fix_type(&self) -> Option<FixType> {
        self.fix_type.clone()
    }

    /// Returns last fixed latitude in degress. None if not fixed.
    pub fn latitude(&self) -> Option<f32> {
        self.latitude
    }

    /// Returns last fixed longitude in degress. None if not fixed.
    pub fn longitude(&self) -> Option<f32> {
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
    pub fn sentence_type(&self, s: &'a str) -> Result<SentenceType, &'a str> {
        match REGEX_TYPE.captures(s) {
            Some(c) => {
                match c.name("type") {
                    Some(s) => {
                        match SentenceType::from(s.as_str()) {
                            SentenceType::None => Err("Unknown type"),
                            _ => Ok(SentenceType::from(s.as_str())),
                        }
                    }
                    _ => Err("Failed to parse type"),
                }
            }
            None => Err("Failed to parse type"),
        }
    }

    /// Parse a HHMMSS string into todays UTC datetime
    fn parse_hms(s: &'a str) -> Result<DateTime<UTC>, &'a str> {

        REGEX_HMS.captures(s)
            .and_then(|caps| {
                UTC::now().with_hour(
                    caps.get(1)
                        .and_then(|l| Self::parse_numeric::<u32>(l.as_str(), 1).ok()).unwrap_or(0))
                .unwrap()
                .with_minute(
                    caps.get(2)
                        .and_then(|l| Self::parse_numeric::<u32>(l.as_str(), 1).ok()).unwrap_or(0))
                .unwrap()
                .with_second(
                    caps.get(3)
                        .and_then(|l| Self::parse_numeric::<u32>(l.as_str(), 1).ok()).unwrap_or(0))
                .unwrap()
                .with_nanosecond(0)
            })
            .ok_or("Failed to parse time")
    }

    fn parse_gga(&mut self, sentence: &'a str) -> Result<SentenceType, &'a str> {
        match REGEX_GGA.captures(sentence) {
            Some(caps) => {
                self.fix_timestamp = caps.name("timestamp")
                    .and_then(|t| Self::parse_hms(t.as_str()).ok());
                self.fix_type = caps.name("fix_type").and_then(|t| Some(FixType::from(t.as_str())));
                self.latitude = caps.name("lat_dir").and_then(|s| {
                    match s.as_str() {
                        "N" => {
                            caps.name("lat")
                                .and_then(|l| Self::parse_numeric::<f32>(l.as_str(), 0.01).ok())
                        }
                        "S" => {
                            caps.name("lat")
                                .and_then(|l| Self::parse_numeric::<f32>(l.as_str(), -0.01).ok())
                        }
                        _ => None,
                    }
                });
                self.longitude = caps.name("lon_dir").and_then(|s| {
                    match s.as_str() {
                        "W" => {
                            caps.name("lon")
                                .and_then(|l| Self::parse_numeric::<f32>(l.as_str(), -0.01).ok())
                        }
                        "E" => {
                            caps.name("lon")
                                .and_then(|l| Self::parse_numeric::<f32>(l.as_str(), 0.01).ok())
                        }
                        _ => None,
                    }
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
            None => Err("Failed to parse GGA sentence"),
        }
    }

    fn parse_gsv(&mut self, sentence: &'a str) -> Result<SentenceType, &'a str> {
        match REGEX_GSV.captures(sentence) {
            Some(caps) => {
                let gnss_type = match caps.name("type") {
                    Some(t) => {
                        match t.as_str() {
                            "GP" => GnssType::Gps,
                            "GL" => GnssType::Glonass,
                            _ => return Err("Unknown GNSS type in GSV sentence"),
                        }
                    }
                    None => return Err("Failed to parse GSV sentence"),
                };

                let number = caps.name("number")
                    .ok_or("Failed to parse number of sats")
                    .and_then(|n| Self::parse_numeric::<usize>(n.as_str(), 1))?;
                let index = caps.name("index")
                    .ok_or("Failed to parse satellite index")
                    .and_then(|n| Self::parse_numeric::<usize>(n.as_str(), 1))?;

                {
                    let sats = caps.name("sats")
                        .ok_or("Failed to parse sats")
                        .and_then(|s| Self::parse_satellites(s.as_str(), &gnss_type))?;
                    let d = self.satellites_scan.get_mut(&gnss_type).ok_or("Invalid GNSS type")?;
                    // Adjust size to this scan
                    d.resize(number, vec!());
                    // Replace data at index with new scan data
                    d.push(sats);
                    d.swap_remove(index -1);
                }

                self.satellites.clear();
                for (_, v) in &self.satellites_scan {
                    for v1 in v {
                        for v2 in v1 {
                            self.satellites.push(v2.clone());
                        }
                    }
                }

                Ok(SentenceType::GSV)
            }
            None => Err("Failed to parse GSV sentence"),
        }
    }

    fn parse_satellites(satellites: &'a str,
                        gnss_type: &GnssType)
                        -> Result<Vec<Satellite>, &'a str> {
        let mut sats = vec!();
        let mut s = satellites.split(',');
        for _ in 0..3 {
            let s = Satellite {
                gnss_type: gnss_type.clone(),
                prn: s.next()
                    .ok_or("Failed to parse PRN")
                    .and_then(|a| Self::parse_numeric::<u32>(a, 1))?,
                elevation: s.next()
                    .ok_or("Failed to parse elevation")
                    .and_then(|a| Self::parse_numeric::<f32>(a, 1.0))?,
                azimuth: s.next()
                    .ok_or("Failed to parse azimuth")
                    .and_then(|a| Self::parse_numeric::<f32>(a, 1.0))?,
                snr: s.next()
                    .ok_or("Failed to parse SNR")
                    .and_then(|a| Self::parse_numeric::<f32>(a, 1.0))?,
            };
            if s.prn != 0 {
                sats.push(s);
            }
        }

        Ok(sats)
    }

    /// Parse any NMEA sentence and stores the result. The type of sentence
    /// is returnd if implemented and valid.
    pub fn parse(&mut self, s: &'a str) -> Result<SentenceType, &'a str> {
        if !Nmea::checksum(s)? {
            return Err("Checksum mismatch");
        }

        match self.sentence_type(&s)? {
            SentenceType::GGA => self.parse_gga(s),
            SentenceType::GSV => self.parse_gsv(s),
            _ => Err("Unknown or implemented type"),
        }
    }

    fn checksum(s: &str) -> Result<bool, &str> {
        let caps = REGEX_CHECKSUM.captures(s).ok_or("Failed to parse sentence")?;
        let sentence = caps.name(&"sentence").ok_or("Failed to parse sentence")?;
        let checksum =
            caps.name(&"checksum")
                .ok_or("Failed to parse checksum")
                .and_then(|c| {
                    u8::from_str_radix(c.as_str(), 16).map_err(|_| "Failed to parse checksun")
                })?;
        Ok(checksum == sentence.as_str().bytes().fold(0, |c, x| c ^ x))
    }

    fn parse_numeric<T>(input: &'a str, factor: T) -> Result<T, &str>
        where T: std::str::FromStr + std::ops::Mul<Output = T> + Copy
    {
        input.parse::<T>().map(|v| v * factor).map_err(|_| "Failed to parse number")
    }
}

impl fmt::Debug for Nmea {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Nmea {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{}: lat: {:3.3} lon: {:3.3} alt: {:5.3} {:?}",
               match self.fix_timestamp {
                   Some(d) => d.to_string(),
                   None => "no date".to_string(),
               },
               self.latitude.unwrap_or(0.0),
               self.longitude.unwrap_or(0.0),
               self.altitude.unwrap_or(0.0),
               self.satellites())
    }
}

#[derive (Clone)]
/// ! A Satellite
pub struct Satellite {
    gnss_type: GnssType,
    prn: u32,
    elevation: f32,
    azimuth: f32,
    snr: f32,
}

impl Satellite {
    pub fn gnss_type(&self) -> GnssType {
        self.gnss_type.clone()
    }

    pub fn prn(&self) -> u32 {
        self.prn
    }

    pub fn elevation(&self) -> f32 {
        self.elevation
    }

    pub fn azimuth(&self) -> f32 {
        self.azimuth
    }

    pub fn snr(&self) -> f32 {
        self.snr
    }
}

impl fmt::Display for Satellite {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{}: {} elv: {} ath: {} snr: {}",
               self.gnss_type,
               self.prn,
               self.elevation,
               self.azimuth,
               self.snr)
    }
}

impl fmt::Debug for Satellite {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "[{},{},{},{},{}]",
               self.gnss_type,
               self.prn,
               self.elevation,
               self.azimuth,
               self.snr)
    }
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
#[derive(PartialEq, Debug)]
pub enum SentenceType {
    None,
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
    ZTG,
}

impl<'a> From<&'a str> for SentenceType {
    fn from(s: &str) -> Self {
        match s {
            "AAM" => SentenceType::AAM,
            "ABK" => SentenceType::ACA,
            "ACA" => SentenceType::ACA,
            "ACK" => SentenceType::ACK,
            "ACS" => SentenceType::ACS,
            "AIR" => SentenceType::AIR,
            "ALM" => SentenceType::ALM,
            "ALR" => SentenceType::ALR,
            "APA" => SentenceType::APA,
            "APB" => SentenceType::APB,
            "ASD" => SentenceType::ASD,
            "BEC" => SentenceType::BEC,
            "BOD" => SentenceType::BOD,
            "BWC" => SentenceType::BWC,
            "BWR" => SentenceType::BWR,
            "BWW" => SentenceType::BWW,
            "CUR" => SentenceType::CUR,
            "DBK" => SentenceType::DBK,
            "DBS" => SentenceType::DBS,
            "DBT" => SentenceType::DBT,
            "DCN" => SentenceType::DCN,
            "DPT" => SentenceType::DPT,
            "DSC" => SentenceType::DSC,
            "DSE" => SentenceType::DSE,
            "DSI" => SentenceType::DSI,
            "DSR" => SentenceType::DSR,
            "DTM" => SentenceType::DTM,
            "FSI" => SentenceType::FSI,
            "GBS" => SentenceType::GBS,
            "GGA" => SentenceType::GGA,
            "GLC" => SentenceType::GLC,
            "GLL" => SentenceType::GLL,
            "GMP" => SentenceType::GMP,
            "GNS" => SentenceType::GNS,
            "GRS" => SentenceType::GRS,
            "GSA" => SentenceType::GSA,
            "GST" => SentenceType::GST,
            "GSV" => SentenceType::GSV,
            "GTD" => SentenceType::GTD,
            "GXA" => SentenceType::GXA,
            "HDG" => SentenceType::HDG,
            "HDM" => SentenceType::HDM,
            "HDT" => SentenceType::HDT,
            "HMR" => SentenceType::HMR,
            "HMS" => SentenceType::HMS,
            "HSC" => SentenceType::HSC,
            "HTC" => SentenceType::HTC,
            "HTD" => SentenceType::HTD,
            "LCD" => SentenceType::LCD,
            "LRF" => SentenceType::LRF,
            "LRI" => SentenceType::LRI,
            "LR1" => SentenceType::LR1,
            "LR2" => SentenceType::LR2,
            "LR3" => SentenceType::LR3,
            "MLA" => SentenceType::MLA,
            "MSK" => SentenceType::MSK,
            "MSS" => SentenceType::MSS,
            "MWD" => SentenceType::MWD,
            "MTW" => SentenceType::MTW,
            "MWV" => SentenceType::MWV,
            "OLN" => SentenceType::OLN,
            "OSD" => SentenceType::OSD,
            "ROO" => SentenceType::ROO,
            "RMA" => SentenceType::RMA,
            "RMB" => SentenceType::RMB,
            "RMC" => SentenceType::RMC,
            "ROT" => SentenceType::ROT,
            "RPM" => SentenceType::RPM,
            "RSA" => SentenceType::RSA,
            "RSD" => SentenceType::RSD,
            "RTE" => SentenceType::RTE,
            "SFI" => SentenceType::SFI,
            "SSD" => SentenceType::SSD,
            "STN" => SentenceType::STN,
            "TLB" => SentenceType::TLB,
            "TLL" => SentenceType::TLL,
            "TRF" => SentenceType::TRF,
            "TTM" => SentenceType::TTM,
            "TUT" => SentenceType::TUT,
            "TXT" => SentenceType::TXT,
            "VBW" => SentenceType::VBW,
            "VDM" => SentenceType::VDO,
            "VDO" => SentenceType::VDO,
            "VDR" => SentenceType::VDR,
            "VHW" => SentenceType::VHW,
            "VLW" => SentenceType::VLW,
            "VPW" => SentenceType::VPW,
            "VSD" => SentenceType::VSD,
            "VTG" => SentenceType::VTG,
            "VWR" => SentenceType::VWR,
            "WCV" => SentenceType::WCV,
            "WNC" => SentenceType::WNC,
            "WPL" => SentenceType::WPL,
            "XDR" => SentenceType::XDR,
            "XTE" => SentenceType::XTE,
            "XTR" => SentenceType::XTR,
            "ZDA" => SentenceType::ZDA,
            "ZDL" => SentenceType::ZDL,
            "ZFO" => SentenceType::ZFO,
            "ZTG" => SentenceType::ZTG,
            _ => SentenceType::None,
        }
    }
}

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
        Regex::new(r"^\$\D{2}(?P<type>\D{3}).*$").unwrap()
    };

    static ref REGEX_GGA: Regex = {
        Regex::new(r"^\$\D\DGGA,(?P<timestamp>\d{6})\.?\d*,(?P<lat>\d+\.\d+),(?P<lat_dir>[NS]),(?P<lon>\d+\.\d+),(?P<lon_dir>[WE]),(?P<fix_type>\d),(?P<fix_satellites>\d+),(?P<hdop>\d+\.\d+),(?P<alt>\d+\.\d+),\D,(?P<geoid_height>\d+\.\d+),\D,,\*([0-9a-fA-F][0-9a-fA-F])").unwrap()
    };
    static ref REGEX_HMS: Regex = {
        Regex::new(r"^(?P<hour>\d\d)(?P<minute>\d\d)(?P<second>\d\d)$").unwrap()
    };
    static ref REGEX_GSV: Regex = {
        Regex::new(r"^\$(?P<type>\D\D)GSV,(?P<number>\d+),(?P<index>\d+),(?P<sat_num>\d+),(?P<sats>.*)\*\d\d$").unwrap()
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
    assert_eq!(Nmea::parse_numeric::<f32>("123.1", 1.0), Ok(123.1));
    assert!(Nmea::parse_numeric::<f32>("123.a", 0.0).is_err());
    assert_eq!(Nmea::parse_numeric::<f32>("100.1", 2.0), Ok(200.2));
    assert_eq!(Nmea::parse_numeric::<f32>("-10.0", 1.0), Ok(-10.0));
    assert_eq!(Nmea::parse_numeric::<f64>("123.1", 1.0), Ok(123.1));
    assert!(Nmea::parse_numeric::<f64>("123.a", 0.0).is_err());
    assert_eq!(Nmea::parse_numeric::<f64>("100.1", 2.0), Ok(200.2));
    assert_eq!(Nmea::parse_numeric::<f64>("-10.0", 1.0), Ok(-10.0));
    assert_eq!(Nmea::parse_numeric::<i32>("0", 0), Ok(0));
    assert_eq!(Nmea::parse_numeric::<i32>("-10", 1), Ok(-10));
    assert_eq!(Nmea::parse_numeric::<u32>("0", 0), Ok(0));
    assert!(Nmea::parse_numeric::<u32>("-1", 0).is_err());
    assert_eq!(Nmea::parse_numeric::<i8>("0", 0), Ok(0));
    assert_eq!(Nmea::parse_numeric::<i8>("-10", 1), Ok(-10));
    assert_eq!(Nmea::parse_numeric::<u8>("0", 0), Ok(0));
    assert!(Nmea::parse_numeric::<u8>("-1", 0).is_err());
}

#[test]
fn test_checksum() {
    let valid = "$GNGSA,A,1,,,,,,,,,,,,,99.99,99.99,99.99*2E";
    let invalid = "$GNZDA,165118.00,13,05,2016,00,00*71";
    let parse_error = "";
    assert_eq!(Nmea::checksum(valid), Ok(true));
    assert_eq!(Nmea::checksum(invalid), Ok(false));
    assert!(Nmea::checksum(parse_error).is_err());
}

#[test]
fn test_message_type() {
    let nmea = Nmea::new();
    let gga = "$GPGGA,A,1,,,,,,,,,,,,,99.99,99.99,99.99*2E";
    let fail = "$GPXXX,A,1,,,,,,,,,,,,,99.99,99.99,99.99*2E";
    assert_eq!(nmea.sentence_type(gga).unwrap(), SentenceType::GGA);
    assert!(nmea.sentence_type(fail).is_err());
}

#[test]
fn test_gga_north_west() {
    let date = UTC::now()
        .with_hour(9)
        .unwrap()
        .with_minute(27)
        .unwrap()
        .with_second(50)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();

    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76").ok();
    assert_eq!(nmea.fix_timestamp().unwrap(), date);
    assert_eq!(nmea.latitude().unwrap(), 53.216802);
    assert_eq!(nmea.longitude().unwrap(), -6.303372);
    assert_eq!(nmea.fix_type().unwrap(), FixType::Gps);
    assert_eq!(nmea.fix_satellites().unwrap(), 8);
    assert_eq!(nmea.hdop().unwrap(), 1.03);
    assert_eq!(nmea.geoid_height().unwrap(), 55.2);
}

#[test]
fn test_gga_north_east() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,N,00630.3372,E,1,8,1.03,61.7,M,55.2,M,,*64").ok();
    assert_eq!(nmea.latitude().unwrap(), 53.216802);
    assert_eq!(nmea.longitude().unwrap(), 6.303372);
}

#[test]
fn test_gga_south_west() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*6B").ok();
    assert_eq!(nmea.latitude().unwrap(), -53.216802);
    assert_eq!(nmea.longitude().unwrap(), -6.303372);
}

#[test]
fn test_gga_south_east() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,E,1,8,1.03,61.7,M,55.2,M,,*79").ok();
    assert_eq!(nmea.latitude().unwrap(), -53.216802);
    assert_eq!(nmea.longitude().unwrap(), 6.303372);
}

#[test]
fn test_gga_invalid() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,E,0,8,1.03,61.7,M,55.2,M,,*7B").ok();
    assert_eq!(nmea.fix_type(), None);
}

#[test]
fn test_gga_gps() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,E,1,8,1.03,61.7,M,55.2,M,,*79").ok();
    assert_eq!(nmea.fix_type(), Some(FixType::Gps));
}

#[test]
fn test_gsv() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGSV,3,1,11,10,63,137,17,07,61,098,15,05,59,290,20,08,54,157,30*70").ok();
    nmea.parse("$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79").ok();
    nmea.parse("$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76").ok();
    assert_eq!(nmea.satellites().len(), 9);

    let sat: &Satellite = &(nmea.satellites()[0]);
    assert_eq!(sat.gnss_type, GnssType::Gps);
    assert_eq!(sat.prn, 10);
    assert_eq!(sat.elevation, 63.0);
    assert_eq!(sat.azimuth, 137.0);
    assert_eq!(sat.snr, 17.0);
}

#[test]
fn test_gsv_order() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79").ok();
    nmea.parse("$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76").ok();
    nmea.parse("$GPGSV,3,1,11,10,63,137,17,07,61,098,15,05,59,290,20,08,54,157,30*70").ok();
    assert_eq!(nmea.satellites().len(), 9);

    let sat: &Satellite = &(nmea.satellites()[0]);
    assert_eq!(sat.gnss_type, GnssType::Gps);
    assert_eq!(sat.prn, 10);
    assert_eq!(sat.elevation, 63.0);
    assert_eq!(sat.azimuth, 137.0);
    assert_eq!(sat.snr, 17.0);
}

#[test]
fn test_gsv_two_of_three() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79").ok();
    nmea.parse("$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76").ok();
    assert_eq!(nmea.satellites().len(), 6);
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
        nmea.parse(s).ok();
    }

    assert_eq!(nmea.latitude().unwrap(), 53.216802);
    assert_eq!(nmea.longitude().unwrap(), -6.303372);
    assert_eq!(nmea.altitude().unwrap(), 61.7);
}
