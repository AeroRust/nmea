/*
 * Copyright (C) 2016 Felix Obenhuber
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

extern crate regex;

use regex::Regex;
use std::fmt;

pub struct Nmea {
    latitude: Option<f32>,
    longitude: Option<f32>,
    altitude: Option<f32>,

    regex_checksum: Regex,
    regex_type: Regex,
    regex_gga: Regex,
}

impl Nmea {
    pub fn new() -> Nmea {
        Nmea {
            latitude: None,
            longitude: None,
            altitude: None,
            regex_checksum: Regex::new(r"^\$(?P<sentence>.*)\*(?P<checksum>..)$").unwrap(),
            regex_type: Regex::new(r"^\$\D{2}(?P<type>\D{3}).*$").unwrap(),
            // $GNGGA,101240.00,4807.48553,N,01133.05879,E,1,12,0.64,533.8,M,46.3,M,,*4E
            regex_gga: Regex::new(r"^\$\D\DGGA,(?P<timestamp>\d+\.\d+),(?P<lat>\d+\.\d+),(?P<lat_dir>[NS]),(?P<lon>\d+\.\d+),(?P<lon_dir>[WE]),(\d),(\d+),(\d+\.\d+),(?P<alt>\d+\.\d+),\D,(\d+\.\d+),\D,,\*([0-9a-fA-F][0-9a-fA-F])").unwrap(),
        }
    }

    pub fn latitude(&self) -> Option<f32> {
        self.latitude
    }

    pub fn longitude(&self) -> Option<f32> {
        self.longitude
    }

    pub fn altitude(&self) -> Option<f32> {
        self.altitude
    }

    fn checksum(&self, s: &str) -> Result<bool, &'static str> {
        let caps = match self.regex_checksum.captures(s) {
            Some(c) => c,
            None => return Err("Checksum parsing failed"),
        };
        let sentence = match caps.name(&"sentence") {
            Some(v) => v,
            None => return Err("Checksum parsing failed"),
        };
        let checksum = match u8::from_str_radix(caps.name(&"checksum").unwrap_or(""), 16) {
            Ok(v) => v,
            Err(_) => return Err("Checksum parsing failed"),
        };
        Ok(sentence.bytes().fold(0, |c, x| c ^ x) == checksum)
    }

    pub fn sentence_type(&self, s: &str) -> Result<Type, &'static str> {
        match self.regex_type.captures(s) {
            Some(c) => match c.name("type") {
                Some(s) => match Type::from(s) {
                    Type::None => Err("Unknown type"),
                    _ => Ok(Type::from(s)),
                },
                _ => Err("Failed to parse type"),
            },
            None => Err("Failed to parse type")
        }
    }

    fn parse_numeric<T>(input: Option<&str>, factor: T) -> Option<T> where T: std::str::FromStr + std::ops::Mul<Output=T> + Copy {
        match input {
            Some(s) => {
                match s.parse::<T>() {
                    Ok(v) => Some(v * factor),
                    Err(_) => None,
                }
            },
            None => None,
        }
    }

    pub fn parse(&mut self, s: &str) -> Result<Type, &'static str> {
        match self.checksum(s) {
            Ok(_) => (),
            Err(e) => return Err(e),
        }

        match self.sentence_type(s) {
            Ok(t) => {
                match t {
                    Type::GGA => {
                        match self.regex_gga.captures(s) {
                            Some(caps) => {
                                self.latitude = Self::parse_numeric::<f32>(caps.name("lat"), 0.01);
                                self.longitude= Self::parse_numeric::<f32>(caps.name("lon"), 0.01);
                                self.altitude = Self::parse_numeric::<f32>(caps.name("alt"), 1.0);
                                return Ok(Type::GGA)
                            },
                            None => return Err("Failed to parse GGA sentence"),
                        }
                    },
                    _ => Err("Unknown or implemented type"),
                }
            },
            Err(e) => Err(e),
        }
    }
}

impl fmt::Debug for Nmea {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "lat: {:3.3} lon: {:3.3} alt: {:5.3}",
               self.latitude.unwrap_or(0.0),
               self.longitude.unwrap_or(0.0),
               self.altitude.unwrap_or(0.0))
    }
}

impl fmt::Display for Nmea {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(PartialEq, Debug)]
pub enum Type {
    None,
    GGA,
}

impl<'a> From<&'a str> for Type {
    fn from(s: &str) -> Self {
        match s {
            "GGA" => Type::GGA,
            _ => Type::None,
        }
    }
}

#[test]
fn test_parse_numeric() {
    assert_eq!(Nmea::parse_numeric::<f32>(Some("123.1"), 1.0), Some(123.1));
    assert_eq!(Nmea::parse_numeric::<f32>(Some("123.a"), 1.0), None);
    assert_eq!(Nmea::parse_numeric::<f32>(Some("100.1"), 2.0), Some(200.2));
    assert_eq!(Nmea::parse_numeric::<f32>(Some("-10.0"), 1.0), Some(-10.0));
    assert_eq!(Nmea::parse_numeric::<f64>(Some("123.1"), 1.0), Some(123.1));
    assert_eq!(Nmea::parse_numeric::<f64>(Some("123.a"), 1.0), None);
    assert_eq!(Nmea::parse_numeric::<f64>(Some("100.1"), 2.0), Some(200.2));
    assert_eq!(Nmea::parse_numeric::<f64>(Some("-10.0"), 1.0), Some(-10.0));
    assert_eq!(Nmea::parse_numeric::<i32>(Some("0"), 0), Some(0));
    assert_eq!(Nmea::parse_numeric::<i32>(Some("-10"), 1), Some(-10));
    assert_eq!(Nmea::parse_numeric::<u32>(Some("0"), 0), Some(0));
    assert_eq!(Nmea::parse_numeric::<u32>(Some("-1"), 0), None);
    assert_eq!(Nmea::parse_numeric::<i8>(Some("0"), 0), Some(0));
    assert_eq!(Nmea::parse_numeric::<i8>(Some("-10"), 1), Some(-10));
    assert_eq!(Nmea::parse_numeric::<u8>(Some("0"), 0), Some(0));
    assert_eq!(Nmea::parse_numeric::<u8>(Some("-1"), 0), None);
}

#[test]
fn test_checksum() {
    let nmea = Nmea::new();
    let valid = "$GNGSA,A,1,,,,,,,,,,,,,99.99,99.99,99.99*2E";
    let invalid = "$GNZDA,165118.00,13,05,2016,00,00*71";
    assert_eq!(nmea.checksum(valid).unwrap(), true);
    assert_eq!(nmea.checksum(invalid).unwrap(), false);
}

#[test]
fn test_message_type() {
    let nmea = Nmea::new();
    let gga = "$GPGGA,A,1,,,,,,,,,,,,,99.99,99.99,99.99*2E";
    let fail = "$GPXXX,A,1,,,,,,,,,,,,,99.99,99.99,99.99*2E";
    assert_eq!(nmea.sentence_type(gga).unwrap(), Type::GGA);
    assert!(nmea.sentence_type(fail).is_err());
}

#[test]
fn test_parse() {
    // sample NMEA sentences from https://en.wikipedia.org/wiki/NMEA_0183
    let sentences = vec![
        "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76",
        "$GPGSA,A,3,10,07,05,02,29,04,08,13,,,,,1.72,1.03,1.38*0A",
        "$GPGSV,3,1,11,10,63,137,17,07,61,098,15,05,59,290,20,08,54,157,30*70",
        "$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79",
        "$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76",
        "$GPRMC,092750.000,A,5321.6802,N,00630.3372,W,0.02,31.66,280511,,,A*43",
        "$GPGGA,092751.000,5321.6802,N,00630.3371,W,1,8,1.03,61.7,M,55.3,M,,*75",
        "$GPGSA,A,3,10,07,05,02,29,04,08,13,,,,,1.72,1.03,1.38*0A",
        "$GPGSV,3,1,11,10,63,137,17,07,61,098,15,05,59,290,20,08,54,157,30*70",
        "$GPGSV,3,2,11,02,39,223,16,13,28,070,17,26,23,252,,04,14,186,15*77",
        "$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76",
        "$GPRMC,092751.000,A,5321.6802,N,00630.3371,W,0.06,31.66,280511,,,A*45",
    ];

    let mut nmea = Nmea::new();
    for s in &sentences {
        nmea.parse(s).ok();
    }

    assert_eq!(nmea.latitude().unwrap(), 53.216802);
    assert_eq!(nmea.longitude().unwrap(), 6.303371);
    assert_eq!(nmea.altitude().unwrap(), 61.7);
}
