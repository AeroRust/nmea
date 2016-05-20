
[NMEA][doc] 0.0.1
====================

[![NMEA on Travis CI][travis-image]][travis] [![Version](https://img.shields.io/crates/v/nmea.svg)](https://crates.io/crates/nmea)
[travis-image]: https://travis-ci.org/flxo/rust-nmea.png
[travis]: https://travis-ci.org/flxo/rust-nmea



NMEA 0183 sentence parser for Rust. 

Currently only _GGA_ sentences are supported.

[Complete Documentation][doc]

[doc]: https://flxo.github.io/rust-nmea/nmea

## Usage

Put this in your `Cargo.toml`:

```toml
[dependencies]
nmea = "0.0.1"
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
