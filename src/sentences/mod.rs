//! All the supported sentence type data and parsers.

mod aam;
mod alm;
mod bod;
mod bwc;
mod gbs;
mod gga;
mod gll;
mod gns;
mod gsa;
mod gsv;
mod mda;
mod mwv;
mod rmc;
mod rmz;
mod txt;
mod utils;
mod vtg;

pub(crate) mod faa_mode;
mod fix_type;
mod gnss_type;

pub use {
    aam::{parse_aam, AamData},
    alm::{parse_alm, AlmData},
    bod::{parse_bod, BodData},
    bwc::{parse_bwc, BwcData},
    faa_mode::{FaaMode, FaaModes},
    fix_type::FixType,
    gbs::{parse_gbs, GbsData},
    gga::{parse_gga, GgaData},
    gll::{parse_gll, GllData},
    gns::{parse_gns, GnsData, NavigationStatus},
    gnss_type::GnssType,
    gsa::{parse_gsa, GsaData},
    gsv::{parse_gsv, GsvData},
    mda::{parse_mda, MdaData},
    mwv::{parse_mwv, MwvData, MwvReference, MwvWindSpeedUnits},
    rmc::{parse_rmc, RmcData, RmcStatusOfFix},
    rmz::{parse_pgrmz, PgrmzData},
    txt::{parse_txt, TxtData},
    vtg::{parse_vtg, VtgData},
};

pub(crate) fn nom_parse_failure(inp: &str) -> nom::Err<nom::error::Error<&str>> {
    nom::Err::Failure(nom::error::Error::new(inp, nom::error::ErrorKind::Fail))
}
