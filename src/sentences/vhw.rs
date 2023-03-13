use crate::{Error, NmeaSentence};

/// VHW - Water speed and heading
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_vhw_water_speed_and_heading>
///
/// ```text
///        1   2 3   4 5   6 7   8 9
///        |   | |   | |   | |   | |
/// $--VHW,x.x,T,x.x,M,x.x,N,x.x,K*hh<CR><LF>
/// ```
/// 1. Heading degrees, True
/// 2. T = True
/// 3. Heading degrees, Magnetic
/// 4. M = Magnetic
/// 5. Speed of vessel relative to the water, knots
/// 6. N = Knots
/// 7. Speed of vessel relative to the water, km/hr
/// 8. K = Kilometers
/// 9. Checksum
///
/// Note that this implementation follows the documentation published by `gpsd`, but the GLOBALSAT documentation may have conflicting definitions.
/// > [[GLOBALSAT](https://gpsd.gitlab.io/gpsd/NMEA.html#GLOBALSAT)] describes a different format in which the first three fields are water-temperature measurements.
/// > It’s not clear which is correct.
#[derive(Clone, PartialEq, Debug)]
pub struct VhwData {
    /// Heading degrees, True
    pub heading_true: Option<f64>,
    /// Heading degrees, Magnetic
    pub heading_magnetic: Option<f64>,
    /// Speed of vessel relative to the water, knots
    pub relative_speed_knots: Option<f64>,
    /// Speed of vessel relative to the water, km/hr
    pub relative_speed_kmph: Option<f64>,
}

/// # Parse VHW message
///
/// ```text
/// ```
pub fn parse_vhw(sentence: NmeaSentence) -> Result<VhwData, Error> {
    todo!()
}

#[cfg(test)]
mod tests {}
