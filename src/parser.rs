//! The [`Nmea`] parser.

use core::{fmt, mem, ops::BitOr};

use chrono::{NaiveDate, NaiveTime};
use heapless::{Deque, Vec};

use crate::{parse_str, sentences::*, Error, ParseResult};

/// NMEA parser
///
/// This struct parses NMEA sentences, including checksum checks and sentence
/// validation.
///
/// # Examples
///
/// ```
/// use nmea::Nmea;
///
/// let mut nmea = Nmea::default();
/// let gga = "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76";
///
/// nmea.parse(gga).unwrap();
/// println!("{}", nmea);
/// ```
#[derive(Debug, Clone, Default)]
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
    pub fix_satellites_prns: Option<Vec<u32, 12>>,
    satellites_scan: [SatsPack; GnssType::COUNT],
    required_sentences_for_nav: SentenceMask,
    last_fix_time: Option<NaiveTime>,
    last_txt: Option<TxtData>,
    sentences_for_this_time: SentenceMask,
}

impl<'a> Nmea {
    /// Constructs a new `Nmea` for navigation purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// use nmea::{Nmea, SentenceType};
    ///
    /// let mut nmea = Nmea::create_for_navigation(&[SentenceType::RMC, SentenceType::GGA]).unwrap();
    /// let gga = "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76";
    /// nmea.parse(gga).unwrap();
    /// println!("{}", nmea);
    /// ```
    pub fn create_for_navigation(
        required_sentences_for_nav: &[SentenceType],
    ) -> Result<Nmea, Error<'a>> {
        if required_sentences_for_nav.is_empty() {
            return Err(Error::EmptyNavConfig);
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

    /// Returns last fixed latitude in degrees. None if not fixed.
    pub fn latitude(&self) -> Option<f64> {
        self.latitude
    }

    /// Returns last fixed longitude in degrees. None if not fixed.
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

    /// Returns used satellites
    pub fn satellites(&self) -> Vec<Satellite, 58> {
        let mut ret = Vec::<Satellite, 58>::new();
        let sat_key = |sat: &Satellite| (sat.gnss_type() as u8, sat.prn());
        for sns in &self.satellites_scan {
            // for sat_pack in sns.data.iter().rev() {
            for sat_pack in sns.data.iter().rev().flatten() {
                for sat in sat_pack.iter() {
                    match ret.binary_search_by_key(&sat_key(sat), sat_key) {
                        //already set
                        Ok(_pos) => {}
                        Err(pos) => ret.insert(pos, sat.clone()).unwrap(),
                    }
                }
            }
        }
        ret
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

    fn merge_gsv_data(&mut self, data: GsvData) -> Result<(), Error<'a>> {
        {
            let d = &mut self.satellites_scan[data.gnss_type as usize];
            let full_pack_size: usize = data
                .sentence_num
                .try_into()
                .map_err(|_| Error::InvalidGsvSentenceNum)?;
            d.max_len = full_pack_size.max(d.max_len);
            d.data
                .push_back(data.sats_info)
                .expect("Should not get the more than expected number of satellites");
            if d.data.len() > d.max_len {
                d.data.pop_front();
            }
        }

        Ok(())
    }

    fn merge_rmc_data(&mut self, rmc_data: RmcData) {
        self.fix_time = rmc_data.fix_time;
        self.fix_date = rmc_data.fix_date;
        self.fix_type = Some(match rmc_data.status_of_fix {
            RmcStatusOfFix::Autonomous => FixType::Gps,
            RmcStatusOfFix::Differential => FixType::DGps,
            RmcStatusOfFix::Invalid => FixType::Invalid,
        });
        self.latitude = rmc_data.lat;
        self.longitude = rmc_data.lon;
        self.speed_over_ground = rmc_data.speed_over_ground;
        self.true_course = rmc_data.true_course;
    }

    fn merge_gns_data(&mut self, gns_data: GnsData) {
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
        if let Some(faa_mode) = gll.faa_mode {
            self.fix_type = Some(faa_mode.into());
        } else {
            self.fix_type = Some(if gll.valid {
                FixType::Gps
            } else {
                FixType::Invalid
            });
        }
    }

    fn merge_txt_data(&mut self, txt: TxtData) {
        self.last_txt = Some(txt);
    }

    /// Parse any NMEA sentence and stores the result of sentences that include:
    /// - altitude
    /// - latitude and longitude
    /// - speed_over_ground
    /// - and other
    ///
    /// The type of sentence is returned if implemented and valid.
    pub fn parse(&mut self, sentence: &'a str) -> Result<SentenceType, Error<'a>> {
        match parse_str(sentence)? {
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
            // ParseResult::BWC(_) | ParseResult::BOD(_) | ParseResult::GBS(_) => {
            //     Err(Error::Unsupported(SentenceType::BWC))
            // }
            ParseResult::Unsupported(sentence_type) => Err(Error::Unsupported(sentence_type)),
            _ => Err(Error::Unsupported(SentenceType::BWC)),
        }
    }

    fn new_tick(&mut self) {
        let old = mem::take(self);
        self.satellites_scan = old.satellites_scan;
        self.required_sentences_for_nav = old.required_sentences_for_nav;
        self.last_fix_time = old.last_fix_time;
    }

    fn clear_position_info(&mut self) {
        self.last_fix_time = None;
        self.new_tick();
    }

    pub fn parse_for_fix(&mut self, xs: &'a str) -> Result<FixType, Error<'a>> {
        match parse_str(xs)? {
            ParseResult::GSA(gsa) => {
                self.merge_gsa_data(gsa);
                return Ok(FixType::Invalid);
            }
            ParseResult::GSV(gsv_data) => {
                self.merge_gsv_data(gsv_data)?;
                return Ok(FixType::Invalid);
            }
            ParseResult::VTG(vtg) => {
                //have no time field, so only if user explicitly mention it
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
                if rmc_data.status_of_fix == RmcStatusOfFix::Invalid {
                    self.clear_position_info();
                    return Ok(FixType::Invalid);
                }
                if !self.update_fix_time(rmc_data.fix_time) {
                    return Ok(FixType::Invalid);
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
                if !self.update_fix_time(gns_data.fix_time) {
                    return Ok(FixType::Invalid);
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
                if !self.update_fix_time(gga_data.fix_time) {
                    return Ok(FixType::Invalid);
                }
                self.merge_gga_data(gga_data);
                self.sentences_for_this_time.insert(SentenceType::GGA);
            }
            ParseResult::GLL(gll_data) => {
                if !self.update_fix_time(Some(gll_data.fix_time)) {
                    return Ok(FixType::Invalid);
                }
                self.merge_gll_data(gll_data);
                return Ok(FixType::Invalid);
            }
            ParseResult::TXT(txt_data) => {
                self.merge_txt_data(txt_data);
                return Ok(FixType::Invalid);
            }
            ParseResult::BWC(_)
            | ParseResult::BOD(_)
            | ParseResult::GBS(_)
            | ParseResult::AAM(_)
            | ParseResult::ALM(_)
            | ParseResult::PGRMZ(_) => return Ok(FixType::Invalid),

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

    fn update_fix_time(&mut self, fix_time: Option<NaiveTime>) -> bool {
        match (self.last_fix_time, fix_time) {
            (Some(ref last_fix_time), Some(ref new_fix_time)) => {
                if *last_fix_time != *new_fix_time {
                    self.new_tick();
                    self.last_fix_time = Some(*new_fix_time);
                }
            }
            (None, Some(ref new_fix_time)) => self.last_fix_time = Some(*new_fix_time),
            (Some(_), None) | (None, None) => {
                self.clear_position_info();
                return false;
            }
        }
        true
    }
}

impl fmt::Display for Nmea {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}: lat: {} lon: {} alt: {} {:?}",
            format_args!("{:?}", self.fix_time),
            format_args!("{:?}", self.latitude),
            format_args!("{:?}", self.longitude),
            format_args!("{:?}", self.altitude),
            self.satellites()
        )
    }
}

#[derive(Debug, Clone, Default)]
struct SatsPack {
    /// max number of visible GNSS satellites per hemisphere, assuming global coverage
    /// GPS: 16
    /// GLONASS: 12
    /// BeiDou: 12 + 3 IGSO + 3 GEO
    /// Galileo: 12
    /// => 58 total Satellites => max 15 rows of data
    data: Deque<Vec<Option<Satellite>, 4>, 15>,
    max_len: usize,
}

#[derive(Clone, PartialEq)]
/// Satellite information
pub struct Satellite {
    pub(crate) gnss_type: GnssType,
    pub(crate) prn: u32,
    pub(crate) elevation: Option<f32>,
    pub(crate) azimuth: Option<f32>,
    pub(crate) snr: Option<f32>,
}

impl Satellite {
    #[inline]
    pub fn gnss_type(&self) -> GnssType {
        self.gnss_type
    }
    #[inline]
    pub fn prn(&self) -> u32 {
        self.prn
    }
    #[inline]
    pub fn elevation(&self) -> Option<f32> {
        self.elevation
    }
    #[inline]
    pub fn azimuth(&self) -> Option<f32> {
        self.azimuth
    }
    #[inline]
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
            format_args!("{:?}", self.elevation),
            format_args!("{:?}", self.azimuth),
            format_args!("{:?}", self.snr),
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

macro_rules! count_tts {
    () => {0usize};
    ($_head:tt , $($tail:tt)*) => {1usize + count_tts!($($tail)*)};
    ($item:tt) => {1usize};
}
pub(crate) use count_tts;

macro_rules! define_sentence_type_enum {
    (
        $(#[$outer:meta])*
        pub enum $Name:ident {
            $(
            $(#[$variant:meta])*
            $Variant:ident
            ),* $(,)* }
    ) => {
        $(#[$outer])*
        pub enum $Name {
            $(
                $(#[$variant])*
                $Variant
            ),*,
        }

        impl<'a> TryFrom<&'a str> for SentenceType {
            type Error = crate::Error<'a>;

            fn try_from(s: &'a str) -> Result<$Name, Self::Error> {
                match s {
                    $(stringify!($Variant) => Ok($Name::$Variant),)*
                    _ => Err(Error::Unknown(s)),
                }
            }
        }

        impl $Name {
            const COUNT: usize = count_tts!($($Variant),*);
            pub const TYPES: [$Name; $Name::COUNT] = [$($Name::$Variant,)*];

            pub fn to_mask_value(self) -> u128 {
                1 << self as u32
            }

            pub fn as_str(&self) -> &str {
                match self {
                    $($Name::$Variant => stringify!($Variant),)*
                }
            }
        }

        // impl core::str::FromStr for $Name {
        //     type Err = crate::Error;

        //     fn from_str(s: &'a str) -> Result<Self, Self::Err> {
        //         match s {
        //             $(stringify!($Variant) => Ok($Name::$Variant),)*
        //             _ => Err(crate::Error::Unknown(s)),
        //         }
        //     }
        // }

        impl core::fmt::Display for $Name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    }
}

define_sentence_type_enum! {
    /// NMEA sentence type
    ///
    /// ## Types
    ///
    /// ### General
    ///
    /// - [`SentenceType::OSD`]
    ///
    /// ### Autopilot:
    ///
    /// - [`SentenceType::APA`]
    /// - [`SentenceType::APB`]
    /// - [`SentenceType::ASD`]
    ///
    /// ### Decca
    ///
    /// - [`SentenceType::DCN`]
    ///
    /// ### D-GPS
    ///
    /// - [`SentenceType::MSK`]
    ///
    /// ### Echo
    /// - [`SentenceType::DBK`]
    /// - [`SentenceType::DBS`]
    /// - [`SentenceType::DBT`]
    ///
    /// ### Radio
    ///
    /// - [`SentenceType::FSI`]
    /// - [`SentenceType::SFI`]
    /// - [`SentenceType::TLL`]
    ///
    /// ### Speed
    ///
    /// - [`SentenceType::VBW`]
    /// - [`SentenceType::VHW`]
    /// - [`SentenceType::VLW`]
    ///
    /// ### GPS
    ///
    /// - [`SentenceType::ALM`]
    /// - [`SentenceType::GBS`]
    /// - [`SentenceType::GGA`]
    /// - [`SentenceType::GNS`]
    /// - [`SentenceType::GSA`]
    /// - [`SentenceType::GSV`]
    ///
    /// ### Course
    ///
    /// - [`SentenceType::DPT`]
    /// - [`SentenceType::HDG`]
    /// - [`SentenceType::HDM`]
    /// - [`SentenceType::HDT`]
    /// - [`SentenceType::HSC`]
    /// - [`SentenceType::ROT`]
    /// - [`SentenceType::VDR`]
    ///
    /// ### Loran-C
    ///
    /// - [`SentenceType::GLC`]
    /// - [`SentenceType::LCD`]
    ///
    /// ### Machine
    ///
    /// - [`SentenceType::RPM`]
    ///
    /// ### Navigation
    ///
    /// - [`SentenceType::RMA`]
    /// - [`SentenceType::RMB`]
    /// - [`SentenceType::RMC`]
    ///
    /// ### Omega
    ///
    /// - [`SentenceType::OLN`]
    ///
    /// ### Position
    ///
    /// - [`SentenceType::GLL`]
    /// - [`SentenceType::DTM`]
    ///
    /// ### Radar
    ///
    /// - [`SentenceType::RSD`]
    /// - [`SentenceType::TLL`]
    /// - [`SentenceType::TTM`]
    ///
    /// ### Rudder
    ///
    /// - [`SentenceType::RSA`]
    ///
    /// ### Temperature
    ///
    /// - [`SentenceType::MTW`]
    ///
    /// ### Transit
    ///
    /// - [`SentenceType::GXA`]
    /// - `SentenceType::RTF` (missing?!)
    ///
    /// ### Waypoints and tacks
    ///
    /// - [`SentenceType::AAM`]
    /// - [`SentenceType::BEC`]
    /// - [`SentenceType::BOD`]
    /// - [`SentenceType::BWC`]
    /// - [`SentenceType::BWR`]
    /// - [`SentenceType::BWW`]
    /// - [`SentenceType::ROO`]
    /// - [`SentenceType::RTE`]
    /// - [`SentenceType::VTG`]
    /// - [`SentenceType::WCV`]
    /// - [`SentenceType::WNC`]
    /// - [`SentenceType::WPL`]
    /// - [`SentenceType::XDR`]
    /// - [`SentenceType::XTE`]
    /// - [`SentenceType::XTR`]
    ///
    /// ### Wind
    ///
    /// - [`SentenceType::MWV`]
    /// - [`SentenceType::VPW`]
    /// - [`SentenceType::VWR`]
    ///
    /// ### Date and Time
    ///
    /// - [`SentenceType::GTD`]
    /// - [`SentenceType::ZDA`]
    /// - [`SentenceType::ZFO`]
    /// - [`SentenceType::ZTG`]
    ///
    /// ### Vendor extensions
    ///
    /// - [`SentenceType::RMZ`]
    #[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
    #[repr(u32)]
    #[allow(rustdoc::bare_urls)]
    pub enum SentenceType {
        /// AAM - Waypoint Arrival Alarm
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_aam_waypoint_arrival_alarm>
        ///
        /// Type: `Waypoints and tacks`
        AAM,
        ABK,
        ACA,
        ACK,
        ACS,
        AIR,
        /// ALM - GPS Almanac Data
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_alm_gps_almanac_data>
        ///
        /// Type: `GPS`
        ALM,
        ALR,
        /// APA - Autopilot Sentence "A"
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_apa_autopilot_sentence_a>
        ///
        /// Type: `Autopilot`
        APA,
        /// APB - Autopilot Sentence "B"
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_apb_autopilot_sentence_b>
        ///
        /// Type: `Autopilot`
        APB,
        /// Type: `Autopilot`
        ASD,
        /// Type: `Waypoints and tacks`
        BEC,
        /// BOD - Bearing - Waypoint to Waypoint
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_bod_bearing_waypoint_to_waypoint>
        ///
        /// Type: `Waypoints and tacks`
        BOD,
        /// BWC - Bearing & Distance to Waypoint - Great Circle
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_bwc_bearing_distance_to_waypoint_great_circle>
        ///
        /// Type: `Waypoints and tacks`
        BWC,
        /// BWR - Bearing and Distance to Waypoint - Rhumb Line
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_bwr_bearing_and_distance_to_waypoint_rhumb_line>
        ///
        /// Type: `Waypoints and tacks`
        BWR,
        /// BWW - Bearing - Waypoint to Waypoint
        ///
        /// https://gpsd.gitlab.io/gpsd/NMEA.html#_bww_bearing_waypoint_to_waypoint
        ///
        /// Type: `Waypoints and tacks`
        BWW,
        CUR,
        /// DBK - Depth Below Keel
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_dbk_depth_below_keel>
        ///
        /// Type: `Echo`
        DBK,
        /// DBS - Depth Below Surface
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_dbs_depth_below_surface>
        ///
        /// Type: `Echo`
        DBS,
        /// DBT - Depth below transducer
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_dbt_depth_below_transducer>
        ///
        /// Type: `Echo`
        DBT,
        /// DCN - Decca Position
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_dcn_decca_position>
        ///
        /// Type: `Decca`
        DCN,
        /// DPT - Depth of Water
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_dpt_depth_of_water>
        ///
        /// Type: `Course`
        DPT,
        DSC,
        DSE,
        DSI,
        /// Type: `Radar`
        DSR,
        /// DTM - Datum Reference
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_dtm_datum_reference>
        ///
        /// Type: `Position`
        DTM,
        /// FSI - Frequency Set Information
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_fsi_frequency_set_information>
        ///
        /// Type: `Radio`
        FSI,
        /// GBS - GPS Satellite Fault Detection
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_gbs_gps_satellite_fault_detection>
        ///
        /// Type: `GPS`
        GBS,
        /// GGA - Global Positioning System Fix Data
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_gga_global_positioning_system_fix_data>
        ///
        /// Type: `GPS`
        GGA,
        /// GLC - Geographic Position, Loran-C
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_glc_geographic_position_loran_c>
        ///
        /// Type: `Loran-C`
        GLC,
        /// GLL - Geographic Position - Latitude/Longitude
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_gll_geographic_position_latitudelongitude>
        ///
        /// Type: `Position`
        GLL,
        GMP,
        /// GNS - Fix data
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_gns_fix_data>
        ///
        /// Type: `GPS`
        GNS,
        /// GRS - GPS Range Residuals
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_grs_gps_range_residuals>
        GRS,
        /// GSA - GPS DOP and active satellites
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_gsa_gps_dop_and_active_satellites>
        ///
        /// Type: `GPS`
        GSA,
        /// GST - GPS Pseudorange Noise Statistics
        ///
        /// https://gpsd.gitlab.io/gpsd/NMEA.html#_gst_gps_pseudorange_noise_statistics
        GST,
        /// GSV - Satellites in view
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_gsv_satellites_in_view>
        ///
        /// Type: `GPS`
        GSV,
        /// GTD - Geographic Location in Time Differences
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_gtd_geographic_location_in_time_differences>
        ///
        /// Type: `Date and Time`
        GTD,
        /// GXA - TRANSIT Position - Latitude/Longitude
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_gxa_transit_position_latitudelongitude>
        ///
        /// Type: `Transit`
        GXA,
        /// HDG - Heading - Deviation & Variation
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_hdg_heading_deviation_variation>
        ///
        /// Type: `Course`
        HDG,
        /// HDM - Heading - Magnetic
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_hdm_heading_magnetic>
        ///
        /// Type: `Course`
        HDM,
        ///
        /// HDT - Heading - True
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_hdt_heading_true>
        ///
        /// Type: `Course`
        HDT,
        /// HFB - Trawl Headrope to Footrope and Bottom
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_hfb_trawl_headrope_to_footrope_and_bottom>
        HFB,
        HMR,
        HMS,
        /// HSC - Heading Steering Command
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_hsc_heading_steering_command>
        ///
        /// Type: `Course`
        HSC,
        /// HWBIAS - Unknown
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_hwbias_unknown>
        HWBIAS,
        HTC,
        HTD,
        /// ITS - Trawl Door Spread 2 Distance
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_its_trawl_door_spread_2_distance>
        ITS,
        /// LCD - Loran-C Signal Data
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_lcd_loran_c_signal_data>
        ///
        /// Type: `Loran-C`
        LCD,
        LRF,
        LRI,
        LR1,
        LR2,
        LR3,
        /// MDA - Meteorological Composite
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_mda_meteorological_composite>
        MDA,
        MLA,
        /// MSK - Control for a Beacon Receiver
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_msk_control_for_a_beacon_receiver>
        ///
        /// Type: `D-GPS`
        MSK,
        /// MSS - Beacon Receiver Status
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_mss_beacon_receiver_status>
        MSS,
        MWD,
        /// MTW - Mean Temperature of Water
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_mtw_mean_temperature_of_water>
        ///
        /// Type: `Temperature`
        MTW,
        /// MWV - Wind Speed and Angle
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_mwv_wind_speed_and_angle>
        ///
        /// Type: `Wind`
        MWV,
        /// OLN - Omega Lane Numbers
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_oln_omega_lane_numbers>
        /// Type: `Omega`
        OLN,
        /// OSD - Own Ship Data
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_osd_own_ship_data>
        ///
        /// Type: `General`
        OSD,
        /// R00 - Waypoints in active route
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_r00_waypoints_in_active_route>
        ///
        /// Type: `Waypoints and tacks`
        ROO,
        /// RLM – Return Link Message
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_rlm_return_link_message>
        RLM,
        /// RMA - Recommended Minimum Navigation Information
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_rma_recommended_minimum_navigation_information>
        ///
        /// Type: `Navigation`
        RMA,
        /// RMB - Recommended Minimum Navigation Information
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_rmb_recommended_minimum_navigation_information>
        ///
        /// Type: `Navigation`
        RMB,
        /// RMC - Recommended Minimum Navigation Information
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_rmc_recommended_minimum_navigation_information>
        ///
        /// Type: `Navigation`
        RMC,
        /// PGRMZ - Garmin Altitude
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_pgrmz_garmin_altitude>
        ///
        /// Type: `Vendor extensions`
        RMZ,
        /// ROT - Rate Of Turn
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_rot_rate_of_turn>
        ///
        /// Type: `Course`
        ROT,
        /// RPM - Revolutions
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_rpm_revolutions>
        ///
        /// Type: `Machine`
        RPM,
        /// RSA - Rudder Sensor Angle
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_rsa_rudder_sensor_angle>
        ///
        /// Type: `Rudder`
        RSA,
        /// RSD - RADAR System Data
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_rsd_radar_system_data>
        ///
        /// Type: `Radar`
        RSD,
        /// RTE - Routes
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_rte_routes>
        ///
        /// Type: `Waypoints and tacks`
        RTE,
        /// SFI - Scanning Frequency Information
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_sfi_scanning_frequency_information>
        ///
        /// Type: `Radio`
        SFI,
        SSD,
        /// STN - Multiple Data ID
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_stn_multiple_data_id>
        STN,
        /// TDS - Trawl Door Spread Distance
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_tds_trawl_door_spread_distance>
        TDS,
        /// TFI - Trawl Filling Indicator
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_tfi_trawl_filling_indicator>
        TFI,
        /// TLB - Target Label
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_tlb_target_label>
        TLB,
        /// TLL - Target Latitude and Longitude
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_tll_target_latitude_and_longitude>
        /// Type: `Radio`
        TLL,
        /// TPC - Trawl Position Cartesian Coordinates
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_tpc_trawl_position_cartesian_coordinates>
        TPC,
        /// TPR - Trawl Position Relative Vessel
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_tpr_trawl_position_relative_vessel>
        TPR,
        /// TPT - Trawl Position True
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_tpt_trawl_position_true>
        TPT,
        /// TRF - TRANSIT Fix Data
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_trf_transit_fix_data>
        TRF,
        /// TTM - Tracked Target Message
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_ttm_tracked_target_message>
        ///
        /// Type: `Radar`
        TTM,
        TUT,
        TXT,
        /// VBW - Dual Ground/Water Speed
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_vbw_dual_groundwater_speed>
        ///
        /// Type: `Speed`
        VBW,
        VDM,
        VDO,
        /// VDR - Set and Drift
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_vdr_set_and_drift>
        ///
        /// Type: `Course`
        VDR,
        /// VHW - Water speed and heading
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_vhw_water_speed_and_heading>
        ///
        /// Type: `Speed`
        VHW,
        /// VLW - Distance Traveled through Water
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_vlw_distance_traveled_through_water>
        ///
        /// Type: `Speed`
        VLW,
        /// VPW - Speed - Measured Parallel to Wind
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_vpw_speed_measured_parallel_to_wind>
        ///
        /// Type: `Wind`
        VPW,
        VSD,
        /// VTG - Track made good and Ground speed
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_vtg_track_made_good_and_ground_speed>
        ///
        /// Type: `Waypoints and tacks`
        VTG,
        /// VWR - Relative Wind Speed and Angle
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_vwr_relative_wind_speed_and_angle>
        ///
        /// Type: `Wind`
        VWR,
        /// WCV - Waypoint Closure Velocity
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_wcv_waypoint_closure_velocity>
        ///
        /// Type: `Waypoints and tacks`
        WCV,
        /// WNC - Distance - Waypoint to Waypoint
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_wnc_distance_waypoint_to_waypoint>
        ///
        /// Type: `Waypoints and tacks`
        WNC,
        /// WPL - Waypoint Location
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_wpl_waypoint_location>
        ///
        /// Type: `Waypoints and tacks`
        WPL,
        /// XDR - Transducer Measurement
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_xdr_transducer_measurement>
        ///
        /// Type: `Waypoints and tacks`
        XDR,
        /// XTE - Cross-Track Error, Measured
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_xte_cross_track_error_measured>
        ///
        /// Type: `Waypoints and tacks`
        XTE,
        /// XTR - Cross Track Error - Dead Reckoning
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_xtr_cross_track_error_dead_reckoning>
        ///
        /// Type: `Waypoints and tacks`
        XTR,
        /// ZDA - Time & Date - UTC, day, month, year and local time zone
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_zda_time_date_utc_day_month_year_and_local_time_zone>
        ///
        /// Type: `Date and Time`
        ZDA,
        ZDL,
        /// ZFO - UTC & Time from origin Waypoint
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_zfo_utc_time_from_origin_waypoint>
        ///
        /// Type: `Date and Time`
        ZFO,
        /// ZTG - UTC & Time to Destination Waypoint
        ///
        /// <https://gpsd.gitlab.io/gpsd/NMEA.html#_ztg_utc_time_to_destination_waypoint>
        ///
        /// Type: `Date and Time`
        ZTG,
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
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

#[cfg(test)]
mod tests {
    use core::convert::TryFrom;

    use quickcheck::{QuickCheck, TestResult};

    use crate::{parse::checksum, sentences::FixType, Error, Nmea, SentenceType};

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
        let valid = "$GNGSA,A,1,,,,,,,,,,,,,99.99,99.99,99.99*2E";
        let invalid = "$GNZDA,165118.00,13,05,2016,00,00*71";
        assert_eq!(checksum(valid[1..valid.len() - 3].as_bytes().iter()), 0x2E);
        assert_ne!(
            checksum(invalid[1..invalid.len() - 3].as_bytes().iter()),
            0x71
        );
    }

    #[test]
    fn test_message_type() {
        assert_eq!(SentenceType::try_from("GGA"), Ok(SentenceType::GGA));
        let parse_err = SentenceType::try_from("XXX").expect_err("Should trigger parsing error");

        assert_eq!(Error::Unknown("XXX"), parse_err);
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
    fn test_sentence_type_enum() {
        // So we don't trip over the max value of u128 when shifting it with
        // SentenceType as u32
        for sentence_type in SentenceType::TYPES {
            assert!((sentence_type as u32) < 127);
        }
    }
}
