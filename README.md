# [NMEA][doc]

[![Version](https://img.shields.io/crates/v/nmea.svg)](https://crates.io/crates/nmea)
[![Build Status](https://github.com/AeroRust/nmea/actions/workflows/ci.yml/badge.svg)](https://github.com/AeroRust/nmea/actions/workflows/ci.yml)
[![License Apache-2](https://img.shields.io/crates/l/nmea.svg)](./LICENSE-APACHE)

[Complete documentation can be found on www.docs.rs/nmea][doc]

## NMEA 0183 sentence parser for Rust.

Supported sentences:

NMEA Standard Sentences
- AAM
- ALM
- BOD
- BWC
- GBS
- GGA *
- GLL *
- GNS *
- GSA *
- GSV *
- MDA
- MTW
- MWV
- RMC *
- VTG *

Other Sentences
- TXT *

Vendor Extensions
- PGRMZ

**\* [`Nmea::parse()`] supported sentences**

[`Nmea::parse()`]: https://docs.rs/nmea/latest/nmea/struct.Nmea.html#method.parse

## How to contribute

We have an ongoing effort to support as many sentences from `NMEA 0183` as possible,
starting with the most well-known.
If you'd like to contribute by writing a parser for a given message, check out the [Supporting additional sentences (AeroRust/nmea#54)](https://github.com/AeroRust/nmea/issues/54) issue and contribute in **3** easy steps:

1. Write a comment - Please write a comment in the issue for the sentence(s) you'd like to implement, you will be mentioned on the task to avoid duplicate implementations.
2. Implement each sentence alongside at least 1 test in its own module under the [`./src/sentences`](./src/sentences) directory using the `nom` crate.
3. Open a PR 🎉

## What is NMEA 0183?

> NMEA 0183 is a combined electrical and data specification for communication
> between marine electronics such as echo sounder, sonars, anemometer,
> gyrocompass, autopilot, GPS receivers and many other types of instruments.
>
- _https://en.wikipedia.org/wiki/NMEA_0183_


[doc]: https://docs.rs/nmea

## Usage

Add the `nmea` dependency in your `Cargo.toml`:

```toml
[dependencies]
nmea = "0.4"
```

### For `no_std`

This crate support `no_std` without the use of an allocator ( `alloc` ),
just add the `nmea` crate without the default features:

```toml
[dependencies]
nmea = {version = "0.4", default-features = false}
```

### For Rust edition 2015

For Rust 2015 edition you should put this in your crate's `lib.rs` or `main.rs`:

```rust
extern crate nmea;
```

To use the NMEA parser create a `Nmea` struct and feed it with NMEA sentences (only supports `GNSS` messages, otherwise use the `parse_str()` and `parse_bytes()`):

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

The Minimum supported Rust version (or MSRV) is **1.59**.

## Unsafe-free crate

We use `#![deny(unsafe_code)]` for a fully `unsafe`-free crate.

## License

This project is licensed under the [Apache-2.0](./LICENSE.txt).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the project by you, shall be licensed as Apache-2.0,
without any additional terms or conditions.
