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

extern crate nmea;

use std::io::{BufRead, BufReader};
use std::fs::File;
use std::env;

fn main() {
    let mut nmea = nmea::Nmea::new();
    let file = match env::args().nth(1) {
        Some(f) => f,
        None => panic!("No file argument"),
    };

    println!("Reading {}", file);

    let f = match File::open(file) {
        Ok(f) => f,
        Err(e) => panic!("{}", e),
    };

    let mut input = BufReader::new(f);
    let mut buffer = "".to_string();

    let mut loops = 0;
    loop {
        match input.read_line(&mut buffer) {
            Ok(s) => if s > 0 {
                for l in buffer.lines() {
                    nmea.parse(&l).ok();
                }
                if loops % 100 == 0 { println!("{:?}", nmea); }
                loops += 1;
            } else {
                println!("{:?}", nmea);
                return
            },
            Err(e) => panic!("{}", e),
        };
        buffer.clear();
    }
}
