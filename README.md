# [NMEA][doc]

[![Version](https://img.shields.io/crates/v/nmea.svg)](https://crates.io/crates/nmea)
[![Build Status](https://github.com/AeroRust/nmea/workflows/CI/badge.svg)](https://github.com/AeroRust/nmea/actions?query=workflow%3ACI+branch%3Amaster)
[![codecov](https://codecov.io/gh/AeroRust/nmea/branch/master/graph/badge.svg)](https://codecov.io/gh/AeroRust/nmea)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/AeroRust/nmea/blob/master/LICENSE.txt)

[Complete documentation can be found on www.docs.rs/nmea][doc]

NMEA 0183 sentence parser for Rust.

Supported sentences:
- BOD (untested)
- BWC (supported by `parse()`, but not by `Nmea::parse()`)
- GGA
- GLL
- GNS
- GSA
- GSV
- RMC
- TXT
- VTG

Feel free to open PR and add others.

> NMEA 0183 is a combined electrical and data specification for communication
> between marine electronics such as echo sounder, sonars, anemometer,
> gyrocompass, autopilot, GPS receivers and many other types of instruments.
>
- _https://en.wikipedia.org/wiki/NMEA_0183_


[doc]: https://docs.rs/nmea

## Usage

Put this in your `Cargo.toml`:

```toml
[dependencies]
nmea = "0.3"
```

For Rust 2015 edition put this in your crate root:

```rust
extern crate nmea;
```

To use the NMEA parser create a `Nmea` struct and feed it with NMEA sentences:

```rust
use nmea::Nmea;

fn main() {
    let mut nmea = Nmea::default();
    let gga = "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76";
    
    nmea.parse(gga).unwrap();
    println!("{}", nmea);
}
```

## Supported Rust Versions

The Minimum supported Rust version (or MSRV) is **1.56**.

## Unsafe-free crate

We use `#![deny(unsafe_code)]` for a fully `unsafe`-free crate.

## License

This project is licensed under the [Apache-2.0](./LICENSE.txt).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the project by you, shall be licensed as Apache-2.0,
without any additional terms or conditions.
