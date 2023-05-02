//! All the supported sentence type data and parsers.

pub mod aam;
pub mod alm;
pub mod bod;
pub mod bwc;
pub mod dbk;
pub mod gbs;
pub mod gga;
pub mod gll;
pub mod gns;
pub mod gsa;
pub mod gsv;
pub mod hdt;
pub mod mda;
pub mod mtw;
pub mod mwv;
pub mod rmc;
pub mod rmz;
pub mod txt;
pub mod utils;
pub mod vhw;
pub mod vtg;
pub mod zda;

pub mod faa_mode;
pub mod fix_type;
pub mod gnss_type;

pub(crate) use {
    aam::{parse_aam, AamData},
    alm::{parse_alm, AlmData},
    bod::{parse_bod, BodData},
    bwc::{parse_bwc, BwcData},
    dbk::{parse_dbk, DbkData},
    faa_mode::{FaaMode, FaaModes},
    fix_type::FixType,
    gbs::{parse_gbs, GbsData},
    gga::{parse_gga, GgaData},
    gll::{parse_gll, GllData},
    gns::{parse_gns, GnsData},
    gnss_type::GnssType,
    gsa::{parse_gsa, GsaData},
    gsv::{parse_gsv, GsvData},
    hdt::{parse_hdt, HdtData},
    mda::{parse_mda, MdaData},
    mtw::{parse_mtw, MtwData},
    mwv::{parse_mwv, MwvData},
    rmc::{parse_rmc, RmcData, RmcStatusOfFix},
    rmz::{parse_pgrmz, PgrmzData},
    txt::{parse_txt, TxtData},
    vhw::{parse_vhw, VhwData},
    vtg::{parse_vtg, VtgData},
    zda::{parse_zda, ZdaData},
};

pub(crate) fn nom_parse_failure(inp: &str) -> nom::Err<nom::error::Error<&str>> {
    nom::Err::Failure(nom::error::Error::new(inp, nom::error::ErrorKind::Fail))
}
