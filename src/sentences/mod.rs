//! All the supported sentence type data and parsers.

pub mod aam;
pub mod alm;
pub mod apa;
pub mod bod;
pub mod bwc;
pub mod bww;
pub mod dbk;
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
    aam::{parse_aam, AamData},
    alm::{parse_alm, AlmData},
    apa::{parse_apa, ApaData},
    bod::{parse_bod, BodData},
    bwc::{parse_bwc, BwcData},
    bww::{parse_bww, BwwData},
    dbk::{parse_dbk, DbkData},
    faa_mode::{FaaMode, FaaModes},
    fix_type::FixType,
    gbs::{parse_gbs, GbsData},
    gga::{parse_gga, GgaData},
    gll::{parse_gll, GllData},
    gns::{parse_gns, GnsData},
    gnss_type::GnssType,
    gsa::{parse_gsa, GsaData},
    gst::{parse_gst, GstData},
    gsv::{parse_gsv, GsvData},
    hdt::{parse_hdt, HdtData},
    mda::{parse_mda, MdaData},
    mtw::{parse_mtw, MtwData},
    mwv::{parse_mwv, MwvData},
    rmc::{parse_rmc, RmcData},
    rmz::{parse_pgrmz, PgrmzData},
    ttm::{
        parse_ttm, TtmAngle, TtmData, TtmDistanceUnit, TtmReference, TtmStatus,
        TtmTypeOfAcquisition,
    },
    txt::{parse_txt, TxtData},
    vhw::{parse_vhw, VhwData},
    vtg::{parse_vtg, VtgData},
    wnc::{parse_wnc, WncData},
    zda::{parse_zda, ZdaData},
    zfo::{parse_zfo, ZfoData},
    ztg::{parse_ztg, ZtgData},
};

pub(crate) fn nom_parse_failure(inp: &str) -> nom::Err<nom::error::Error<&str>> {
    nom::Err::Failure(nom::error::Error::new(inp, nom::error::ErrorKind::Fail))
}
