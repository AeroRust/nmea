use core::fmt;

use crate::count_tts;

macro_rules! define_enum_with_count {
    (
        $(#[$outer:meta])*
        enum $Name:ident { $(
            $(#[$variant:meta])*
            $Variant:ident
        ),* $(,)* }
    ) => {
        $(#[$outer])*
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
