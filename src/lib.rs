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

#[cfg(test)]
extern crate quickcheck;
#[macro_use]
extern crate nom;
extern crate chrono;
#[cfg(test)]
#[macro_use]
extern crate approx;

mod parse;

use std::collections::HashMap;
use std::{fmt, str, mem};
use std::vec::Vec;
use std::iter::Iterator;
use std::collections::HashSet;

use chrono::{NaiveTime, Date, UTC};
pub use parse::{GsvData, GgaData, RmcData, RmcStatusOfFix, parse, ParseResult, GsaData, VtgData};


/// NMEA parser
#[derive(Default)]
pub struct Nmea {
    pub fix_time: Option<NaiveTime>,
    pub fix_date: Option<Date<UTC>>,
    pub fix_type: Option<FixType>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f32>,
    pub speed_over_ground: Option<f32>,
    pub true_course: Option<f32>,
    pub num_of_fix_satellites: Option<u32>,
    pub hdop: Option<f32>,
    pub vdop: Option<f32>,
    pub pdop: Option<f32>,
    pub geoid_height: Option<f32>,
    pub satellites: Vec<Satellite>,
    pub fix_satellites_prns: Option<Vec<u32>>,
    satellites_scan: HashMap<GnssType, Vec<Vec<Satellite>>>,
    required_sentences_for_nav: HashSet<SentenceType>,
    last_fix_time: Option<NaiveTime>,
    sentences_for_this_time: HashSet<SentenceType>,
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

    /// Constructs a new `Nmea` for navigation purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// use nmea::{Nmea, SentenceType};
    ///
    /// let mut nmea = Nmea::create_for_navigation([SentenceType::RMC, SentenceType::GGA]
    ///                                                .iter()
    ///                                                .map(|v| v.clone())
    ///                                                .collect()).unwrap();
    /// let gga = "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76";
    /// nmea.parse(gga).unwrap();
    /// println!("{}", nmea);
    /// ```
    pub fn create_for_navigation(required_sentences_for_nav: HashSet<SentenceType>)
                                 -> Result<Nmea, &'static str> {
        if required_sentences_for_nav.is_empty() {
            return Err("Should be at least one sentence type in required");
        }
        let mut n = Self::new();
        n.required_sentences_for_nav = required_sentences_for_nav;
        Ok(n)
    }


    /// Returns fix type
    pub fn fix_timestamp(&self) -> Option<NaiveTime> {
        self.fix_time
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
        self.num_of_fix_satellites
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

    fn merge_gga_data(&mut self, gga_data: GgaData) {
        self.fix_time = gga_data.fix_timestamp_time;
        self.latitude = gga_data.latitude;
        self.longitude = gga_data.longitude;
        self.fix_type = gga_data.fix_type;
        self.num_of_fix_satellites = gga_data.fix_satellites;
        self.hdop = gga_data.hdop;
        self.altitude = gga_data.altitude;
        self.geoid_height = gga_data.geoid_height;
    }

    fn merge_gsv_data(&mut self, data: GsvData) -> Result<(), &'static str> {
        {
            let d = self.satellites_scan
                .get_mut(&data.gnss_type)
                .ok_or("Invalid GNSS type")?;
            // Adjust size to this scan
            d.resize(data.number_of_sentences as usize, vec![]);
            // Replace data at index with new scan data
            d.push(data.sats_info
                       .iter()
                       .filter(|v| v.is_some())
                       .map(|v| v.clone().unwrap())
                       .collect());
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

    fn merge_rmc_data(&mut self, rmc_data: RmcData) {
        self.fix_time = rmc_data.fix_time.map(|v| v.time());
        self.fix_date = rmc_data.fix_time.map(|v| v.date());
        self.fix_type = rmc_data
            .status_of_fix
            .map(|v| match v {
                     RmcStatusOfFix::Autonomous => FixType::Gps,
                     RmcStatusOfFix::Differential => FixType::DGps,
                     RmcStatusOfFix::Invalid => FixType::Invalid,
                 });
        self.latitude = rmc_data.lat;
        self.longitude = rmc_data.lon;
        self.speed_over_ground = rmc_data.speed_over_ground;
        self.true_course = rmc_data.true_course;
    }

    fn merge_gsa_data(&mut self, gsa: GsaData) {
        self.fix_satellites_prns = Some(gsa.fix_sats_prn);
        self.hdop = Some(gsa.hdop);
        self.vdop = Some(gsa.vdop);
        self.pdop = Some(gsa.pdop);
    }

    fn merge_vtg_data(&mut self, vtg: VtgData) {
        self.speed_over_ground = vtg.speed_over_ground;
        self.true_course = vtg.true_course;
    }

    /// Parse any NMEA sentence and stores the result. The type of sentence
    /// is returnd if implemented and valid.
    pub fn parse(&mut self, s: &'a str) -> Result<SentenceType, String> {
        match parse(s.as_bytes())? {
            ParseResult::VTG(vtg) => {
                self.merge_vtg_data(vtg);
                Ok(SentenceType::VTG)
            }
            ParseResult::GGA(gga) => {
                self.merge_gga_data(gga);
                Ok(SentenceType::GGA)
            }
            ParseResult::GSV(gsv) => {
                self.merge_gsv_data(gsv)?;
                Ok(SentenceType::GSV)
            }
            ParseResult::RMC(rmc) => {
                self.merge_rmc_data(rmc);
                Ok(SentenceType::RMC)
            }
            ParseResult::GSA(gsa) => {
                self.merge_gsa_data(gsa);
                Ok(SentenceType::GSA)
            }
            ParseResult::Unsupported(msg_id) => {
                Err(format!("Unknown or implemented sentence type: {:?}", msg_id))
            }
        }
    }

    fn new_tick(&mut self) {
        let old = mem::replace(self, Self::default());
        self.satellites_scan = old.satellites_scan;
        self.satellites = old.satellites;
        self.required_sentences_for_nav = old.required_sentences_for_nav;
        self.last_fix_time = old.last_fix_time;
    }

    fn clear_position_info(&mut self) {
        self.last_fix_time = None;
        self.new_tick();
    }

    pub fn parse_for_fix(&mut self, xs: &[u8]) -> Result<FixType, String> {
        match parse(xs)? {
            ParseResult::GSA(gsa) => {
                self.merge_gsa_data(gsa);
                return Ok(FixType::Invalid);
            }
            ParseResult::GSV(gsv_data) => {
                self.merge_gsv_data(gsv_data)?;
                return Ok(FixType::Invalid);
            }
            ParseResult::VTG(vtg) => {
                //have no time field, so only if user explicity mention it
                if self.required_sentences_for_nav
                       .contains(&SentenceType::VTG) {
                    if vtg.true_course.is_none() || vtg.speed_over_ground.is_none() {
                        self.clear_position_info();
                        return Ok(FixType::Invalid);
                    }
                    self.merge_vtg_data(vtg);
                    self.sentences_for_this_time.insert(SentenceType::VTG);
                } else {
                    return Ok(FixType::Invalid);
                }
            }
            ParseResult::RMC(rmc_data) => {
                match rmc_data.status_of_fix {
                    Some(RmcStatusOfFix::Invalid) |
                    None => {
                        self.clear_position_info();
                        return Ok(FixType::Invalid);
                    }
                    _ => { /*nothing*/ }
                }
                match (self.last_fix_time, rmc_data.fix_time) {
                    (Some(ref last_fix_time), Some(ref rmc_fix_time)) => {
                        if *last_fix_time != rmc_fix_time.time() {
                            self.new_tick();
                            self.last_fix_time = Some(rmc_fix_time.time());
                        }
                    }
                    (None, Some(ref rmc_fix_time)) => {
                        self.last_fix_time = Some(rmc_fix_time.time())
                    }
                    (Some(_), None) | (None, None) => {
                        self.clear_position_info();
                        return Ok(FixType::Invalid);
                    }
                }
                self.merge_rmc_data(rmc_data);
                self.sentences_for_this_time.insert(SentenceType::RMC);
            }
            ParseResult::GGA(gga_data) => {
                match gga_data.fix_type {
                    Some(FixType::Invalid) |
                    None => {
                        self.clear_position_info();
                        return Ok(FixType::Invalid);
                    }
                    _ => { /*nothing*/ }
                }
                match (self.last_fix_time, gga_data.fix_timestamp_time) {
                    (Some(ref last_fix_time), Some(ref gga_fix_time)) => {
                        if last_fix_time != gga_fix_time {
                            self.new_tick();
                            self.last_fix_time = Some(*gga_fix_time);
                        }
                    }
                    (None, Some(ref gga_fix_time)) => self.last_fix_time = Some(*gga_fix_time),
                    (Some(_), None) | (None, None) => {
                        self.clear_position_info();
                        return Ok(FixType::Invalid);
                    }
                }
                self.merge_gga_data(gga_data);
                self.sentences_for_this_time.insert(SentenceType::GGA);
            }
            ParseResult::Unsupported(_) => {
                return Ok(FixType::Invalid);
            }
        }
        match self.fix_type {
            Some(FixType::Invalid) |
            None => Ok(FixType::Invalid),
            Some(ref fix_type) if self.required_sentences_for_nav
                                      .is_subset(&self.sentences_for_this_time) => {
                Ok(fix_type.clone())
            }
            _ => Ok(FixType::Invalid),
        }
    }
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
               self.fix_time
                   .map(|l| format!("{:?}", l))
                   .unwrap_or("None".to_owned()),
               self.latitude
                   .map(|l| format!("{:3.8}", l))
                   .unwrap_or("None".to_owned()),
               self.longitude
                   .map(|l| format!("{:3.8}", l))
                   .unwrap_or("None".to_owned()),
               self.altitude
                   .map(|l| format!("{:.3}", l))
                   .unwrap_or("None".to_owned()),
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
               self.elevation
                   .map(|e| format!("{}", e))
                   .unwrap_or("--".to_owned()),
               self.azimuth
                   .map(|e| format!("{}", e))
                   .unwrap_or("--".to_owned()),
               self.snr
                   .map(|e| format!("{}", e))
                   .unwrap_or("--".to_owned()))
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
    ($Name:ident { $($Variant:ident),* $(,)* }) => {
        #[derive(PartialEq, Debug, Hash, Eq, Clone)]
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

        impl $Name {
            fn try_from(s: &[u8]) -> Result<Self, &'static str> {
                match str::from_utf8(s).map_err(|_| "invalid header")? {
                    $(stringify!($Variant) => Ok($Name::$Variant),)*
                    _ => Ok($Name::None),
                }
            }
        }
    }
}

#[test]
fn test_define_sentence_type_enum() {
    define_sentence_type_enum!(TestEnum { AAA, BBB });

    let a = TestEnum::AAA;
    let b = TestEnum::BBB;
    let n = TestEnum::None;
    assert_eq!(TestEnum::from("AAA"), a);
    assert_eq!(TestEnum::from("BBB"), b);
    assert_eq!(TestEnum::from("fdafa"), n);

    assert_eq!(TestEnum::try_from(b"AAA").unwrap(), a);
    assert_eq!(TestEnum::try_from(b"BBB").unwrap(), b);
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
/// ! Waypoints and tacks: AAM | BEC | BOD | BWC | BWR | BWW | ROO | RTE |
/// !                      VTG | WCV | WNC | WPL | XDR | XTE | XTR |
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
                               ZTG,
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

impl From<char> for FixType {
    fn from(x: char) -> Self {
        match x {
            '0' => FixType::Invalid,
            '1' => FixType::Gps,
            '2' => FixType::DGps,
            '3' => FixType::Pps,
            '4' => FixType::Rtk,
            '5' => FixType::FloatRtk,
            '6' => FixType::Estimated,
            '7' => FixType::Manual,
            '8' => FixType::Simulation,
            _ => FixType::Invalid,
        }
    }
}

#[test]
fn test_fix_type() {
    assert_eq!(FixType::from('A'), FixType::Invalid);
    assert_eq!(FixType::from('0'), FixType::Invalid);
    assert_eq!(FixType::from('1'), FixType::Gps);
    assert_eq!(FixType::from('2'), FixType::DGps);
    assert_eq!(FixType::from('3'), FixType::Pps);
    assert_eq!(FixType::from('4'), FixType::Rtk);
    assert_eq!(FixType::from('5'), FixType::FloatRtk);
    assert_eq!(FixType::from('6'), FixType::Estimated);
    assert_eq!(FixType::from('7'), FixType::Manual);
    assert_eq!(FixType::from('8'), FixType::Simulation);
}

#[test]
fn test_checksum() {
    use parse::checksum;
    let valid = "$GNGSA,A,1,,,,,,,,,,,,,99.99,99.99,99.99*2E";
    let invalid = "$GNZDA,165118.00,13,05,2016,00,00*71";
    assert_eq!(checksum((&valid[1..valid.len() - 3]).as_bytes().iter()),
               0x2E);
    assert_ne!(checksum((&invalid[1..invalid.len() - 3]).as_bytes().iter()),
               0x71);
}

#[test]
fn test_message_type() {
    assert_eq!(SentenceType::try_from(b"GGA").unwrap(), SentenceType::GGA);
    assert_eq!(SentenceType::try_from(b"XXX").unwrap(), SentenceType::None);
}

#[test]
fn test_gga_north_west() {
    use chrono::Timelike;
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76")
        .unwrap();
    assert_eq!(nmea.fix_timestamp().unwrap().second(), 50);
    assert_eq!(nmea.fix_timestamp().unwrap().minute(), 27);
    assert_eq!(nmea.fix_timestamp().unwrap().hour(), 9);
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
    nmea.parse("$GPGGA,092750.000,5321.6802,N,00630.3372,E,1,8,1.03,61.7,M,55.2,M,,*64")
        .unwrap();
    assert_eq!(nmea.latitude().unwrap(), 53. + 21.6802 / 60.);
    assert_eq!(nmea.longitude().unwrap(), 6. + 30.3372 / 60.);
}

#[test]
fn test_gga_south_west() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*6B")
        .unwrap();
    assert_eq!(nmea.latitude().unwrap(), -(53. + 21.6802 / 60.));
    assert_eq!(nmea.longitude().unwrap(), -(6. + 30.3372 / 60.));
}

#[test]
fn test_gga_south_east() {
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,E,1,8,1.03,61.7,M,55.2,M,,*79")
        .unwrap();
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
    use chrono::Timelike;
    let mut nmea = Nmea::new();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,E,1,8,1.03,61.7,M,55.2,M,,*79")
        .unwrap();
    assert_eq!(nmea.fix_timestamp().unwrap().second(), 50);
    assert_eq!(nmea.fix_timestamp().unwrap().minute(), 27);
    assert_eq!(nmea.fix_timestamp().unwrap().hour(), 9);
    assert_eq!(-(53. + 21.6802 / 60.), nmea.latitude.unwrap());
    assert_eq!(6. + 30.3372 / 60., nmea.longitude.unwrap());
    assert_eq!(nmea.fix_type(), Some(FixType::Gps));
    assert_eq!(8, nmea.num_of_fix_satellites.unwrap());
    assert_eq!(1.03, nmea.hdop.unwrap());
    assert_eq!(61.7, nmea.altitude.unwrap());
    assert_eq!(55.2, nmea.geoid_height.unwrap());
}

#[test]
fn test_gsv() {
    let mut nmea = Nmea::new();
    //                        10           07           05           08
    nmea.parse("$GPGSV,3,1,11,10,63,137,17,07,61,098,15,05,59,290,20,08,54,157,30*70")
        .unwrap();
    //                        02           13           26         04
    nmea.parse("$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79")
        .unwrap();
    //                        29           16         36
    nmea.parse("$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76")
        .unwrap();
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
    let real_data = ["$GPGSV,3,1,12,01,49,196,41,03,71,278,32,06,02,323,27,11,21,196,39*72",
                     "$GPGSV,3,2,12,14,39,063,33,17,21,292,30,19,20,310,31,22,82,181,36*73",
                     "$GPGSV,3,3,12,23,34,232,42,25,11,045,33,31,45,092,38,32,14,061,39*75",
                     "$GLGSV,3,1,10,74,40,078,43,66,23,275,31,82,10,347,36,73,15,015,38*6B",
                     "$GLGSV,3,2,10,75,19,135,36,65,76,333,31,88,32,233,33,81,40,302,38*6A",
                     "$GLGSV,3,3,10,72,40,075,43,87,00,000,*6F",

                     "$GPGSV,4,4,15,26,02,112,,31,45,071,,32,01,066,*4C"];
    for line in &real_data {
        assert_eq!(nmea.parse(line).unwrap(), SentenceType::GSV);
    }
}

#[test]
fn test_gsv_order() {
    let mut nmea = Nmea::new();
    //                         2           13           26         04
    nmea.parse("$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79")
        .unwrap();
    //                        29           16         36
    nmea.parse("$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76")
        .unwrap();
    //                        10           07           05           08
    nmea.parse("$GPGSV,3,1,11,10,63,137,17,07,61,098,15,05,59,290,20,08,54,157,30*70")
        .unwrap();
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
    nmea.parse("$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79")
        .unwrap();
    //                        29           16         36
    nmea.parse("$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76")
        .unwrap();
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
        let res = nmea.parse(s).unwrap();
        println!("test_parse res {:?}", res);
    }

    assert_eq!(nmea.latitude().unwrap(), 53. + 21.6802 / 60.);
    assert_eq!(nmea.longitude().unwrap(), -(6. + 30.3372 / 60.));
    assert_eq!(nmea.altitude().unwrap(), 61.7);
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::QuickCheck;
    use super::parse::checksum;

    fn check_parsing_lat_lon_in_gga(lat: f64, lon: f64) -> bool {
        let lat_min = (lat.abs() * 60.0) % 60.0;
        let lon_min = (lon.abs() * 60.0) % 60.0;
        let mut nmea = Nmea::new();
        let mut s = format!("$GPGGA,092750.000,{lat_deg:02}{lat_min:09.6},{lat_dir},\
                             {lon_deg:03}{lon_min:09.6},{lon_dir},1,8,1.03,61.7,M,55.2,M,,*",
                            lat_deg = lat.abs().floor() as u8, lon_deg = lon.abs().floor() as u8,
                            lat_min = lat_min, lon_min = lon_min,
                            lat_dir = if lat.is_sign_positive() { 'N' } else { 'S' },
                            lon_dir = if lon.is_sign_positive() { 'E' } else { 'W' },
        );
        let cs = checksum(s.as_bytes()[1..s.len() - 1].iter());
        s.push_str(&format!("{:02X}", cs));
        nmea.parse(&s).unwrap();
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

#[test]
fn test_parse_for_fix() {
    {
        let mut nmea = Nmea::create_for_navigation([SentenceType::RMC, SentenceType::GGA]
                                                       .iter()
                                                       .map(|v| v.clone())
                                                       .collect())
                .unwrap();
        let log = [("$GPRMC,123308.2,A,5521.76474,N,03731.92553,E,000.48,071.9,090317,010.2,E,A*3B",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 200))),
                   ("$GPGGA,123308.2,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*52",
                    FixType::Gps,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 200))),
                   ("$GPVTG,071.9,T,061.7,M,000.48,N,0000.88,K,A*10",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 200))),
                   ("$GPRMC,123308.3,A,5521.76474,N,03731.92553,E,000.51,071.9,090317,010.2,E,A*32",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 300))),
                   ("$GPGGA,123308.3,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*53",
                    FixType::Gps,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 300))),
                   ("$GPVTG,071.9,T,061.7,M,000.51,N,0000.94,K,A*15",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 300))),
                   ("$GPRMC,123308.4,A,5521.76474,N,03731.92553,E,000.54,071.9,090317,010.2,E,A*30",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 400))),
                   ("$GPGGA,123308.4,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*54",
                    FixType::Gps,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 400))),
                   ("$GPVTG,071.9,T,061.7,M,000.54,N,0001.00,K,A*1C",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 400))),
                   ("$GPRMC,123308.5,A,5521.76474,N,03731.92553,E,000.57,071.9,090317,010.2,E,A*32",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 500))),
                   ("$GPGGA,123308.5,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*55",
                    FixType::Gps,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 500))),
                   ("$GPVTG,071.9,T,061.7,M,000.57,N,0001.05,K,A*1A",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 500))),
                   ("$GPRMC,123308.6,A,5521.76474,N,03731.92553,E,000.58,071.9,090317,010.2,E,A*3E",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 600))),
                   ("$GPGGA,123308.6,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*56",
                    FixType::Gps,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 600))),
                   ("$GPVTG,071.9,T,061.7,M,000.58,N,0001.08,K,A*18",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 600))),
                   ("$GPRMC,123308.7,A,5521.76474,N,03731.92553,E,000.59,071.9,090317,010.2,E,A*3E",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 700))),
                   ("$GPGGA,123308.7,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*57",
                    FixType::Gps,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 700))),
                   ("$GPVTG,071.9,T,061.7,M,000.59,N,0001.09,K,A*18",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 700)))];

        for (i, item) in log.iter().enumerate() {
            let res = nmea.parse_for_fix(item.0.as_bytes()).unwrap();
            println!("parse result({}): {:?}, {:?}", i, res, nmea.fix_time);
            assert_eq!((&res, &nmea.fix_time), (&item.1, &item.2));
        }
    }

    {
        let mut nmea = Nmea::create_for_navigation([SentenceType::RMC, SentenceType::GGA]
                                                       .iter()
                                                       .map(|v| v.clone())
                                                       .collect())
                .unwrap();
        let log = [("$GPRMC,123308.2,A,5521.76474,N,03731.92553,E,000.48,071.9,090317,010.2,E,A*3B",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 200))),
                   ("$GPRMC,123308.3,A,5521.76474,N,03731.92553,E,000.51,071.9,090317,010.2,E,A*32",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 300))),
                   ("$GPGGA,123308.3,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*53",
                    FixType::Gps,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 300)))];

        for (i, item) in log.iter().enumerate() {
            let res = nmea.parse_for_fix(item.0.as_bytes()).unwrap();
            println!("parse result({}): {:?}, {:?}", i, res, nmea.fix_time);
            assert_eq!((&res, &nmea.fix_time), (&item.1, &item.2));
        }
    }
}

#[test]
fn test_some_reciever() {
    let lines = ["$GPRMC,171724.000,A,6847.2474,N,03245.8351,E,0.26,140.74,250317,,*02",
                 "$GPGGA,171725.000,6847.2473,N,03245.8351,E,1,08,1.0,87.7,M,18.5,M,,0000*66",
                 "$GPGSA,A,3,02,25,29,12,31,06,23,14,,,,,2.0,1.0,1.7*3A",
                 "$GPRMC,171725.000,A,6847.2473,N,03245.8351,E,0.15,136.12,250317,,*05",
                 "$GPGGA,171726.000,6847.2473,N,03245.8352,E,1,08,1.0,87.8,M,18.5,M,,0000*69",
                 "$GPGSA,A,3,02,25,29,12,31,06,23,14,,,,,2.0,1.0,1.7*3A",
                 "$GPRMC,171726.000,A,6847.2473,N,03245.8352,E,0.16,103.49,250317,,*0E",
                 "$GPGGA,171727.000,6847.2474,N,03245.8353,E,1,08,1.0,87.9,M,18.5,M,,0000*6F",
                 "$GPGSA,A,3,02,25,29,12,31,06,23,14,,,,,2.0,1.0,1.7*3A",
                 "$GPRMC,171727.000,A,6847.2474,N,03245.8353,E,0.49,42.80,250317,,*32"];
    let mut nmea = Nmea::create_for_navigation([SentenceType::RMC, SentenceType::GGA]
                                                   .iter()
                                                   .map(|v| v.clone())
                                                   .collect())
            .unwrap();
    println!("start test");
    let mut nfixes = 0_usize;
    for line in &lines {
        match nmea.parse_for_fix(line.as_bytes()) {
            Ok(FixType::Invalid) => {
                println!("invalid");
                continue;
            }
            Err(msg) => {
                println!("update_gnss_info_nmea: parse_for_fix failed: {}", msg);
                continue;
            }
            Ok(_) => nfixes += 1,
        }
    }
    assert_eq!(nfixes, 3);
}
