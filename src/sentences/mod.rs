//! All the supported sentence type data and parsers.

mod aam;
mod bod;
mod bwc;
mod gbs;
mod gga;
mod gll;
mod gns;
mod gsa;
mod gsv;
mod rmc;
mod txt;
mod utils;
mod vtg;

pub(crate) mod faa_mode;
mod fix_type;
mod gnss_type;

pub use {
    aam::{parse_aam, AamData},
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
    rmc::{parse_rmc, RmcData, RmcStatusOfFix},
    txt::{parse_txt, TxtData},
    vtg::{parse_vtg, VtgData},
};

pub(crate) fn nom_parse_failure(inp: &str) -> nom::Err<nom::error::Error<&str>> {
    nom::Err::Failure(nom::error::Error::new(inp, nom::error::ErrorKind::Fail))
}
