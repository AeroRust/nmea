# Changelog

All notable changes to this project will be documented in this file.

## [0.8.0] - 2026-08-08

### 🚀 Features

- Derive defmt::Format and Clone/Copy for additional structs
- Document all features + add DBS to features
- ParseResult - derive serde's Serialize & Deserialize

### 🐛 Bug Fixes

- *(docs)* Parser - bare urls
- Edition 2024 suggestion for temp. variable
- Broken docs link and commented out code
- Fix clearing of fix_satellites_prns
- Reduce stack usage (#168)

### 💼 Other

- Implementing DBS(Depth Below Surface)
- Updated readme.md with DBS in supported sentences
- Added a sentence parsing check for DBS
- Update tests/all_supported_messages.rs
- Parse talker ID `GQ` as QZSS
- Make `GllData.fix_time` optional
- Fix compilation error when printing `Error` with defmt
- Extend GSA parsing with System ID
- Fix parse error for GSA with empty tail and system ID (#3)
- Implement From<GnssSystemID> for GnssType
- Replace system_id_from_maybe_raw with a TryFrom<u8> implementation
- Add support for GSA satellite accumulation in multi-constellation setups with GSA cycle detection
- Stricter tests

### 📚 Documentation

- Fix and add some documentation + clean up imports
- README - add example sentences for documentations

### ⚙️ Miscellaneous Tasks

- Update deps & bump serde_with to 3.11
- Dbs - fix doc comment
- Fix new lints
- Bump deps and rename defmt feature name
- Update nom parsers and fix CI feature
- Bump MSRV to 1.87
- Update benches-harness
- Update gsa many0 function from nom
- Rust 2024 edition
