# [NMEA][doc]

[![Version](https://img.shields.io/crates/v/nmea.svg)](https://crates.io/crates/nmea)
[![Build Status](https://github.com/AeroRust/nmea/actions/workflows/ci.yml/badge.svg)](https://github.com/AeroRust/nmea/actions/workflows/ci.yml)
[![License Apache-2](https://img.shields.io/crates/l/nmea.svg)](./LICENSE.txt)

[Complete documentation can be found on www.docs.rs/nmea][doc]

## NMEA 0183 sentence parser

Supported sentences (alphabetically ordered):


- `AAM` - Waypoint Arrival Alarm (feature: `waypoint`)
- `ALM` - GPS Almanac Data (feature: `GNSS`)
- `APA` - Autopilot Sentence "A" (feature: `GNSS`)
- `BOD` - Bearing - Waypoint to Waypoint (feature: `waypoint`)
- `BWC` - Bearing & Distance to Waypoint - Great Circle (feature: `waypoint`)
- `BWW` - Bearing - Waypoint to Waypoint (feature: `waypoint`)
- `DBK` - Depth Below Keel (feature: `water`)
- `DBS` - Depth Below Surface (feature: `water`)
- `DPT` - Depth of Water (feature: `water`)
- `GBS` - GPS Satellite Fault Detection (feature: `GNSS`)
- `GGA` - * Global Positioning System Fix Data (feature: `GNSS`)
- `GLL` - * Geographic Position - Latitude/Longitude (feature: `GNSS`)
- `GNS` - * Fix data (feature: `GNSS`)
- `GSA` - * GPS DOP and active satellites (feature: `GNSS`)
- `GST` - GPS Pseudorange Noise Statistics (feature: `GNSS`)
- `GSV` - * Satellites in view (feature: `GNSS`)
- `HDT` - Heading - True (feature: `other`)
- `MDA` - Meterological Composite (feature: `other`)
- `MTW` - Mean Temperature of Water (feature: `water`)
- `MWV` - Wind Speed and Angle (feature: `other`)
- `RMC` - * Recommended Minimum Navigation Information (feature: `GNSS`)
- `RMZ` - PGRMZ - Garmin Altitude (feature: `vendor-specific`)
- `TTM` - Tracked target message (feature: `radar`)
- `TXT` - * Text message (feature: `other`)
- `VHW` - Water speed and heading (feature: `water`)
- `VTG` - * Track made good and Ground speed (feature: `GNSS`)
- `WNC` - Distance - Waypoint to waypoint (feature: `waypoint`)
- `ZDA` - Time & Date - UTC, day, month, year and local time zone (feature: `other`)
- `ZFO` - UTC & Time from origin Waypoint (feature: `waypoint`)
- `ZTG` - UTC & Time to Destination Waypoint (feature: `waypoint`)

**\* [`Nmea::parse()`] supported sentences**

[`Nmea::parse()`]: https://docs.rs/nmea/latest/nmea/struct.Nmea.html#method.parse

## How to contribute

We have an ongoing effort to support as many sentences from `NMEA 0183` as possible,
starting with the most well-known.
If you'd like to contribute by writing a parser for a given message, check out the [Supporting additional sentences (AeroRust/nmea#54)](https://github.com/AeroRust/nmea/issues/54) issue and contribute in **3** easy steps:

1. Write a comment in the issue for the sentence(s) you'd like to implement, you will be mentioned on the task to avoid duplicate efforts.
2. Implement each sentence in it's own branch alongside:
   - At least **2 tests** (**1 passing** and **1 failing**) in its own module under the [`./src/sentences`](./src/sentences) directory using the `nom` crate.
   - Re-export the structures and parsing function in [`./src/sentences.rs`](./src/sentences.rs) 
   - Add a **passing** test to [`tests/all_supported_messages.rs`](./tests/all_supported_messages.rs)
   - Add the sentence to the features list in `Cargo.toml` in **alphabetical order** and assign it to proper category (if you are unsure which category to use, open a PR to discuss it)
   - Add the sentence to the `README.md` list of [supported sentences above](./README.md#nmea-0183-sentence-parser-for-rust)
   - Passing linters checks. Just run `cargo fmt` and fix any issues raised by `cargo clippy`
   - Appropriate documentation following the rest of the sentences format. For proper documentation you can take a look at `GSV`, `APA` and `WNC` sentences.
3. Open a PR 🎉

**NB:** We use [https://gpsd.gitlab.io/gpsd/NMEA.html](https://gpsd.gitlab.io/gpsd/NMEA.html) as a reference for most sentences as it's a very well documented project.

## What is NMEA 0183?

> NMEA 0183 is a combined electrical and data specification for communication
> between marine electronics such as echo sounder, sonars, anemometer,
> gyrocompass, autopilot, GPS receivers and many other types of instruments.

- _https://en.wikipedia.org/wiki/NMEA_0183_

[doc]: https://docs.rs/nmea

## Usage

Add the `nmea` dependency in your `Cargo.toml`:

```toml
[dependencies]
nmea = "0.7"
```

### For `no_std`

This crate support `no_std` without the use of an allocator ( `alloc` ),
just add the `nmea` crate without the default features:

```toml
[dependencies]
nmea = { version = "0.7", default-features = false }
```

### Parse

To use the NMEA parser create a `Nmea` struct and feed it with NMEA sentences (only supports `GNSS` messages, otherwise use the `parse_str()` and `parse_bytes()`):

```rust
use nmea::Nmea;

fn main() {
    let mut nmea = Nmea::default();
    let gga = "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76";

    // feature `GGA` should be enabled to parse this sentence.
    #[cfg(feature = "GGA")]
    {
        nmea.parse(gga).unwrap();
        println!("{}", nmea);
    }
}
```

## Supported Rust Versions

The Minimum supported Rust version (or MSRV) is **1.87.0**.

## Unsafe-free crate

We use `#![deny(unsafe_code)]` for a fully `unsafe`-free crate.

## License

This project is licensed under the [Apache-2.0](./LICENSE.txt).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the project by you, shall be licensed as Apache-2.0,
without any additional terms or conditions.
