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

/// ! NMEA parser
pub struct Nmea {
    fix_timestamp: Option<DateTime<UTC>>,
    fix_type: FixType,
    latitude: Option<f32>,
    longitude: Option<f32>,
    altitude: Option<f32>,
    fix_satellites: Option<u32>,
    hdop: Option<f32>,
    geoid_height: Option<f32>,
    satellites: Vec<Satellite>,
    satellites_scan: HashMap<GnssType, Vec<Vec<Satellite>>>,
}

impl Nmea {
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
        let mut n = Nmea {
            fix_timestamp: None,
            fix_type: FixType::Invalid,
            latitude: None,
            longitude: None,
            altitude: None,
            fix_satellites: None,
            hdop: None,
            geoid_height: None,
            satellites: Vec::new(),
            satellites_scan: HashMap::new(),
        };
        n.satellites_scan.insert(GnssType::Galileo, Vec::new());
        n.satellites_scan.insert(GnssType::Gps, Vec::new());
        n.satellites_scan.insert(GnssType::Glonass, Vec::new());
        n
    }

    /// Returns fix type
    pub fn fix_timestamp(&self) -> Option<DateTime<UTC>> {
        self.fix_timestamp
    }

    /// Returns fix type
    pub fn fix_type(&self) -> FixType {
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
    pub fn satellites(&self) -> &Vec<Satellite> {
        &self.satellites
    }

    /// Returns the NMEA sentence type.
    pub fn sentence_type(&self, s: &str) -> Result<SentenceType, &'static str> {
        match REGEX_TYPE.captures(s) {
            Some(c) => {
                match c.name("type") {
                    Some(s) => {
                        match SentenceType::from(s) {
                            SentenceType::None => Err("Unknown type"),
                            _ => Ok(SentenceType::from(s)),
                        }
                    }
                    _ => Err("Failed to parse type"),
                }
            }
            None => Err("Failed to parse type"),
        }
    }

    /// Parse a HHMMSS string into todays UTC datetime
    fn parse_hms(s: &str) -> Option<DateTime<UTC>> {
        match REGEX_HMS.captures(s) {
            Some(c) => {
                let hour = match Self::parse_numeric::<u32>(c.at(1), 1) {
                    Some(h) => {
                        if h > 24 {
                            return None;
                        } else {
                            h
                        }
                    }
                    None => return None,
                };
                let minute = match Self::parse_numeric::<u32>(c.at(2), 1) {
                    Some(h) => {
                        if h > 59 {
                            return None;
                        } else {
                            h
                        }
                    }
                    None => return None,
                };
                let second = match Self::parse_numeric::<u32>(c.at(3), 1) {
                    Some(h) => {
                        if h > 59 {
                            return None;
                        } else {
                            h
                        }
                    }
                    None => return None,
                };

                Some(UTC::now()
                    .with_hour(hour)
                    .unwrap()
                    .with_minute(minute)
                    .unwrap()
                    .with_second(second)
                    .unwrap()
                    .with_nanosecond(0)
                    .unwrap())
            }
            None => None,
        }
    }

    fn parse_gga(&mut self, sentence: &str) -> Result<SentenceType, &'static str> {
        match REGEX_GGA.captures(sentence) {
            Some(caps) => {
                self.fix_timestamp = match caps.name("timestamp") {
                    Some(time_str) => Self::parse_hms(time_str),
                    None => None,
                };
                self.fix_type = FixType::from(caps.name("fix_type").unwrap_or("Invalid"));
                self.latitude = match caps.name("lat_dir") {
                    Some(s) => {
                        match s {
                            "N" => Self::parse_numeric::<f32>(caps.name("lat"), 0.01),
                            "S" => Self::parse_numeric::<f32>(caps.name("lat"), -0.01),
                            _ => None,
                        }
                    }
                    None => None,
                };
                self.longitude = match caps.name("lon_dir") {
                    Some(s) => {
                        match s {
                            "W" => Self::parse_numeric::<f32>(caps.name("lon"), -0.01),
                            "E" => Self::parse_numeric::<f32>(caps.name("lon"), 0.01),
                            _ => None,
                        }
                    }
                    None => None,
                };
                self.altitude = Self::parse_numeric::<f32>(caps.name("alt"), 1.0);
                self.fix_satellites = Self::parse_numeric::<u32>(caps.name("fix_satellites"), 1);
                self.hdop = Self::parse_numeric::<f32>(caps.name("hdop"), 1.0);
                self.geoid_height = Self::parse_numeric::<f32>(caps.name("geoid_height"), 1.0);
                Ok(SentenceType::GGA)
            }
            None => Err("Failed to parse GGA sentence"),
        }
    }

    fn parse_gsv(&mut self, sentence: &str) -> Result<SentenceType, &'static str> {
        match REGEX_GSV.captures(sentence) {
            Some(caps) => {
                let gnss_type = match caps.name("type") {
                    Some(t) => {
                        match t {
                            "GP" => GnssType::Gps,
                            "GL" => GnssType::Glonass,
                            _ => return Err("Unknown GNSS type in GSV sentence"),
                        }
                    }
                    None => return Err("Failed to parse GSV sentence"),
                };

                let number = match Self::parse_numeric::<u8>(caps.name("number"), 1) {
                    Some(n) => n,
                    None => return Err("Failed to parse index"),
                };

                let index = match Self::parse_numeric::<u8>(caps.name("index"), 1) {
                    Some(n) => n,
                    None => return Err("Failed to parse index"),
                };

                {
                    let d = match self.satellites_scan.get_mut(&gnss_type) {
                        Some(v) => v,
                        None => return Err("Invalid GNSS type"),
                    };

                    if d.len() < number as usize {
                        for _ in 0..(number as usize - d.len()) {
                            d.push(Vec::new());
                        }
                    } else {
                        d.truncate(number as usize);
                    }

                    let satellites = match caps.name("sats") {
                        Some(sats) => Self::parse_satellites(sats, &gnss_type),
                        None => return Err("Failed to parse satellites in view"),
                    };

                    d[(index - 1) as usize] = satellites;
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

    fn parse_satellites(satellites: &str, gnss_type: &GnssType) -> Vec<Satellite> {
        let mut sats = Vec::new();

        let mut s = satellites.split(',');
        for _ in 0..3 {
            let s = Satellite {
                gnss_type: gnss_type.clone(),
                prn: Self::parse_numeric::<u32>(s.next(), 1).unwrap_or(0),
                elevation: Self::parse_numeric::<f32>(s.next(), 1.0).unwrap_or(0.0),
                azimuth: Self::parse_numeric::<f32>(s.next(), 1.0).unwrap_or(0.0),
                snr: Self::parse_numeric::<f32>(s.next(), 1.0).unwrap_or(0.0),
            };
            if s.prn != 0 {
                sats.push(s);
            }
        }

        sats
    }

    /// Parse any NMEA sentence and stores the result. The type of sentence
    /// is returnd if implemented and valid.
    pub fn parse(&mut self, s: &str) -> Result<SentenceType, &'static str> {
        if s.len() == 0 {
            return Err("Empty sentence");
        }

        if !Nmea::checksum(s) {
            return Err("checksum test failed");
        }

        let sentence_type = match self.sentence_type(s) {
            Ok(t) => t,
            Err(e) => return Err(e),
        };

        match sentence_type {
            SentenceType::GGA => self.parse_gga(s),
            SentenceType::GSV => self.parse_gsv(s),
            _ => Err("Unknown or implemented type"),
        }
    }

    fn checksum(s: &str) -> bool {
        let caps = match REGEX_CHECKSUM.captures(s) {
            Some(c) => c,
            None => return false,
        };
        let sentence = match caps.name(&"sentence") {
            Some(v) => v,
            None => return false,
        };
        let checksum = match u8::from_str_radix(caps.name(&"checksum").unwrap_or(""), 16) {
            Ok(v) => v,
            Err(_) => return false,
        };
        let c = sentence.bytes().fold(0, |c, x| c ^ x);
        println!("{}: {}", s, c);
        c == checksum
    }

    fn parse_numeric<T>(input: Option<&str>, factor: T) -> Option<T>
        where T: std::str::FromStr + std::ops::Mul<Output = T> + Copy
    {
        match input {
            Some(s) => {
                match s.parse::<T>() {
                    Ok(v) => Some(v * factor),
                    Err(_) => None,
                }
            }
            None => None,
        }
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

/// ! NMEA sentence type
#[derive(PartialEq, Debug)]
pub enum SentenceType {
    None,
    GGA,
    GSV,
}

impl<'a> From<&'a str> for SentenceType {
    fn from(s: &str) -> Self {
        match s {
            "GGA" => SentenceType::GGA,
            "GSV" => SentenceType::GSV,
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
    fn from(s: &str) -> Self {
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
                match Nmea::parse_numeric::<u8>(Some(s), 1) {
                    Some(n) => {
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
                    None => FixType::Invalid,
                }
            }
        }

    }
}

#[test]
fn test_fix_type() {
    assert_eq!(FixType::from(""), FixType::Invalid);
    assert_eq!(FixType::from("42"), FixType::Invalid);
    assert_eq!(FixType::from("256"), FixType::Invalid);
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
    assert_eq!(Nmea::parse_numeric::<f32>(Some("123.1"), 1.0), Some(123.1));
    assert_eq!(Nmea::parse_numeric::<f32>(Some("123.a"), 1.0), None);
    assert_eq!(Nmea::parse_numeric::<f32>(Some("100.1"), 2.0), Some(200.2));
    assert_eq!(Nmea::parse_numeric::<f32>(Some("-10.0"), 1.0), Some(-10.0));
    assert_eq!(Nmea::parse_numeric::<f64>(Some("123.1"), 1.0), Some(123.1));
    assert_eq!(Nmea::parse_numeric::<f64>(Some("123.a"), 1.0), None);
    assert_eq!(Nmea::parse_numeric::<f64>(Some("100.1"), 2.0), Some(200.2));
    assert_eq!(Nmea::parse_numeric::<f64>(Some("-10.0"), 1.0), Some(-10.0));
    assert_eq!(Nmea::parse_numeric::<i32>(Some("0"), 0), Some(0));
    assert_eq!(Nmea::parse_numeric::<i32>(Some("-10"), 1), Some(-10));
    assert_eq!(Nmea::parse_numeric::<u32>(Some("0"), 0), Some(0));
    assert_eq!(Nmea::parse_numeric::<u32>(Some("-1"), 0), None);
    assert_eq!(Nmea::parse_numeric::<i8>(Some("0"), 0), Some(0));
    assert_eq!(Nmea::parse_numeric::<i8>(Some("-10"), 1), Some(-10));
    assert_eq!(Nmea::parse_numeric::<u8>(Some("0"), 0), Some(0));
    assert_eq!(Nmea::parse_numeric::<u8>(Some("-1"), 0), None);
}

#[test]
fn test_checksum() {
    let valid = "$GNGSA,A,1,,,,,,,,,,,,,99.99,99.99,99.99*2E";
    let invalid = "$GNZDA,165118.00,13,05,2016,00,00*71";
    let parse_error = "";
    assert_eq!(Nmea::checksum(valid), true);
    assert_eq!(Nmea::checksum(invalid), false);
    assert!(!Nmea::checksum(parse_error));
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
    assert_eq!(nmea.fix_type(), FixType::Gps);
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
    assert_eq!(nmea.fix_type(), FixType::Invalid);
}

#[test]
fn test_gga_gps() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,E,1,8,1.03,61.7,M,55.2,M,,*79").ok();
    assert_eq!(nmea.fix_type(), FixType::Gps);
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
    let sentences = [
        "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76",
        "$GPGSA,A,3,10,07,05,02,29,04,08,13,,,,,1.72,1.03,1.38*0A",
        "$GPGSV,3,1,11,10,63,137,17,07,61,098,15,05,59,290,20,08,54,157,30*70",
        "$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79",
        "$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76",
        "$GPRMC,092750.000,A,5321.6802,N,00630.3372,W,0.02,31.66,280511,,,A*43",
    ];

    let mut nmea = Nmea::new();
    for s in &sentences {
        nmea.parse(s).ok();
    }

    assert_eq!(nmea.latitude().unwrap(), 53.216802);
    assert_eq!(nmea.longitude().unwrap(), -6.303372);
    assert_eq!(nmea.altitude().unwrap(), 61.7);
}
