use crate::count_tts;
use core::fmt;

macro_rules! define_enum_with_count {
    (
        $(#[$outer:meta])*
        enum $Name:ident { $(
            $(#[$variant:meta])*
            $Variant:ident
        ),* $(,)* }
    ) => {
        $(#[$outer])*
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        #[derive(PartialEq, Debug, Hash, Eq, Clone, Copy)]
        #[repr(u8)]
        pub enum $Name {
            $($Variant),*
        }
        impl $Name {
            pub(crate) const COUNT: usize = count_tts!($($Variant),*);
            pub const ALL_TYPES: [$Name; $Name::COUNT] = [
                $($Name::$Variant),*
            ];
        }
    };
}

define_enum_with_count!(
    /// Supported GNSS types
    enum GnssType {
        /// BeiDou Navigation Satellite System (BDS) from China.
        Beidou,
        /// European Global Navigation System (Galileo) from Europe.
        Galileo,
        /// Global Positioning System (GPS) from the United States.
        Gps,
        /// Globalnaya Navigazionnaya Sputnikovaya Sistema (GLONASS) from Russia.
        Glonass,
        /// Navigation Indian Constellation (NavIC) from India.
        NavIC,
        /// Quasi-Zenith Satellite System (QZSS) from Japan.
        Qzss,
    }
);

impl fmt::Display for GnssType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GnssType::Beidou => write!(f, "Beidou"),
            GnssType::Galileo => write!(f, "Galileo"),
            GnssType::Gps => write!(f, "GPS"),
            GnssType::Glonass => write!(f, "GLONASS"),
            GnssType::NavIC => write!(f, "NavIC"),
            GnssType::Qzss => write!(f, "QZSS"),
        }
    }
}

impl From<GnssSystemId> for GnssType {
    fn from(id: GnssSystemId) -> Self {
        match id {
            GnssSystemId::Gps => Self::Gps,
            GnssSystemId::Glonass => Self::Glonass,
            GnssSystemId::Galileo => Self::Galileo,
            GnssSystemId::Beidou => Self::Beidou,
        }
    }
}

define_enum_with_count!(
    /// Supported GNSS types for System ID (GSA NMEA 4.1+)
    enum GnssSystemId {
        /// Global Positioning System (GPS) from the United States.
        Gps,
        /// Globalnaya Navigazionnaya Sputnikovaya Sistema (GLONASS) from Russia.
        Glonass,
        /// European Global Navigation System (Galileo) from Europe.
        Galileo,
        /// BeiDou Navigation Satellite System (BDS) from China.
        Beidou,
    }
);

impl fmt::Display for GnssSystemId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Beidou => write!(f, "Beidou"),
            Self::Galileo => write!(f, "Galileo"),
            Self::Gps => write!(f, "GPS"),
            Self::Glonass => write!(f, "GLONASS"),
        }
    }
}

impl TryFrom<u8> for GnssSystemId {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Gps),
            2 => Ok(Self::Glonass),
            3 => Ok(Self::Galileo),
            4 => Ok(Self::Beidou),
            _ => Err(()),
        }
    }
}
