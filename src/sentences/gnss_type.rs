use core::fmt;

use crate::count_tts;

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
            pub(crate) const COUNT: usize = count_tts!($($Variant),*);
            pub const ALL_TYPES: [$Name; $Name::COUNT] = [
                $($Name::$Variant),*
            ];
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
