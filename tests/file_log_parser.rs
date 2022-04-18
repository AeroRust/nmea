use std::{
    error::Error,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

use nmea::{parse, Nmea};

#[test]
fn test_parse_file_log() {
    let res = process_file(&Path::new("tests").join("data").join("nmea1.log"))
        .unwrap_or_else(|err| panic!("process file failed with error '{}'", err));

    let expected: Vec<_> = BufReader::new(
        File::open(&Path::new("tests").join("data").join("nmea1.log.expected")).unwrap(),
    )
    .lines()
    .map(|v| v.unwrap())
    .collect();
    assert_eq!(expected, res);
}

#[test]
fn test_parse_issue_2() {
    let mut input =
        BufReader::new(File::open(&Path::new("tests").join("data").join("nmea2.log")).unwrap());
    let mut nmea = Nmea::default();
    for _ in 0..100 {
        let mut buffer = String::new();
        let size = input.read_line(&mut buffer).unwrap();
        eprintln!("buffer = {:?}", buffer);
        if size > 0 {
            if buffer.as_bytes()[0] == b'$' {
                let _ = nmea.parse(&buffer);
                println!("{:?}", nmea);
            }
        } else {
            break;
        }
    }
}

#[test]
fn test_parse_all_logs() {
    for log_path in [
        Path::new("tests")
            .join("data")
            .join("nmea_with_sat_info.log"),
        Path::new("tests").join("data").join("nmea1.log"),
        Path::new("tests").join("data").join("nmea2.log"),
    ] {
        println!("test parsing of {log_path:?}");
        let full_log = fs::read_to_string(&log_path).unwrap();

        let mut nmea1 = Nmea::default();
        let mut nmea2 = Nmea::default();

        for (line_no, line) in full_log.lines().enumerate() {
            if line.starts_with("$GNGNS")
                || line.starts_with("$GNGRS")
                || line.starts_with("$GNGST")
                || line.starts_with("$GNZDA")
                || line.starts_with("$GNGBS")
            {
                println!("Ignroing unsupported {line} at {log_path:?}:{line_no}");
                continue;
            }
            let s = line.as_bytes();

            macro_rules! err_handler {
                () => {
                    |err| {
                        panic!(
                            "Parsing of {line} at {log_path:?}:{line_no} failed: {err}",
                            line_no = line_no + 1
                        )
                    }
                };
            }
            parse(s).unwrap_or_else(err_handler!());
            nmea1.parse(line).unwrap_or_else(err_handler!());
            nmea2.parse_for_fix(s).unwrap_or_else(err_handler!());
        }
    }
}

fn err_to_string<E: Error>(e: E) -> String {
    e.to_string()
}

fn process_file(n: &Path) -> Result<Vec<String>, String> {
    let input = BufReader::new(File::open(n).map_err(err_to_string)?);
    let mut nmea = nmea::Nmea::default();
    let mut ret = Vec::with_capacity(15_000);
    for (num, line) in input.lines().enumerate() {
        let line = line
            .map_err(err_to_string)
            .map_err(|s| format!("{} at line {}", s, num + 1))?;
        let parse_res = nmea
            .parse(&line)
            .map_err(|s| format!("{:?} at line {}", s, num + 1))?;
        ret.push(format!("{:?}", parse_res));
    }
    Ok(ret)
}
