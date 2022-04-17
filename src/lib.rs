//! NMEA 0183 parser
//!
//! Use nmea::Nmea::parse and nmea::Nmea::parse_for_fix to preserve
//! state between recieving new nmea sentence, and nmea::parse
//! to parse sentences without state
//!
//! Units that used every where: degrees, knots, meters for altitude
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

mod parse;
mod sentences;

pub use crate::parse::{
    parse, BwcData, GgaData, GllData, GsaData, GsvData, NmeaError, ParseResult, PosSystemIndicator,
    RmcData, RmcStatusOfFix, TxtData, VtgData, SENTENCE_MAX_LEN,
};
use chrono::{NaiveDate, NaiveTime};
use core::{fmt, iter::Iterator, mem, ops::BitOr};
use std::collections::HashMap;

/// NMEA parser
/// This struct parses NMEA sentences, including checksum checks and sentence
/// validation.
///
/// # Examples
///
/// ```
/// use nmea::Nmea;
///
/// let mut nmea= Nmea::default();
/// let gga = "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76";
/// nmea.parse(gga).unwrap();
/// println!("{}", nmea);
/// ```
#[derive(Debug, Clone)]
pub struct Nmea {
    pub fix_time: Option<NaiveTime>,
    pub fix_date: Option<NaiveDate>,
    pub fix_type: Option<FixType>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    /// MSL Altitude in meters
    pub altitude: Option<f32>,
    pub speed_over_ground: Option<f32>,
    pub true_course: Option<f32>,
    pub num_of_fix_satellites: Option<u32>,
    pub hdop: Option<f32>,
    pub vdop: Option<f32>,
    pub pdop: Option<f32>,
    /// Geoid separation in meters
    pub geoid_separation: Option<f32>,
    pub satellites: Vec<Satellite>,
    pub fix_satellites_prns: Option<Vec<u32>>,
    satellites_scan: HashMap<GnssType, Vec<Vec<Satellite>>>,
    required_sentences_for_nav: SentenceMask,
    last_fix_time: Option<NaiveTime>,
    last_txt: Option<TxtData>,
    sentences_for_this_time: SentenceMask,
}

impl<'a> Default for Nmea {
    fn default() -> Self {
        let mut n = Self {
            fix_time: None,
            fix_date: None,
            fix_type: None,
            latitude: None,
            longitude: None,
            altitude: None,
            speed_over_ground: None,
            true_course: None,
            num_of_fix_satellites: None,
            hdop: None,
            vdop: None,
            pdop: None,
            geoid_separation: None,
            satellites: Vec::new(),
            fix_satellites_prns: None,
            satellites_scan: HashMap::with_capacity(4),
            required_sentences_for_nav: SentenceMask::default(),
            last_fix_time: None,
            last_txt: None,
            sentences_for_this_time: SentenceMask::default(),
        };
        for gnss_type in [
            GnssType::Galileo,
            GnssType::Gps,
            GnssType::Glonass,
            GnssType::Beidou,
        ] {
            n.satellites_scan.insert(gnss_type, vec![]);
        }
        n
    }
}

impl<'a> Nmea {
    /// Constructs a new `Nmea` for navigation purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// use nmea::{Nmea, SentenceType};
    ///
    /// let mut nmea = Nmea::create_for_navigation(&[SentenceType::RMC,
    /// SentenceType::GGA]).unwrap();
    /// let gga = "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76";
    /// nmea.parse(gga).unwrap();
    /// println!("{}", nmea);
    /// ```
    pub fn create_for_navigation(
        required_sentences_for_nav: &[SentenceType],
    ) -> Result<Nmea, NmeaError<'a>> {
        if required_sentences_for_nav.is_empty() {
            return Err(NmeaError::EmptyNavConfig);
        }
        let mut n = Self::default();
        for sentence in required_sentences_for_nav.iter() {
            n.required_sentences_for_nav.insert(*sentence);
        }
        Ok(n)
    }

    /// Returns fix type
    pub fn fix_timestamp(&self) -> Option<NaiveTime> {
        self.fix_time
    }

    /// Returns fix type
    pub fn fix_type(&self) -> Option<FixType> {
        self.fix_type
    }

    /// Returns last fixed latitude in degress. None if not fixed.
    pub fn latitude(&self) -> Option<f64> {
        self.latitude
    }

    /// Returns last fixed longitude in degress. None if not fixed.
    pub fn longitude(&self) -> Option<f64> {
        self.longitude
    }

    /// Returns altitude above WGS-84 ellipsoid, meters.
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

    /// Returns the altitude above MSL (geoid), meters.
    pub fn geoid_altitude(&self) -> Option<f32> {
        match (self.altitude, self.geoid_separation) {
            (Some(alt), Some(geoid_diff)) => Some(alt + geoid_diff),
            _ => None,
        }
    }

    /// Returns the height of geoid above WGS84
    pub fn satellites(&self) -> Vec<Satellite> {
        self.satellites.clone()
    }

    fn merge_gga_data(&mut self, gga_data: GgaData) {
        self.fix_time = gga_data.fix_time;
        self.latitude = gga_data.latitude;
        self.longitude = gga_data.longitude;
        self.fix_type = gga_data.fix_type;
        self.num_of_fix_satellites = gga_data.fix_satellites;
        self.hdop = gga_data.hdop;
        self.altitude = gga_data.altitude;
        self.geoid_separation = gga_data.geoid_separation;
    }

    fn merge_gsv_data(&mut self, data: GsvData) -> Result<(), NmeaError<'a>> {
        {
            let d = self
                .satellites_scan
                .get_mut(&data.gnss_type)
                .ok_or(NmeaError::InvalidGnssType)?;
            // Adjust size to this scan
            d.resize(data.number_of_sentences as usize, vec![]);
            // Replace data at index with new scan data
            d.push(
                data.sats_info
                    .iter()
                    .filter(|v| v.is_some())
                    .map(|v| v.clone().unwrap())
                    .collect(),
            );
            d.swap_remove(data.sentence_num as usize - 1);
        }
        self.satellites.clear();
        for v in self.satellites_scan.values() {
            for v1 in v {
                for v2 in v1 {
                    self.satellites.push(v2.clone());
                }
            }
        }

        Ok(())
    }

    fn merge_rmc_data(&mut self, rmc_data: RmcData) {
        self.fix_time = rmc_data.fix_time;
        self.fix_date = rmc_data.fix_date;
        self.fix_type = rmc_data.status_of_fix.map(|v| match v {
            RmcStatusOfFix::Autonomous => FixType::Gps,
            RmcStatusOfFix::Differential => FixType::DGps,
            RmcStatusOfFix::Invalid => FixType::Invalid,
        });
        self.latitude = rmc_data.lat;
        self.longitude = rmc_data.lon;
        self.speed_over_ground = rmc_data.speed_over_ground;
        self.true_course = rmc_data.true_course;
    }

    fn merge_gns_data(&mut self, gns_data: parse::GnsData) {
        self.fix_time = gns_data.fix_time;
        self.fix_type = Some(gns_data.faa_modes.into());
        self.latitude = gns_data.lat;
        self.longitude = gns_data.lon;
        self.altitude = gns_data.alt;
        self.hdop = gns_data.hdop;
        self.geoid_separation = gns_data.geoid_separation;
    }

    fn merge_gsa_data(&mut self, gsa: GsaData) {
        self.fix_satellites_prns = Some(gsa.fix_sats_prn);
        self.hdop = gsa.hdop;
        self.vdop = gsa.vdop;
        self.pdop = gsa.pdop;
    }

    fn merge_vtg_data(&mut self, vtg: VtgData) {
        self.speed_over_ground = vtg.speed_over_ground;
        self.true_course = vtg.true_course;
    }

    fn merge_gll_data(&mut self, gll: GllData) {
        self.latitude = gll.latitude;
        self.longitude = gll.longitude;
        self.fix_time = Some(gll.fix_time);
    }

    fn merge_txt_data(&mut self, txt: TxtData) {
        self.last_txt = Some(txt);
    }

    /// Parse any NMEA sentence and stores the result. The type of sentence
    /// is returnd if implemented and valid.
    pub fn parse(&mut self, s: &'a str) -> Result<SentenceType, NmeaError<'a>> {
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
            ParseResult::GNS(gns) => {
                self.merge_gns_data(gns);
                Ok(SentenceType::GNS)
            }
            ParseResult::GSA(gsa) => {
                self.merge_gsa_data(gsa);
                Ok(SentenceType::GSA)
            }
            ParseResult::GLL(gll) => {
                self.merge_gll_data(gll);
                Ok(SentenceType::GLL)
            }
            ParseResult::TXT(txt) => {
                self.merge_txt_data(txt);
                Ok(SentenceType::TXT)
            }
            ParseResult::BWC(_) => Err(NmeaError::Unsupported(SentenceType::BWC)),
            ParseResult::Unsupported(sentence_type) => Err(NmeaError::Unsupported(sentence_type)),
        }
    }

    fn new_tick(&mut self) {
        let old = mem::take(self);
        self.satellites_scan = old.satellites_scan;
        self.satellites = old.satellites;
        self.required_sentences_for_nav = old.required_sentences_for_nav;
        self.last_fix_time = old.last_fix_time;
    }

    fn clear_position_info(&mut self) {
        self.last_fix_time = None;
        self.new_tick();
    }

    pub fn parse_for_fix(&mut self, xs: &'a [u8]) -> Result<FixType, NmeaError<'a>> {
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
                if self.required_sentences_for_nav.contains(&SentenceType::VTG) {
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
                    Some(RmcStatusOfFix::Invalid) | None => {
                        self.clear_position_info();
                        return Ok(FixType::Invalid);
                    }
                    _ => { /*nothing*/ }
                }
                match (self.last_fix_time, rmc_data.fix_time) {
                    (Some(ref last_fix_time), Some(ref rmc_fix_time)) => {
                        if *last_fix_time != *rmc_fix_time {
                            self.new_tick();
                            self.last_fix_time = Some(*rmc_fix_time);
                        }
                    }
                    (None, Some(ref rmc_fix_time)) => self.last_fix_time = Some(*rmc_fix_time),
                    (Some(_), None) | (None, None) => {
                        self.clear_position_info();
                        return Ok(FixType::Invalid);
                    }
                }
                self.merge_rmc_data(rmc_data);
                self.sentences_for_this_time.insert(SentenceType::RMC);
            }
            ParseResult::GNS(gns_data) => {
                let fix_type: FixType = gns_data.faa_modes.into();
                if !fix_type.is_valid() {
                    self.clear_position_info();
                    return Ok(FixType::Invalid);
                }
                match (self.last_fix_time, gns_data.fix_time) {
                    (Some(ref last_fix_time), Some(ref gns_fix_time)) => {
                        if *last_fix_time != *gns_fix_time {
                            self.new_tick();
                            self.last_fix_time = Some(*gns_fix_time);
                        }
                    }
                    (None, Some(ref gns_fix_time)) => self.last_fix_time = Some(*gns_fix_time),
                    (Some(_), None) | (None, None) => {
                        self.clear_position_info();
                        return Ok(FixType::Invalid);
                    }
                }
                self.merge_gns_data(gns_data);
                self.sentences_for_this_time.insert(SentenceType::GNS);
            }
            ParseResult::GGA(gga_data) => {
                match gga_data.fix_type {
                    Some(FixType::Invalid) | None => {
                        self.clear_position_info();
                        return Ok(FixType::Invalid);
                    }
                    _ => { /*nothing*/ }
                }
                match (self.last_fix_time, gga_data.fix_time) {
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
            ParseResult::GLL(gll_data) => {
                self.merge_gll_data(gll_data);
                return Ok(FixType::Invalid);
            }
            ParseResult::TXT(txt_data) => {
                self.merge_txt_data(txt_data);
                return Ok(FixType::Invalid);
            }
            ParseResult::BWC(_) => return Ok(FixType::Invalid),
            ParseResult::Unsupported(_) => {
                return Ok(FixType::Invalid);
            }
        }
        match self.fix_type {
            Some(FixType::Invalid) | None => Ok(FixType::Invalid),
            Some(ref fix_type)
                if self
                    .required_sentences_for_nav
                    .is_subset(&self.sentences_for_this_time) =>
            {
                Ok(*fix_type)
            }
            _ => Ok(FixType::Invalid),
        }
    }

    pub fn last_txt(&self) -> Option<&TxtData> {
        self.last_txt.as_ref()
    }
}

impl fmt::Display for Nmea {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}: lat: {} lon: {} alt: {} {:?}",
            self.fix_time
                .map(|l| format!("{:?}", l))
                .unwrap_or_else(|| "None".to_owned()),
            self.latitude
                .map(|l| format!("{:3.8}", l))
                .unwrap_or_else(|| "None".to_owned()),
            self.longitude
                .map(|l| format!("{:3.8}", l))
                .unwrap_or_else(|| "None".to_owned()),
            self.altitude
                .map(|l| format!("{:.3}", l))
                .unwrap_or_else(|| "None".to_owned()),
            self.satellites()
        )
    }
}

#[derive(Clone, PartialEq)]
/// Satellite information
pub struct Satellite {
    gnss_type: GnssType,
    prn: u32,
    elevation: Option<f32>,
    azimuth: Option<f32>,
    snr: Option<f32>,
}

impl Satellite {
    pub fn gnss_type(&self) -> GnssType {
        self.gnss_type
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
        write!(
            f,
            "{}: {} elv: {} ath: {} snr: {}",
            self.gnss_type,
            self.prn,
            self.elevation
                .map(|e| format!("{}", e))
                .unwrap_or_else(|| "--".to_owned()),
            self.azimuth
                .map(|e| format!("{}", e))
                .unwrap_or_else(|| "--".to_owned()),
            self.snr
                .map(|e| format!("{}", e))
                .unwrap_or_else(|| "--".to_owned())
        )
    }
}

impl fmt::Debug for Satellite {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[{:?},{:?},{:?},{:?},{:?}]",
            self.gnss_type, self.prn, self.elevation, self.azimuth, self.snr
        )
    }
}

macro_rules! define_sentence_type_enum {
    (
        $(#[$outer:meta])*
        enum $Name:ident { $($Variant:ident),* $(,)* }
    ) => {
        $(#[$outer])*
        #[derive(PartialEq, Debug, Hash, Eq, Clone, Copy)]
        #[repr(C)]
        pub enum $Name {
            $($Variant),*,
            None
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
            fn from_slice(s: &[u8]) -> Self {
                $(
                    #[allow(nonstandard_style)]
                    const $Variant: &[u8] = stringify!($Variant).as_bytes();
                )*
                match s {
                    $($Variant => $Name::$Variant,)*
                    _ => $Name::None,
                }
            }

            fn to_mask_value(&self) -> u128 {
                1 << *self as u32
            }
        }
    }
}

define_sentence_type_enum!(
    /// NMEA sentence type
    /// General: OSD |
    /// Autopilot: APA | APB | ASD |
    /// Decca: DCN |
    /// D-GPS: MSK
    /// Echo: DBK | DBS | DBT |
    /// Radio: FSI | SFI | TLL
    /// Speed: VBW | VHW | VLW |
    /// GPS: ALM | GBS | GGA | GNS | GSA | GSV |
    /// Course: DPT | HDG | HDM | HDT | HSC | ROT | VDR |
    /// Loran-C: GLC | LCD |
    /// Machine: RPM |
    /// Navigation: RMA | RMB | RMC |
    /// Omega: OLN |
    /// Position: GLL | DTM
    /// Radar: RSD | TLL | TTM |
    /// Rudder: RSA |
    /// Temperature: MTW |
    /// Transit: GXA | RTF |
    /// Waypoints and tacks: AAM | BEC | BOD | BWC | BWR | BWW | ROO | RTE |
    ///                      VTG | WCV | WNC | WPL | XDR | XTE | XTR |
    /// Wind: MWV | VPW | VWR |
    /// Date and Time: GDT | ZDA | ZFO | ZTG |
    enum SentenceType {
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
);

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct SentenceMask {
    mask: u128,
}

impl SentenceMask {
    fn contains(&self, sentence_type: &SentenceType) -> bool {
        sentence_type.to_mask_value() & self.mask != 0
    }

    fn is_subset(&self, mask: &Self) -> bool {
        (mask.mask | self.mask) == mask.mask
    }

    fn insert(&mut self, sentence_type: SentenceType) {
        self.mask |= sentence_type.to_mask_value()
    }
}

impl BitOr for SentenceType {
    type Output = SentenceMask;
    fn bitor(self, rhs: Self) -> Self::Output {
        SentenceMask {
            mask: self.to_mask_value() | rhs.to_mask_value(),
        }
    }
}

impl BitOr<SentenceType> for SentenceMask {
    type Output = Self;
    fn bitor(self, rhs: SentenceType) -> Self {
        SentenceMask {
            mask: self.mask | rhs.to_mask_value(),
        }
    }
}

/// Fix type
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum FixType {
    Invalid,
    Gps,
    DGps,
    /// Precise Position Service
    Pps,
    Rtk,
    FloatRtk,
    Estimated,
    Manual,
    Simulation,
}

impl FixType {
    #[inline]
    pub fn is_valid(self) -> bool {
        match self {
            FixType::Simulation | FixType::Manual | FixType::Estimated | FixType::Invalid => false,
            FixType::DGps | FixType::Gps | FixType::Rtk | FixType::FloatRtk | FixType::Pps => true,
        }
    }
}

/// GNSS type
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum GnssType {
    Beidou,
    Galileo,
    Gps,
    Glonass,
}

impl fmt::Display for GnssType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GnssType::Beidou => write!(f, "Beidou"),
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

#[cfg(test)]
mod tests {
    use super::parse::checksum;
    use super::*;
    use approx::assert_relative_eq;
    use quickcheck::{QuickCheck, TestResult};

    fn check_parsing_lat_lon_in_gga(lat: f64, lon: f64) -> TestResult {
        fn scale(val: f64, max: f64) -> f64 {
            val % max
        }
        if !lat.is_finite() || !lon.is_finite() {
            return TestResult::discard();
        }
        let lat = scale(lat, 90.0);
        let lon = scale(lon, 180.0);
        let lat_min = (lat.abs() * 60.0) % 60.0;
        let lon_min = (lon.abs() * 60.0) % 60.0;
        let mut nmea = Nmea::default();
        let mut s = format!(
            "$GPGGA,092750.000,{lat_deg:02}{lat_min:09.6},{lat_dir},\
             {lon_deg:03}{lon_min:09.6},{lon_dir},1,8,1.03,61.7,M,55.2,M,,*",
            lat_deg = lat.abs().floor() as u8,
            lon_deg = lon.abs().floor() as u8,
            lat_min = lat_min,
            lon_min = lon_min,
            lat_dir = if lat.is_sign_positive() { 'N' } else { 'S' },
            lon_dir = if lon.is_sign_positive() { 'E' } else { 'W' },
        );
        let cs = checksum(s.as_bytes()[1..s.len() - 1].iter());
        s.push_str(&format!("{:02X}", cs));
        nmea.parse(&s).unwrap();
        let (new_lat, new_lon) = (nmea.latitude.unwrap(), nmea.longitude.unwrap());
        const MAX_COOR_DIFF: f64 = 1e-7;
        TestResult::from_bool(
            (new_lat - lat).abs() < MAX_COOR_DIFF && (new_lon - lon).abs() < MAX_COOR_DIFF,
        )
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
        use crate::parse::checksum;
        let valid = "$GNGSA,A,1,,,,,,,,,,,,,99.99,99.99,99.99*2E";
        let invalid = "$GNZDA,165118.00,13,05,2016,00,00*71";
        assert_eq!(
            checksum((&valid[1..valid.len() - 3]).as_bytes().iter()),
            0x2E
        );
        assert_ne!(
            checksum((&invalid[1..invalid.len() - 3]).as_bytes().iter()),
            0x71
        );
    }

    #[test]
    fn test_message_type() {
        assert_eq!(SentenceType::from_slice(b"GGA"), SentenceType::GGA);
        assert_eq!(SentenceType::from_slice(b"XXX"), SentenceType::None);
    }

    #[test]
    fn test_gga_north_west() {
        use chrono::Timelike;
        let mut nmea = Nmea::default();
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
        assert_relative_eq!(nmea.geoid_altitude().unwrap(), (61.7 + 55.2));
    }

    #[test]
    fn test_gga_north_east() {
        let mut nmea = Nmea::default();
        nmea.parse("$GPGGA,092750.000,5321.6802,N,00630.3372,E,1,8,1.03,61.7,M,55.2,M,,*64")
            .unwrap();
        assert_eq!(nmea.latitude().unwrap(), 53. + 21.6802 / 60.);
        assert_eq!(nmea.longitude().unwrap(), 6. + 30.3372 / 60.);
    }

    #[test]
    fn test_gga_south_west() {
        let mut nmea = Nmea::default();
        nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*6B")
            .unwrap();
        assert_eq!(nmea.latitude().unwrap(), -(53. + 21.6802 / 60.));
        assert_eq!(nmea.longitude().unwrap(), -(6. + 30.3372 / 60.));
    }

    #[test]
    fn test_gga_south_east() {
        let mut nmea = Nmea::default();
        nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,E,1,8,1.03,61.7,M,55.2,M,,*79")
            .unwrap();
        assert_eq!(nmea.latitude().unwrap(), -(53. + 21.6802 / 60.));
        assert_eq!(nmea.longitude().unwrap(), 6. + 30.3372 / 60.);
    }

    #[test]
    fn test_gga_invalid() {
        let mut nmea = Nmea::default();
        nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,E,0,8,1.03,61.7,M,55.2,M,,*7B")
            .unwrap_err();
        assert_eq!(nmea.fix_type(), None);
    }

    #[test]
    fn test_gga_gps() {
        use chrono::Timelike;
        let mut nmea = Nmea::default();
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
        assert_eq!(55.2, nmea.geoid_separation.unwrap());
    }

    #[test]
    fn test_gsv() {
        let mut nmea = Nmea::default();
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
        let mut nmea = Nmea::default();
        let real_data = [
            "$GPGSV,3,1,12,01,49,196,41,03,71,278,32,06,02,323,27,11,21,196,39*72",
            "$GPGSV,3,2,12,14,39,063,33,17,21,292,30,19,20,310,31,22,82,181,36*73",
            "$GPGSV,3,3,12,23,34,232,42,25,11,045,33,31,45,092,38,32,14,061,39*75",
            "$GLGSV,3,1,10,74,40,078,43,66,23,275,31,82,10,347,36,73,15,015,38*6B",
            "$GLGSV,3,2,10,75,19,135,36,65,76,333,31,88,32,233,33,81,40,302,38*6A",
            "$GLGSV,3,3,10,72,40,075,43,87,00,000,*6F",
            "$GPGSV,4,4,15,26,02,112,,31,45,071,,32,01,066,*4C",
        ];
        for line in &real_data {
            assert_eq!(nmea.parse(line).unwrap(), SentenceType::GSV);
        }
    }

    #[test]
    fn test_gsv_order() {
        let mut nmea = Nmea::default();
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
        let mut nmea = Nmea::default();
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
        let sentences = [
            "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76",
            "$GPGSA,A,3,10,07,05,02,29,04,08,13,,,,,1.72,1.03,1.38*0A",
            "$GPGSV,3,1,11,10,63,137,17,07,61,098,15,05,59,290,20,08,54,157,30*70",
            "$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79",
            "$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76",
            "$GPRMC,092750.000,A,5321.6802,N,00630.3372,W,0.02,31.66,280511,,,A*43",
        ];

        let mut nmea = Nmea::default();
        for s in &sentences {
            let res = nmea.parse(s).unwrap();
            println!("test_parse res {:?}", res);
        }

        assert_eq!(nmea.latitude().unwrap(), 53. + 21.6802 / 60.);
        assert_eq!(nmea.longitude().unwrap(), -(6. + 30.3372 / 60.));
        assert_eq!(nmea.altitude().unwrap(), 61.7);
    }

    #[test]
    fn test_parsing_lat_lon_in_gga() {
        // regressions found by quickcheck,
        // explicit because of quickcheck use random gen
        assert!(!check_parsing_lat_lon_in_gga(0., 57.89528).is_failure());
        assert!(!check_parsing_lat_lon_in_gga(0., -43.33031).is_failure());
        QuickCheck::new()
            .tests(10_000_000_000)
            .quickcheck(check_parsing_lat_lon_in_gga as fn(f64, f64) -> TestResult);
    }

    #[test]
    fn test_parse_for_fix() {
        {
            let mut nmea =
                Nmea::create_for_navigation(&[SentenceType::RMC, SentenceType::GGA]).unwrap();
            let log = [
                (
                    "$GPRMC,123308.2,A,5521.76474,N,03731.92553,E,000.48,071.9,090317,010.2,E,A*3B",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 200)),
                ),
                (
                    "$GPGGA,123308.2,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*52",
                    FixType::Gps,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 200)),
                ),
                (
                    "$GPVTG,071.9,T,061.7,M,000.48,N,0000.88,K,A*10",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 200)),
                ),
                (
                    "$GPRMC,123308.3,A,5521.76474,N,03731.92553,E,000.51,071.9,090317,010.2,E,A*32",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 300)),
                ),
                (
                    "$GPGGA,123308.3,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*53",
                    FixType::Gps,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 300)),
                ),
                (
                    "$GPVTG,071.9,T,061.7,M,000.51,N,0000.94,K,A*15",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 300)),
                ),
                (
                    "$GPRMC,123308.4,A,5521.76474,N,03731.92553,E,000.54,071.9,090317,010.2,E,A*30",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 400)),
                ),
                (
                    "$GPGGA,123308.4,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*54",
                    FixType::Gps,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 400)),
                ),
                (
                    "$GPVTG,071.9,T,061.7,M,000.54,N,0001.00,K,A*1C",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 400)),
                ),
                (
                    "$GPRMC,123308.5,A,5521.76474,N,03731.92553,E,000.57,071.9,090317,010.2,E,A*32",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 500)),
                ),
                (
                    "$GPGGA,123308.5,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*55",
                    FixType::Gps,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 500)),
                ),
                (
                    "$GPVTG,071.9,T,061.7,M,000.57,N,0001.05,K,A*1A",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 500)),
                ),
                (
                    "$GPRMC,123308.6,A,5521.76474,N,03731.92553,E,000.58,071.9,090317,010.2,E,A*3E",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 600)),
                ),
                (
                    "$GPGGA,123308.6,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*56",
                    FixType::Gps,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 600)),
                ),
                (
                    "$GPVTG,071.9,T,061.7,M,000.58,N,0001.08,K,A*18",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 600)),
                ),
                (
                    "$GPRMC,123308.7,A,5521.76474,N,03731.92553,E,000.59,071.9,090317,010.2,E,A*3E",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 700)),
                ),
                (
                    "$GPGGA,123308.7,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*57",
                    FixType::Gps,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 700)),
                ),
                (
                    "$GPVTG,071.9,T,061.7,M,000.59,N,0001.09,K,A*18",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 700)),
                ),
            ];

            for (i, item) in log.iter().enumerate() {
                let res = nmea.parse_for_fix(item.0.as_bytes()).unwrap();
                println!("parse result({}): {:?}, {:?}", i, res, nmea.fix_time);
                assert_eq!((&res, &nmea.fix_time), (&item.1, &item.2));
            }
        }

        {
            let mut nmea =
                Nmea::create_for_navigation(&[SentenceType::RMC, SentenceType::GGA]).unwrap();
            let log = [
                (
                    "$GPRMC,123308.2,A,5521.76474,N,03731.92553,E,000.48,071.9,090317,010.2,E,A*3B",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 200)),
                ),
                (
                    "$GPRMC,123308.3,A,5521.76474,N,03731.92553,E,000.51,071.9,090317,010.2,E,A*32",
                    FixType::Invalid,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 300)),
                ),
                (
                    "$GPGGA,123308.3,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*53",
                    FixType::Gps,
                    Some(NaiveTime::from_hms_milli(12, 33, 8, 300)),
                ),
            ];

            for (i, item) in log.iter().enumerate() {
                let res = nmea.parse_for_fix(item.0.as_bytes()).unwrap();
                println!("parse result({}): {:?}, {:?}", i, res, nmea.fix_time);
                assert_eq!((&res, &nmea.fix_time), (&item.1, &item.2));
            }
        }
    }

    #[test]
    fn test_some_reciever() {
        let lines = [
            "$GPRMC,171724.000,A,6847.2474,N,03245.8351,E,0.26,140.74,250317,,*02",
            "$GPGGA,171725.000,6847.2473,N,03245.8351,E,1,08,1.0,87.7,M,18.5,M,,0000*66",
            "$GPGSA,A,3,02,25,29,12,31,06,23,14,,,,,2.0,1.0,1.7*3A",
            "$GPRMC,171725.000,A,6847.2473,N,03245.8351,E,0.15,136.12,250317,,*05",
            "$GPGGA,171726.000,6847.2473,N,03245.8352,E,1,08,1.0,87.8,M,18.5,M,,0000*69",
            "$GPGSA,A,3,02,25,29,12,31,06,23,14,,,,,2.0,1.0,1.7*3A",
            "$GPRMC,171726.000,A,6847.2473,N,03245.8352,E,0.16,103.49,250317,,*0E",
            "$GPGGA,171727.000,6847.2474,N,03245.8353,E,1,08,1.0,87.9,M,18.5,M,,0000*6F",
            "$GPGSA,A,3,02,25,29,12,31,06,23,14,,,,,2.0,1.0,1.7*3A",
            "$GPRMC,171727.000,A,6847.2474,N,03245.8353,E,0.49,42.80,250317,,*32",
        ];
        let mut nmea =
            Nmea::create_for_navigation(&[SentenceType::RMC, SentenceType::GGA]).unwrap();
        println!("start test");
        let mut nfixes = 0_usize;
        for line in &lines {
            match nmea.parse_for_fix(line.as_bytes()) {
                Ok(FixType::Invalid) => {
                    println!("invalid");
                    continue;
                }
                Err(msg) => {
                    println!("update_gnss_info_nmea: parse_for_fix failed: {:?}", msg);
                    continue;
                }
                Ok(_) => nfixes += 1,
            }
        }
        assert_eq!(nfixes, 3);
    }
    #[test]
    fn test_sentence_type_enum() {
        // So we don't trip over the max value of u128 when shifting it with
        // SentenceType as u32
        assert!((SentenceType::None as u32) < 127);
    }

    #[test]
    fn test_gll() {
        use chrono::Timelike;
        let mut nmea = Nmea::default();

        // Example from https://docs.novatel.com/OEM7/Content/Logs/GPGLL.htm
        nmea.parse("$GPGLL,5107.0013414,N,11402.3279144,W,205412.00,A,A*73")
            .unwrap();
        assert_eq!(51. + 7.0013414 / 60., nmea.latitude().unwrap());
        assert_eq!(-(114. + 2.3279144 / 60.), nmea.longitude().unwrap());
        assert_eq!(20, nmea.fix_timestamp().unwrap().hour());
        assert_eq!(54, nmea.fix_timestamp().unwrap().minute());
        assert_eq!(12, nmea.fix_timestamp().unwrap().second());

        // Example from https://www.gpsinformation.org/dale/nmea.htm#GLL
        nmea.parse("$GPGLL,4916.45,N,12311.12,W,225444,A,*1D")
            .unwrap();
        assert_eq!(49. + 16.45 / 60., nmea.latitude().unwrap());
        assert_eq!(-(123. + 11.12 / 60.), nmea.longitude().unwrap());
        assert_eq!(22, nmea.fix_timestamp().unwrap().hour());
        assert_eq!(54, nmea.fix_timestamp().unwrap().minute());
        assert_eq!(44, nmea.fix_timestamp().unwrap().second());
    }
}
