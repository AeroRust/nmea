[NMEA][doc]
===========

[![Version](https://img.shields.io/crates/v/nmea.svg)](https://crates.io/crates/nmea)
[![Build Status](https://github.com/AeroRust/nmea/workflows/CI/badge.svg)](https://github.com/AeroRust/nmea/actions?query=workflow%3ACI+branch%3Amaster)
[![codecov](https://codecov.io/gh/AeroRust/nmea/branch/master/graph/badge.svg)](https://codecov.io/gh/rusqlite/rusqlite)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/Dushistov/rust-nmea/blob/master/LICENSE.txt)

**[Full Documentation][doc]**

`nmea` is an NMEA 0183 parser made in Rust. Currently only `GGA`, `GSV`, `GSA`,
`VTG` and `RMC` sentences are supported. Feel free to add others.

The current units this crate is tested to handle are:

- Degrees
- Knots
- Meters (of altitude)

## Usage

Put this in your `Cargo.toml`:

```toml
[dependencies]
nmea = "0.0.7"
```

Then you can import and feed it nmea sentances!

```rust
use nmea::Nmea;

let mut nmea = Nmea::new();
let gga = "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76";

nmea.parse(gga).unwrap();

println!("{}", nmea);
```
