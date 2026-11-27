//! ## NMEA 0183 parser
//!
//! Use [`Nmea::parse()`] and [`Nmea::parse_for_fix()`]
//! to preserve state between receiving new NMEA sentences
//! (large size, not recommended for embedded platforms).
//!
//! For embedded platforms, i.e. `no_std`, use [`parse_str()`] or [`parse_bytes()`]
//! to parse sentences without preserving state.
//!
//! Units used: **celsius**, **degrees**, **knots**, **meters** for altitude.
//!
//! Check the feature flags below for all the supported sentences.
//!
//! ## Crate features
#![cfg_attr(feature = "features-docs", doc = document_features::document_features!())]
#![cfg_attr(
    not(feature = "features-docs"),
    doc = "Please enable the `features-docs` feature to see the documented features list"
)]
//!
//! **\* [`Nmea::parse()`] supported sentences**
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
pub mod stream;

pub mod sentences;

#[doc(inline)]
pub use parser::*;

pub use error::Error;

#[doc(inline)]
pub use parse::*;

#[cfg(doctest)]
// Test the README examples
doc_comment::doctest!("../README.md");
