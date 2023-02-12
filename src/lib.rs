//! NMEA 0183 parser
//!
//! Use [`Nmea::parse()`] and [`Nmea::parse_for_fix()`]
//! to preserve state between receiving new NMEA sentence,
//! and [`parse_str()`] or [`parse_bytes()`] to parse sentences without state.
//!
//! Units used: **celsius**, **degrees**, **knots**, **meters** for altitude
//!
//! # Supported sentences:
//!
//! NMEA Standard Sentences
//! - AAM
//! - ALM
//! - BOD
//! - BWC
//! - GBS
//! - GGA *
//! - GLL *
//! - GNS *
//! - GSA *
//! - GSV *
//! - MDA
//! - MWV
//! - RMC *
//! - VTG *
//!
//! Other Sentences
//! - TXT *
//!
//! Vendor Extension
//! - PGRMZ
//!
//! **\* [`Nmea::parse()`] supported sentences**
//!
//!
//! # Crate features
//!
//! - `default` features - `std`
//! - `std` - enable `std`
//!
//! [`Nmea::parse()`]: Nmea::parse
//! [`Nmea::parse_for_fix()`]: Nmea::parse_for_fix

// only enables the `doc_cfg` feature when
// the `docsrs` configuration attribute is defined
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(any(feature = "std", test)), no_std)]
#![deny(unsafe_code, rustdoc::broken_intra_doc_links)]

mod error;
pub(crate) mod parse;
mod parser;

pub mod sentences;

#[doc(inline)]
pub use parser::*;

pub use error::Error;

#[doc(inline)]
pub use parse::*;

#[cfg(doctest)]
// Test the README examples
doc_comment::doctest!("../README.md");
