use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

fn err_to_string<E: Error>(e: E) -> String {
    e.to_string()
}

fn process_file(n: &Path) -> Result<Vec<String>, String> {
    let input = BufReader::new(File::open(n).map_err(err_to_string)?);
    let mut nmea = nmea::Nmea::new();
    let mut ret = Vec::with_capacity(15_000);
    for (num, line) in input.lines().enumerate() {
        let line = line
            .map_err(err_to_string)
            .map_err(|s| format!("{} at line {}", s, num + 1))?;
        let parse_res = nmea
            .parse(&line)
            .map_err(|s| format!("{} at line {}", s, num + 1))?;
        ret.push(format!("{:?}", parse_res));
    }
    Ok(ret)
}

#[test]
fn test_parse_file_log() {
    let res = process_file(&Path::new("tests").join("nmea1.log"))
        .unwrap_or_else(|err| panic!("process file failed with error '{}'", err));

    let expected: Vec<_> =
        BufReader::new(File::open(&Path::new("tests").join("nmea1.log.expected")).unwrap())
            .lines()
            .map(|v| v.unwrap())
            .collect();
    assert_eq!(expected, res);
}

#[test]
fn test_parse_issue_2() {
    let mut input = BufReader::new(File::open(&Path::new("tests").join("nmea2.log")).unwrap());
    let mut nmea = nmea::Nmea::new();
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
