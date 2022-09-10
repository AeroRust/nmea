use std::{
    error::Error,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

use helpers::format_satellites;
use nmea::{parse_str, Nmea};

mod helpers;

#[test]
fn test_parse_file_log() {
    let res = process_file(&Path::new("tests").join("data").join("nmea1.log"))
        .unwrap_or_else(|err| panic!("process file failed with error '{}'", err));

    let expected: Vec<_> = BufReader::new(
        File::open(&Path::new("tests").join("data").join("nmea1.log.expected")).unwrap(),
    )
    .lines()
    .collect::<Result<_, _>>()
    .expect("Should collect lines");

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
    for (i, log_path) in [
        Path::new("tests")
            .join("data")
            .join("nmea_with_sat_info.log"),
        Path::new("tests").join("data").join("nmea1.log"),
        Path::new("tests").join("data").join("nmea2.log"),
    ]
    .iter()
    .enumerate()
    {
        println!("test parsing of {:?}", log_path);
        let full_log = fs::read_to_string(&log_path).unwrap();

        let mut nmea1 = Nmea::default();
        let mut nmea2 = Nmea::default();

        for (line_index, line) in full_log.lines().enumerate() {
            let line_no = line_index + 1;
            if line.starts_with("$GNGRS")
                || line.starts_with("$GNGST")
                || line.starts_with("$GNZDA")
                || line.starts_with("$GNGBS")
            {
                println!(
                    "Ignoring unsupported {} at {:?}:{}",
                    line, log_path, line_no
                );
                continue;
            }

            let expect_msg = format!("Parsing of {} at {:?}:{} failed", line, log_path, line_no);

            parse_str(line).expect(&expect_msg);
            nmea1.parse(line).expect(&expect_msg);
            nmea2.parse_for_fix(line).expect(&expect_msg);
        }

        let sat_state = match i {
            0 => vec![
                "{Beidou 2 Some(11.0) Some(112.0) None}",
                "{Beidou 5 Some(28.0) Some(135.0) None}",
                "{Beidou 7 Some(22.0) Some(49.0) None}",
                "{Beidou 9 Some(2.0) Some(118.0) None}",
                "{Beidou 10 Some(36.0) Some(54.0) Some(18.0)}",
                "{Beidou 11 Some(20.0) Some(75.0) Some(18.0)}",
                "{Beidou 12 Some(5.0) Some(29.0) None}",
                "{Beidou 19 Some(0.0) Some(0.0) None}",
                "{Beidou 20 Some(37.0) Some(296.0) Some(30.0)}",
                "{Beidou 23 Some(66.0) Some(39.0) Some(17.0)}",
                "{Beidou 25 Some(19.0) Some(68.0) None}",
                "{Beidou 28 Some(3.0) Some(153.0) None}",
                "{Galileo 2 Some(56.0) Some(46.0) None}",
                "{Galileo 3 Some(11.0) Some(149.0) None}",
                "{Galileo 7 Some(54.0) Some(298.0) None}",
                "{Galileo 8 Some(60.0) Some(174.0) None}",
                "{Galileo 11 Some(1.0) Some(46.0) None}",
                "{Galileo 25 Some(8.0) Some(52.0) None}",
                "{Galileo 27 Some(11.0) Some(233.0) None}",
                "{Galileo 30 Some(66.0) Some(239.0) None}",
                "{Gps 5 Some(19.0) Some(222.0) Some(19.0)}",
                "{Gps 7 Some(5.0) Some(90.0) Some(18.0)}",
                "{Gps 13 Some(84.0) Some(239.0) Some(23.0)}",
                "{Gps 14 Some(56.0) Some(52.0) Some(16.0)}",
                "{Gps 15 Some(50.0) Some(296.0) Some(30.0)}",
                "{Gps 17 Some(35.0) Some(125.0) Some(24.0)}",
                "{Gps 19 Some(23.0) Some(147.0) None}",
                "{Gps 20 Some(3.0) Some(201.0) None}",
                "{Gps 23 Some(11.0) Some(319.0) Some(23.0)}",
                "{Gps 24 Some(16.0) Some(284.0) Some(25.0)}",
                "{Gps 28 Some(0.0) Some(0.0) Some(19.0)}",
                "{Gps 30 Some(28.0) Some(84.0) Some(20.0)}",
                "{Glonass 68 Some(39.0) Some(185.0) Some(28.0)}",
                "{Glonass 69 Some(63.0) Some(275.0) Some(34.0)}",
                "{Glonass 70 Some(14.0) Some(330.0) Some(22.0)}",
                "{Glonass 79 Some(61.0) Some(298.0) Some(36.0)}",
                "{Glonass 81 Some(0.0) Some(0.0) Some(17.0)}",
                "{Glonass 87 Some(14.0) Some(39.0) None}",
                "{Glonass 88 Some(15.0) Some(82.0) None}",
            ],
            1 => vec![
                "{Gps 1 Some(9.0) Some(74.0) None}",
                "{Gps 8 Some(3.0) Some(29.0) Some(22.0)}",
                "{Gps 10 Some(4.0) Some(350.0) Some(18.0)}",
                "{Gps 11 Some(19.0) Some(59.0) Some(19.0)}",
                "{Gps 13 Some(59.0) Some(220.0) None}",
                "{Gps 15 Some(45.0) Some(281.0) None}",
                "{Gps 17 Some(36.0) Some(151.0) None}",
                "{Gps 18 Some(11.0) Some(323.0) None}",
                "{Gps 19 Some(17.0) Some(170.0) None}",
                "{Gps 20 Some(6.0) Some(258.0) None}",
                "{Gps 24 Some(13.0) Some(288.0) None}",
                "{Gps 28 Some(65.0) Some(71.0) None}",
                "{Gps 30 Some(35.0) Some(109.0) None}",
                "{Glonass 65 Some(24.0) Some(229.0) None}",
                "{Glonass 66 Some(38.0) Some(296.0) None}",
                "{Glonass 67 Some(11.0) Some(347.0) Some(18.0)}",
                "{Glonass 74 Some(35.0) Some(78.0) None}",
                "{Glonass 75 Some(76.0) Some(343.0) None}",
                "{Glonass 76 Some(29.0) Some(279.0) None}",
                "{Glonass 83 Some(13.0) Some(12.0) Some(10.0)}",
                "{Glonass 84 Some(41.0) Some(67.0) None}",
                "{Glonass 85 Some(26.0) Some(132.0) None}",
            ],
            2 => vec![
                "{Gps 2 Some(35.0) Some(291.0) None}",
                "{Gps 3 Some(9.0) Some(129.0) None}",
                "{Gps 5 Some(14.0) Some(305.0) None}",
                "{Gps 6 Some(38.0) Some(226.0) None}",
                "{Gps 7 Some(56.0) Some(177.0) None}",
                "{Gps 9 Some(70.0) Some(67.0) None}",
                "{Gps 16 Some(20.0) Some(55.0) None}",
                "{Gps 23 Some(41.0) Some(76.0) None}",
                "{Gps 26 Some(10.0) Some(30.0) None}",
                "{Gps 29 Some(5.0) Some(341.0) None}",
                "{Gps 30 Some(26.0) Some(199.0) None}",
                "{Gps 36 Some(30.0) Some(158.0) None}",
                "{Gps 49 Some(32.0) Some(192.0) None}",
                "{Glonass 66 Some(45.0) Some(91.0) None}",
                "{Glonass 67 Some(67.0) Some(334.0) None}",
                "{Glonass 68 Some(17.0) Some(297.0) None}",
                "{Glonass 75 Some(13.0) Some(25.0) None}",
                "{Glonass 76 Some(49.0) Some(59.0) None}",
                "{Glonass 77 Some(40.0) Some(156.0) None}",
                "{Glonass 78 Some(0.0) Some(183.0) None}",
                "{Glonass 82 Some(15.0) Some(246.0) None}",
                "{Glonass 83 Some(28.0) Some(298.0) None}",
                "{Glonass 84 Some(10.0) Some(352.0) None}",
            ],
            _ => panic!("You need to add sat state for new log here"),
        };
        assert_eq!(sat_state, format_satellites(nmea1.satellites()));
        assert_eq!(sat_state, format_satellites(nmea2.satellites()));
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
