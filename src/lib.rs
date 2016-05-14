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
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn latitude(&self) -> Option<f32> {
        self.latitude
    }
    #[allow(dead_code)]
    pub fn longitude(&self) -> Option<f32> {
        self.longitude
    }
    #[allow(dead_code)]
    pub fn altitude(&self) -> Option<f32> {
        self.altitude
    }

    #[allow(dead_code)]
    fn checksum(&self, s: &str) -> bool {
        let caps = match self.regex_checksum.captures(s) {
            Some(c) => c,
            None => return false,
        };

        let sentence = match caps.name(&"sentence") {
            Some(v) => v,
            None => return false,
        };
        let checksum = match u8::from_str_radix(caps.name(&"checksum").unwrap_or(""), 16) {
            Ok(v) => v,
            Err(_) => return false,
        };

        sentence.bytes().fold(0, |c, x| c ^ x) == checksum
    }

    #[allow(dead_code)]
    pub fn parse_type(&self, s: &str) -> Type {
        match self.regex_type.captures(s) {
            Some(c) => match c.name("type") {
                Some(s) => Type::from(s),
                _ => Type::None,
            },
            None => Type::None,
        }
    }

    #[allow(dead_code)]
    pub fn parse(&mut self, s: &str) {
        if !self.checksum(s) {
            return;
        }


        let cap_to_f32 = |c: Option<&str>, f: f32 | -> Option<f32> {
            match c {
                Some(e) => match e.parse::<f32>() {
                    Ok(v) => Some(v * f),
                    Err(_) => None,
                },
                None => None,
            }
        };

        match self.parse_type(s) {
            Type::GGA => {
                match self.regex_gga.captures(s) {
                    Some(caps) => {
                        self.latitude = cap_to_f32(caps.name("lat"), 0.01);
                        self.longitude = cap_to_f32(caps.name("lon"), 0.01);
                        self.altitude = cap_to_f32(caps.name("alt"), 1.0);
                    },
                    None => return,
                }
            },
            Type::None => return,
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

#[test]
fn test_checksum() {
    let nmea = Nmea::new();
    let valid = "$GNGSA,A,1,,,,,,,,,,,,,99.99,99.99,99.99*2E";
    let invalid = "$GNZDA,165118.00,13,05,2016,00,00*71";
    assert_eq!(nmea.checksum(valid), true);
    assert_eq!(nmea.checksum(invalid), false);
}

#[test]
fn test_message_type() {
    let nmea = Nmea::new();
    let gga = "$GPGGA,A,1,,,,,,,,,,,,,99.99,99.99,99.99*2E";
    assert_eq!(nmea.parse_type(gga), Type::GGA);
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
fn test_parse() {
    let nmea = Nmea::new();

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

    for s in &sentences {
        nmea.parse(s);
    }

    assert_eq!(nmea.latitude().unwrap(), 53.216802);
    assert_eq!(nmea.longitude().unwrap(), 63.03372);
    assert_eq!(nmea.altitude().unwrap(), 61.7);

}
