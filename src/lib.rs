//! NMEA 0183 parser
//!
//! Use [`Nmea::parse()`](Nmea::parse) and [`Nmea::parse_for_fix()`](Nmea::parse_for_fix)
//! to preserve state between receiving new NMEA sentence,
//! and [`parse()`] to parse sentences without state.
//!
//! Units used: **degrees**, **knots**, **meters** for altitude
//!
//! # Supported sentences:
//! - BWC
//! - GGA
//! - GLL
//! - GNS
//! - GSA
//! - GSV
//! - RMC
//! - TXT
//! - VTG
//!
//! # Crate features
//!
//! - `default` features - `std`
//! - `std` - enable `std`

// only enables the `doc_cfg` feature when
// the `docsrs` configuration attribute is defined
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(any(feature = "std", test)), no_std)]
#![deny(unsafe_code, rustdoc::broken_intra_doc_links)]

mod parse;
mod parser;

pub(crate) mod sentences;

#[doc(inline)]
pub use parser::*;

#[doc(inline)]
pub use parse::{
    parse, BwcData, GgaData, GllData, GsaData, GsvData, NmeaError, ParseResult, RmcData,
    RmcStatusOfFix, TxtData, VtgData, SENTENCE_MAX_LEN,
};

#[cfg(doctest)]
// Test the README examples
doc_comment::doctest!("../README.md");

