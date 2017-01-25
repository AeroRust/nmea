// Copyright (C) 2016 Felix Obenhuber
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

extern crate nmea;

use std::io::{BufRead, BufReader};
use std::fs::File;
use std::env;

fn main() {
    run().unwrap();
}

fn run() -> Result<(), String> {
    let mut nmea = nmea::Nmea::new();

    let file = env::args().nth(1).ok_or("Invalid arguments".to_owned())?;
    let mut input = BufReader::new(File::open(file).map_err(|e| format!("{}", e))?);

    for _ in 0..100 {
        let mut buffer = String::new();
        let size = input.read_line(&mut buffer).map_err(|e| format!("{}", e))?;
        if size > 0 {
            nmea.parse(&buffer)?;
            println!("{:?}", nmea);
        } else {
            break;
        }
    }
    Ok(())
}
