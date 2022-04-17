mod bwc;
mod gga;
mod gll;
mod gns;
mod gsa;
mod gsv;
mod rmc;
mod txt;
mod utils;
mod vtg;

pub use bwc::{parse_bwc, BwcData};
pub use gga::{parse_gga, GgaData};
pub use gll::{parse_gll, GllData, PosSystemIndicator};
pub use gns::{parse_gns, GnsData, NavigationStatus};
pub use gsa::{parse_gsa, GsaData};
pub use gsv::{parse_gsv, GsvData};
pub use rmc::{parse_rmc, RmcData, RmcStatusOfFix};
pub use txt::{parse_txt, TxtData};
pub use vtg::{parse_vtg, VtgData};

use nom::IResult;

use crate::FixType;

/// for now let's handle only two GPS and GLONASS
#[derive(Clone, Copy, Debug, PartialEq)]
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FaaMode {
    Autonomous,
    /// Quectel unique
    Caution,
    Differential,
    /// Estimated dead reckoning
    Estimated,
    FloatRtk,
    DataNotValid,
    /// Sort of DGPS, NMEA 4+
    Precise,
    FixedRtk,
    /// Same as simulated?  Or surveyed?
    Manual,
    Simualtor,
    /// Quectel unique
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
            FaaMode::Simualtor => FixType::Simulation,
            FaaMode::Unsafe => FixType::Invalid,
        }
    }
}

pub(crate) fn parse_faa_modes(i: &[u8]) -> IResult<&[u8], FaaModes> {
    let (sym, rest) = match i.split_first() {
        Some(x) => x,
        None => {
            return Err(nom_parse_failure(i));
        }
    };

    let mut ret = FaaModes {
        sys_state0: parse_faa_mode(*sym).ok_or_else(|| nom_parse_failure(i))?,
        sys_state1: None,
    };

    let (sym, rest2) = match rest.split_first() {
        Some(x) => x,
        None => {
            return Ok((rest, ret));
        }
    };
    ret.sys_state1 = Some(parse_faa_mode(*sym).ok_or_else(|| nom_parse_failure(rest))?);
    if rest2.is_empty() {
        Ok((rest2, ret))
    } else {
        Err(nom_parse_failure(rest2))
    }
}

fn parse_faa_mode(value: u8) -> Option<FaaMode> {
    match value {
        b'A' => Some(FaaMode::Autonomous),
        b'C' => Some(FaaMode::Caution),
        b'D' => Some(FaaMode::Differential),
        b'E' => Some(FaaMode::Estimated),
        b'F' => Some(FaaMode::FloatRtk),
        b'N' => Some(FaaMode::DataNotValid),
        b'P' => Some(FaaMode::Precise),
        b'R' => Some(FaaMode::FixedRtk),
        b'M' => Some(FaaMode::Manual),
        b'S' => Some(FaaMode::Simualtor),
        b'U' => Some(FaaMode::Unsafe),
        _ => None,
    }
}

fn nom_parse_failure(inp: &[u8]) -> nom::Err<nom::error::Error<&[u8]>> {
    nom::Err::Failure(nom::error::Error::new(inp, nom::error::ErrorKind::Fail))
}

#[test]
fn test_parse_faa_modes() {
    assert_eq!(nom_parse_failure(b""), parse_faa_modes(b"").unwrap_err());
    assert_eq!(
        (
            b"" as &[u8],
            FaaModes {
                sys_state0: FaaMode::Autonomous,
                sys_state1: None,
            }
        ),
        parse_faa_modes(b"A").unwrap()
    );

    assert_eq!(
        (
            b"" as &[u8],
            FaaModes {
                sys_state0: FaaMode::DataNotValid,
                sys_state1: Some(FaaMode::Autonomous),
            }
        ),
        parse_faa_modes(b"NA").unwrap()
    );
}
