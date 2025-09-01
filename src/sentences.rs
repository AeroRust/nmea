//! All the supported sentence type data and parsers.

pub mod aam;
pub mod alm;
pub mod apa;
pub mod bod;
pub mod bwc;
pub mod bww;
pub mod dbk;
pub mod dbs;
pub mod dpt;
pub mod gbs;
pub mod gga;
pub mod gll;
pub mod gns;
pub mod gsa;
pub mod gst;
pub mod gsv;
pub mod hdt;
pub mod mda;
pub mod mtw;
pub mod mwv;
pub mod rmc;
pub mod rmz;
pub mod ttm;
pub mod txt;
pub mod utils;
pub mod vhw;
pub mod vtg;
pub mod wnc;
pub mod zda;
pub mod zfo;
pub mod ztg;

pub mod faa_mode;
pub mod fix_type;
pub mod gnss_type;

#[doc(inline)]
pub use {
    aam::{AamData, parse_aam},
    alm::{AlmData, parse_alm},
    apa::{ApaData, parse_apa},
    bod::{BodData, parse_bod},
    bwc::{BwcData, parse_bwc},
    bww::{BwwData, parse_bww},
    dbk::{DbkData, parse_dbk},
    dbs::{DbsData, parse_dbs},
    dpt::{DptData, parse_dpt},
    faa_mode::{FaaMode, FaaModes},
    fix_type::FixType,
    gbs::{GbsData, parse_gbs},
    gga::{GgaData, parse_gga},
    gll::{GllData, parse_gll},
    gns::{GnsData, parse_gns},
    gnss_type::GnssType,
    gsa::{GsaData, parse_gsa},
    gst::{GstData, parse_gst},
    gsv::{GsvData, parse_gsv},
    hdt::{HdtData, parse_hdt},
    mda::{MdaData, parse_mda},
    mtw::{MtwData, parse_mtw},
    mwv::{MwvData, parse_mwv},
    rmc::{RmcData, parse_rmc},
    rmz::{PgrmzData, parse_pgrmz},
    ttm::{
        TtmAngle, TtmData, TtmDistanceUnit, TtmReference, TtmStatus, TtmTypeOfAcquisition,
        parse_ttm,
    },
    txt::{TxtData, parse_txt},
    vhw::{VhwData, parse_vhw},
    vtg::{VtgData, parse_vtg},
    wnc::{WncData, parse_wnc},
    zda::{ZdaData, parse_zda},
    zfo::{ZfoData, parse_zfo},
    ztg::{ZtgData, parse_ztg},
};

pub(crate) fn nom_parse_failure(inp: &str) -> nom::Err<nom::error::Error<&str>> {
    nom::Err::Failure(nom::error::Error::new(inp, nom::error::ErrorKind::Fail))
}
