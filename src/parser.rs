//! The [`NMEA`] parser.

use core::{fmt, mem, ops::BitOr};

use chrono::{NaiveDate, NaiveTime};
use heapless::{Deque, Vec};

use crate::{parse, sentences::*, NmeaError, ParseResult};

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

    fn merge_gsv_data(&mut self, data: GsvData) -> Result<(), NmeaError<'a>> {
        {
            let d = &mut self.satellites_scan[data.gnss_type as usize];
            let full_pack_size: usize = data
                .sentence_num
                .try_into()
                .map_err(|_| NmeaError::InvalidGsvSentenceNum)?;
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

    /// Parse any NMEA sentence and stores the result. The type of sentence
    /// is returned if implemented and valid.
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

macro_rules! define_sentence_type_enum {
    (
        $(#[$outer:meta])*
        enum $Name:ident {
            $(
            $(#[$variant:meta])*
            $Variant:ident
            ),* $(,)* }
    ) => {
        $(#[$outer])*
        #[derive(PartialEq, Debug, Hash, Eq, Clone, Copy)]
        #[repr(C)]
        pub enum $Name {
            $(
                $(#[$variant])*
                $Variant
            ),*,
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
            pub(crate) fn from_slice(s: &[u8]) -> Self {
                $(
                    #[allow(nonstandard_style)]
                    const $Variant: &[u8] = stringify!($Variant).as_bytes();
                )*
                match s {
                    $($Variant => $Name::$Variant,)*
                    _ => $Name::None,
                }
            }

            fn to_mask_value(self) -> u128 {
                1 << self as u32
            }
        }
    }
}

define_sentence_type_enum!(
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
    enum SentenceType {
        /// Type: `Waypoints and tacks`
        AAM,
        ABK,
        ACA,
        ACK,
        ACS,
        AIR,
        /// Type: `GPS`
        ALM,
        ALR,
        /// Type: `Autopilot`
        APA,
        /// Type: `Autopilot`
        APB,
        /// Type: `Autopilot`
        ASD,
        /// Type: `Waypoints and tacks`
        BEC,
        /// Type: `Waypoints and tacks`
        BOD,
        /// Type: `Waypoints and tacks`
        BWC,
        /// Type: `Waypoints and tacks`
        BWR,
        /// Type: `Waypoints and tacks`
        BWW,
        CUR,
        /// Type: `Echo`
        DBK,
        /// Type: `Echo`
        DBS,
        /// Type: `Echo`
        DBT,
        /// Type: `Decca`
        DCN,
        /// Type: `Course`
        DPT,
        DSC,
        DSE,
        DSI,
        /// Type: `Radar`
        DSR,
        /// Type: `Position`
        DTM,
        /// Type: `Radio`
        FSI,
        /// Type: `GPS`
        GBS,
        /// Type: `GPS`
        GGA,
        /// Type: `Loran-C`
        GLC,
        /// Type: `Position`
        GLL,
        GMP,
        /// Type: `GPS`
        GNS,
        GRS,
        /// Type: `GPS`
        GSA,
        GST,
        /// Type: `GPS`
        GSV,
        /// Type: `Date and Time`
        GTD,
        /// Type: `Transit`
        GXA,
        /// Type: `Course`
        HDG,
        /// Type: `Course`
        HDM,
        /// Type: `Course`
        HDT,
        HMR,
        HMS,
        /// Type: `Course`
        HSC,
        HTC,
        HTD,
        /// Type: `Loran-C`
        LCD,
        LRF,
        LRI,
        LR1,
        LR2,
        LR3,
        MLA,
        /// Type: `D-GPS`
        MSK,
        MSS,
        MWD,
        /// Type: `Temperature`
        MTW,
        /// Type: `Wind`
        MWV,
        /// Type: `Omega`
        OLN,
        /// Type: `General`
        OSD,
        /// Type: `Waypoints and tacks`
        ROO,
        /// Type: `Navigation`
        RMA,
        /// Type: `Navigation`
        RMB,
        /// Type: `Navigation`
        RMC,
        /// Type: `Course`
        ROT,
        /// Type: `Machine`
        RPM,
        /// Type: `Rudder`
        RSA,
        /// Type: `Radar`
        RSD,
        /// Type: `Waypoints and tacks`
        RTE,
        /// Type: `Radio`
        SFI,
        SSD,
        STN,
        TLB,
        /// Type: `Radio`
        TLL,
        TRF,
        /// Type: `Radar`
        TTM,
        TUT,
        TXT,
        /// Type: `Speed`
        VBW,
        VDM,
        VDO,
        /// Type: `Course`
        VDR,
        /// Type: `Speed`
        VHW,
        /// Type: `Speed`
        VLW,
        /// Type: `Wind`
        VPW,
        VSD,
        /// Type: `Waypoints and tacks`
        VTG,
        /// Type: `Wind`
        VWR,
        /// Type: `Waypoints and tacks`
        WCV,
        /// Type: `Waypoints and tacks`
        WNC,
        /// Type: `Waypoints and tacks`
        WPL,
        /// Type: `Waypoints and tacks`
        XDR,
        /// Type: `Waypoints and tacks`
        XTE,
        /// Type: `Waypoints and tacks`
        XTR,
        /// Type: `Date and Time`
        ZDA,
        ZDL,
        /// Type: `Date and Time`
        ZFO,
        /// Type: `Date and Time`
        ZTG,
    }
);

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

/// Fix type
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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

macro_rules! count_tts {
    () => {0usize};
    ($_head:tt , $($tail:tt)*) => {1usize + count_tts!($($tail)*)};
    ($item:tt) => {1usize};
}

macro_rules! define_enum_with_count {
    (
        $(#[$outer:meta])*
        enum $Name:ident { $($Variant:ident),* $(,)* }
    ) => {
        $(#[$outer])*
        #[derive(PartialEq, Debug, Hash, Eq, Clone, Copy)]
        #[repr(u8)]
        pub enum $Name {
            $($Variant),*
        }
        impl $Name {
            const COUNT: usize = count_tts!($($Variant),*);
        }
    };
}

define_enum_with_count!(
    /// GNSS type
    enum GnssType {
        Beidou,
        Galileo,
        Gps,
        Glonass,
    }
);

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
    use quickcheck::{QuickCheck, TestResult};

    use crate::{FixType, Nmea, SentenceType};

    use super::parse::checksum;

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
        assert_eq!(SentenceType::from_slice(b"GGA"), SentenceType::GGA);
        assert_eq!(SentenceType::from_slice(b"XXX"), SentenceType::None);
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
        assert!((SentenceType::None as u32) < 127);
    }

    #[test]
    fn test_all_supported_messages() {
        let mut nmea = Nmea::default();

        let messages = [
            // BWC
            "$GPBWC,220516,5130.02,N,00046.34,W,213.8,T,218.0,M,0004.6,N,EGLM*21",
            // GGA
            "$GPGGA,133605.0,5521.75946,N,03731.93769,E,0,00,,,M,,M,,*4F",
            // GLL
            "$GPGLL,5107.0013414,N,11402.3279144,W,205412.00,A,A*73",
            // GNS
            "$GPGNS,224749.00,3333.4268304,N,11153.3538273,W,D,19,0.6,406.110,-26.294,6.0,0138,S,*46",
            // GSA
            "$GPGSA,A,3,23,31,22,16,03,07,,,,,,,1.8,1.1,1.4*3E",
            // GSV
            "$GPGSV,3,1,12,01,49,196,41,03,71,278,32,06,02,323,27,11,21,196,39*72",
            // RMC
            "$GPRMC,225446.33,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E,A*2B",
            // TXT
            "$GNTXT,01,01,02,u-blox AG - www.u-blox.com*4E",
            // VTG
            "$GPVTG,360.0,T,348.7,M,000.0,N,000.0,K*43",
        ];

        let results = messages
            .iter()
            .map(|message| nmea.parse(message).map_err(|result| (message, result)))
            .collect::<Vec<_>>();

        let errors = results
            .into_iter()
            .filter_map(|result| result.err())
            .collect::<Vec<_>>();

        // for displaying
        let display_errors = errors
            .iter()
            .map(|(msg, err)| format!("Message: {} with error: {}", msg, err))
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(
            0,
            errors.len(),
            "All supported messages should be parsable:\n{}",
            display_errors
        )
    }
}
