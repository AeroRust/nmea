use nom::{character::complete::anychar, combinator::opt, IResult};

use super::{nom_parse_failure, FixType};

/// for now let's handle only two GPS and GLONASS
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaaModes {
    sys_state0: FaaMode,
    sys_state1: Option<FaaMode>,
}

impl From<FaaModes> for FixType {
    fn from(modes: FaaModes) -> Self {
        let fix_type: FixType = modes.sys_state0.into();
        if fix_type.is_valid() {
            return fix_type;
        }
        if let Some(fix_type2) = modes.sys_state1.map(FixType::from) {
            if fix_type2.is_valid() {
                return fix_type2;
            }
        }
        fix_type
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaaMode {
    /// A - Autonomous mode
    Autonomous,
    /// C - Quectel Querk, "Caution"
    Caution,
    /// D - Differential Mode
    Differential,
    /// E - Estimated (dead-reckoning) mode
    Estimated,
    /// F - RTK Float mode
    FloatRtk,
    /// M - Manual Input Mode
    Manual,
    /// N - Data Not Valid
    DataNotValid,
    /// P - Precise (4.00 and later)
    ///
    /// Sort of DGPS, NMEA 4+
    Precise,
    /// R - RTK Integer mode
    FixedRtk,
    /// S - Simulated Mode
    Simulator,
    /// U - Quectel Querk, "Unsafe"
    Unsafe,
}

impl From<FaaMode> for FixType {
    fn from(mode: FaaMode) -> Self {
        match mode {
            FaaMode::Autonomous => FixType::Gps,
            FaaMode::Caution => FixType::Invalid,
            FaaMode::Differential => FixType::DGps,
            FaaMode::Estimated => FixType::Estimated,
            FaaMode::FloatRtk => FixType::FloatRtk,
            FaaMode::DataNotValid => FixType::Invalid,
            FaaMode::Precise => FixType::DGps,
            FaaMode::FixedRtk => FixType::Rtk,
            FaaMode::Manual => FixType::Manual,
            FaaMode::Simulator => FixType::Simulation,
            FaaMode::Unsafe => FixType::Invalid,
        }
    }
}

pub(crate) fn parse_faa_modes(i: &str) -> IResult<&str, FaaModes> {
    let (rest, sym) = anychar(i)?;

    let mut ret = FaaModes {
        sys_state0: parse_faa_mode(sym).ok_or_else(|| nom_parse_failure(i))?,
        sys_state1: None,
    };

    let (rest2, sym) = opt(anychar)(rest)?;

    match sym {
        Some(sym) => {
            ret.sys_state1 = Some(parse_faa_mode(sym).ok_or_else(|| nom_parse_failure(rest))?);
        }
        None => return Ok((rest, ret)),
    };

    if rest2.is_empty() {
        Ok((rest2, ret))
    } else {
        Err(nom_parse_failure(rest2))
    }
}

pub(crate) fn parse_faa_mode(value: char) -> Option<FaaMode> {
    match value {
        'A' => Some(FaaMode::Autonomous),
        'C' => Some(FaaMode::Caution),
        'D' => Some(FaaMode::Differential),
        'E' => Some(FaaMode::Estimated),
        'F' => Some(FaaMode::FloatRtk),
        'N' => Some(FaaMode::DataNotValid),
        'P' => Some(FaaMode::Precise),
        'R' => Some(FaaMode::FixedRtk),
        'M' => Some(FaaMode::Manual),
        'S' => Some(FaaMode::Simulator),
        'U' => Some(FaaMode::Unsafe),
        _ => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_faa_modes() {
        assert_eq!(
            nom::Err::Error(nom::error::Error::new("", nom::error::ErrorKind::Eof)),
            parse_faa_modes("").unwrap_err(),
            "Should return a Digit error on empty string"
        );
        assert_eq!(
            (
                "",
                FaaModes {
                    sys_state0: FaaMode::Autonomous,
                    sys_state1: None,
                }
            ),
            parse_faa_modes("A").unwrap()
        );

        assert_eq!(
            (
                "",
                FaaModes {
                    sys_state0: FaaMode::DataNotValid,
                    sys_state1: Some(FaaMode::Autonomous),
                }
            ),
            parse_faa_modes("NA").unwrap()
        );
    }
}
