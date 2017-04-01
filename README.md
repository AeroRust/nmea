[NMEA][doc]
===========

[![Version](https://img.shields.io/crates/v/nmea.svg)](https://crates.io/crates/nmea)
[![Build Status](https://travis-ci.org/Dushistov/rust-nmea.png)](https://travis-ci.org/Dushistov/rust-nmea)

NMEA 0183 sentence parser for Rust. 

Currently only _GGA_,_GSV_, _GSA_, _VTG_ and _RMC_ sentences are supported. Feel free to add others.

[Complete Documentation][doc]

[doc]: https://docs.rs/nmea/

## Usage

Put this in your `Cargo.toml`:

```toml
[dependencies]
nmea = "0.0.4"
```

And put this in your crate root:

```rust
extern crate nmea;
```

To use the NMEA parser create a Nmea struct and feed it with NMEA sentences:

```rust
use nmea::Nmea;

let mut nmea = Nmea::new();
let gga = "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76";
nmea.parse(gga).ok();
println!("{}", nmea);
```
