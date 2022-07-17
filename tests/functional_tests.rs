mod file_log_parser;

use approx::assert_relative_eq;
use chrono::NaiveTime;
use heapless::Vec;
use nmea::*;

#[test]
fn test_invalid_datetime() {
    let mut nmea = Nmea::default();
    let res = nmea.parse("$,GRMC,,A,,,,,,,290290GLCR*40");
    println!("parse result {:?}", res);
    assert!(matches!(res, Err(NmeaError::ParsingError(_))));
}

#[test]
fn test_gga_north_west() {
    use chrono::Timelike;
    let mut nmea = Nmea::default();
    nmea.parse("$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76")
        .unwrap();
    assert_eq!(nmea.fix_timestamp().unwrap().second(), 50);
    assert_eq!(nmea.fix_timestamp().unwrap().minute(), 27);
    assert_eq!(nmea.fix_timestamp().unwrap().hour(), 9);
    assert_eq!(nmea.latitude().unwrap(), 53. + 21.6802 / 60.);
    assert_eq!(nmea.longitude().unwrap(), -(6. + 30.3372 / 60.));
    assert_eq!(nmea.fix_type().unwrap(), FixType::Gps);
    assert_eq!(nmea.fix_satellites().unwrap(), 8);
    assert_eq!(nmea.hdop().unwrap(), 1.03);
    assert_relative_eq!(nmea.geoid_altitude().unwrap(), (61.7 + 55.2));
}

#[test]
fn test_gga_north_east() {
    let mut nmea = Nmea::default();
    nmea.parse("$GPGGA,092750.000,5321.6802,N,00630.3372,E,1,8,1.03,61.7,M,55.2,M,,*64")
        .unwrap();
    assert_eq!(nmea.latitude().unwrap(), 53. + 21.6802 / 60.);
    assert_eq!(nmea.longitude().unwrap(), 6. + 30.3372 / 60.);
}

#[test]
fn test_gga_south_west() {
    let mut nmea = Nmea::default();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*6B")
        .unwrap();
    assert_eq!(nmea.latitude().unwrap(), -(53. + 21.6802 / 60.));
    assert_eq!(nmea.longitude().unwrap(), -(6. + 30.3372 / 60.));
}

#[test]
fn test_gga_south_east() {
    let mut nmea = Nmea::default();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,E,1,8,1.03,61.7,M,55.2,M,,*79")
        .unwrap();
    assert_eq!(nmea.latitude().unwrap(), -(53. + 21.6802 / 60.));
    assert_eq!(nmea.longitude().unwrap(), 6. + 30.3372 / 60.);
}

#[test]
fn test_gga_invalid() {
    let mut nmea = Nmea::default();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,E,0,8,1.03,61.7,M,55.2,M,,*7B")
        .unwrap_err();
    assert_eq!(nmea.fix_type(), None);
}

#[test]
fn test_gga_gps() {
    use chrono::Timelike;
    let mut nmea = Nmea::default();
    nmea.parse("$GPGGA,092750.000,5321.6802,S,00630.3372,E,1,8,1.03,61.7,M,55.2,M,,*79")
        .unwrap();
    assert_eq!(nmea.fix_timestamp().unwrap().second(), 50);
    assert_eq!(nmea.fix_timestamp().unwrap().minute(), 27);
    assert_eq!(nmea.fix_timestamp().unwrap().hour(), 9);
    assert_eq!(-(53. + 21.6802 / 60.), nmea.latitude.unwrap());
    assert_eq!(6. + 30.3372 / 60., nmea.longitude.unwrap());
    assert_eq!(nmea.fix_type(), Some(FixType::Gps));
    assert_eq!(8, nmea.num_of_fix_satellites.unwrap());
    assert_eq!(1.03, nmea.hdop.unwrap());
    assert_eq!(61.7, nmea.altitude.unwrap());
    assert_eq!(55.2, nmea.geoid_separation.unwrap());
}

#[test]
fn test_gsv() {
    let mut nmea = Nmea::default();

    //                        10           07           05           08
    nmea.parse("$GPGSV,3,1,11,10,63,137,17,07,61,098,15,05,59,290,20,08,54,157,30*70")
        .unwrap();
    pretty_assertions::assert_eq!(
        vec![
            "{Gps 5 Some(59.0) Some(290.0) Some(20.0)}",
            "{Gps 7 Some(61.0) Some(98.0) Some(15.0)}",
            "{Gps 8 Some(54.0) Some(157.0) Some(30.0)}",
            "{Gps 10 Some(63.0) Some(137.0) Some(17.0)}",
        ],
        format_satellites(nmea.satellites())
    );
    //                        02           13           26         04
    nmea.parse("$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79")
        .unwrap();
    pretty_assertions::assert_eq!(
        vec![
            "{Gps 2 Some(39.0) Some(223.0) Some(19.0)}",
            "{Gps 4 Some(14.0) Some(186.0) Some(14.0)}",
            "{Gps 5 Some(59.0) Some(290.0) Some(20.0)}",
            "{Gps 7 Some(61.0) Some(98.0) Some(15.0)}",
            "{Gps 8 Some(54.0) Some(157.0) Some(30.0)}",
            "{Gps 10 Some(63.0) Some(137.0) Some(17.0)}",
            "{Gps 13 Some(28.0) Some(70.0) Some(17.0)}",
            "{Gps 26 Some(23.0) Some(252.0) None}",
        ],
        format_satellites(nmea.satellites())
    );
    //                        29           16         36
    nmea.parse("$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76")
        .unwrap();
    pretty_assertions::assert_eq!(
        vec![
            "{Gps 2 Some(39.0) Some(223.0) Some(19.0)}",
            "{Gps 4 Some(14.0) Some(186.0) Some(14.0)}",
            "{Gps 5 Some(59.0) Some(290.0) Some(20.0)}",
            "{Gps 7 Some(61.0) Some(98.0) Some(15.0)}",
            "{Gps 8 Some(54.0) Some(157.0) Some(30.0)}",
            "{Gps 10 Some(63.0) Some(137.0) Some(17.0)}",
            "{Gps 13 Some(28.0) Some(70.0) Some(17.0)}",
            "{Gps 16 Some(9.0) Some(20.0) None}",
            "{Gps 26 Some(23.0) Some(252.0) None}",
            "{Gps 29 Some(9.0) Some(301.0) Some(24.0)}",
            "{Gps 36 None None None}"
        ],
        format_satellites(nmea.satellites())
    );
}

#[test]
fn test_gsv_real_data() {
    let mut nmea = Nmea::default();
    let real_data = [
        // GPS
        //             1             3            6           11
        "$GPGSV,3,1,12,01,49,196,41,03,71,278,32,06,02,323,27,11,21,196,39*72",
        // GPS
        //             14           17           19           22
        "$GPGSV,3,2,12,14,39,063,33,17,21,292,30,19,20,310,31,22,82,181,36*73",
        // GPS
        //             23           25           31           32
        "$GPGSV,3,3,12,23,34,232,42,25,11,045,33,31,45,092,38,32,14,061,39*75",
        "$GLGSV,3,1,10,74,40,078,43,66,23,275,31,82,10,347,36,73,15,015,38*6B",
        "$GLGSV,3,2,10,75,19,135,36,65,76,333,31,88,32,233,33,81,40,302,38*6A",
        "$GLGSV,3,3,10,72,40,075,43,87,00,000,*6F",
        //             26         31         32
        "$GPGSV,4,4,15,26,02,112,,31,45,071,,32,01,066,*4C",
    ];
    for line in &real_data {
        pretty_assertions::assert_eq!(nmea.parse(line).unwrap(), SentenceType::GSV);
    }
    pretty_assertions::assert_eq!(
        vec![
            "{Gps 1 Some(49.0) Some(196.0) Some(41.0)}",
            "{Gps 3 Some(71.0) Some(278.0) Some(32.0)}",
            "{Gps 6 Some(2.0) Some(323.0) Some(27.0)}",
            "{Gps 11 Some(21.0) Some(196.0) Some(39.0)}",
            "{Gps 14 Some(39.0) Some(63.0) Some(33.0)}",
            "{Gps 17 Some(21.0) Some(292.0) Some(30.0)}",
            "{Gps 19 Some(20.0) Some(310.0) Some(31.0)}",
            "{Gps 22 Some(82.0) Some(181.0) Some(36.0)}",
            "{Gps 23 Some(34.0) Some(232.0) Some(42.0)}",
            "{Gps 25 Some(11.0) Some(45.0) Some(33.0)}",
            "{Gps 26 Some(2.0) Some(112.0) None}",
            "{Gps 31 Some(45.0) Some(71.0) None}",
            "{Gps 32 Some(1.0) Some(66.0) None}",
            "{Glonass 65 Some(76.0) Some(333.0) Some(31.0)}",
            "{Glonass 66 Some(23.0) Some(275.0) Some(31.0)}",
            "{Glonass 72 Some(40.0) Some(75.0) Some(43.0)}",
            "{Glonass 73 Some(15.0) Some(15.0) Some(38.0)}",
            "{Glonass 74 Some(40.0) Some(78.0) Some(43.0)}",
            "{Glonass 75 Some(19.0) Some(135.0) Some(36.0)}",
            "{Glonass 81 Some(40.0) Some(302.0) Some(38.0)}",
            "{Glonass 82 Some(10.0) Some(347.0) Some(36.0)}",
            "{Glonass 87 Some(0.0) Some(0.0) None}",
            "{Glonass 88 Some(32.0) Some(233.0) Some(33.0)}",
        ],
        format_satellites(nmea.satellites())
    );
}

#[test]
fn test_gsv_order() {
    let mut nmea = Nmea::default();
    //                         2           13           26         04
    nmea.parse("$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79")
        .unwrap();
    //                        29           16         36
    nmea.parse("$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76")
        .unwrap();
    //                        10           07           05           08
    nmea.parse("$GPGSV,3,1,11,10,63,137,17,07,61,098,15,05,59,290,20,08,54,157,30*70")
        .unwrap();
    assert_eq!(
        vec![
            "{Gps 2 Some(39.0) Some(223.0) Some(19.0)}",
            "{Gps 4 Some(14.0) Some(186.0) Some(14.0)}",
            "{Gps 5 Some(59.0) Some(290.0) Some(20.0)}",
            "{Gps 7 Some(61.0) Some(98.0) Some(15.0)}",
            "{Gps 8 Some(54.0) Some(157.0) Some(30.0)}",
            "{Gps 10 Some(63.0) Some(137.0) Some(17.0)}",
            "{Gps 13 Some(28.0) Some(70.0) Some(17.0)}",
            "{Gps 16 Some(9.0) Some(20.0) None}",
            "{Gps 26 Some(23.0) Some(252.0) None}",
            "{Gps 29 Some(9.0) Some(301.0) Some(24.0)}",
            "{Gps 36 None None None}",
        ],
        format_satellites(nmea.satellites())
    );
}

#[test]
fn test_gsv_two_of_three() {
    let mut nmea = Nmea::default();
    //                         2           13           26          4
    nmea.parse("$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79")
        .unwrap();
    //                        29           16         36
    nmea.parse("$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76")
        .unwrap();
    assert_eq!(
        vec![
            "{Gps 2 Some(39.0) Some(223.0) Some(19.0)}",
            "{Gps 4 Some(14.0) Some(186.0) Some(14.0)}",
            "{Gps 13 Some(28.0) Some(70.0) Some(17.0)}",
            "{Gps 16 Some(9.0) Some(20.0) None}",
            "{Gps 26 Some(23.0) Some(252.0) None}",
            "{Gps 29 Some(9.0) Some(301.0) Some(24.0)}",
            "{Gps 36 None None None}",
        ],
        format_satellites(nmea.satellites())
    );
}

#[test]
fn test_parse() {
    let sentences = [
        "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76",
        "$GPGSA,A,3,10,07,05,02,29,04,08,13,,,,,1.72,1.03,1.38*0A",
        "$GPGSV,3,1,11,10,63,137,17,07,61,098,15,05,59,290,20,08,54,157,30*70",
        "$GPGSV,3,2,11,02,39,223,19,13,28,070,17,26,23,252,,04,14,186,14*79",
        "$GPGSV,3,3,11,29,09,301,24,16,09,020,,36,,,*76",
        "$GPRMC,092750.000,A,5321.6802,N,00630.3372,W,0.02,31.66,280511,,,A*43",
    ];

    let mut nmea = Nmea::default();
    for s in &sentences {
        let res = nmea.parse(s).unwrap();
        println!("test_parse res {:?}", res);
    }

    assert_eq!(nmea.latitude().unwrap(), 53. + 21.6802 / 60.);
    assert_eq!(nmea.longitude().unwrap(), -(6. + 30.3372 / 60.));
    assert_eq!(nmea.altitude().unwrap(), 61.7);
    pretty_assertions::assert_eq!(
        vec![
            "{Gps 2 Some(39.0) Some(223.0) Some(19.0)}",
            "{Gps 4 Some(14.0) Some(186.0) Some(14.0)}",
            "{Gps 5 Some(59.0) Some(290.0) Some(20.0)}",
            "{Gps 7 Some(61.0) Some(98.0) Some(15.0)}",
            "{Gps 8 Some(54.0) Some(157.0) Some(30.0)}",
            "{Gps 10 Some(63.0) Some(137.0) Some(17.0)}",
            "{Gps 13 Some(28.0) Some(70.0) Some(17.0)}",
            "{Gps 16 Some(9.0) Some(20.0) None}",
            "{Gps 26 Some(23.0) Some(252.0) None}",
            "{Gps 29 Some(9.0) Some(301.0) Some(24.0)}",
            "{Gps 36 None None None}",
        ],
        format_satellites(nmea.satellites())
    );
}

#[test]
fn test_parse_for_fix() {
    {
        let mut nmea =
            Nmea::create_for_navigation(&[SentenceType::RMC, SentenceType::GGA]).unwrap();
        let log = [
            (
                "$GPRMC,123308.2,A,5521.76474,N,03731.92553,E,000.48,071.9,090317,010.2,E,A*3B",
                FixType::Invalid,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 200)),
            ),
            (
                "$GPGGA,123308.2,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*52",
                FixType::Gps,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 200)),
            ),
            (
                "$GPVTG,071.9,T,061.7,M,000.48,N,0000.88,K,A*10",
                FixType::Invalid,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 200)),
            ),
            (
                "$GPRMC,123308.3,A,5521.76474,N,03731.92553,E,000.51,071.9,090317,010.2,E,A*32",
                FixType::Invalid,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 300)),
            ),
            (
                "$GPGGA,123308.3,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*53",
                FixType::Gps,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 300)),
            ),
            (
                "$GPVTG,071.9,T,061.7,M,000.51,N,0000.94,K,A*15",
                FixType::Invalid,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 300)),
            ),
            (
                "$GPRMC,123308.4,A,5521.76474,N,03731.92553,E,000.54,071.9,090317,010.2,E,A*30",
                FixType::Invalid,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 400)),
            ),
            (
                "$GPGGA,123308.4,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*54",
                FixType::Gps,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 400)),
            ),
            (
                "$GPVTG,071.9,T,061.7,M,000.54,N,0001.00,K,A*1C",
                FixType::Invalid,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 400)),
            ),
            (
                "$GPRMC,123308.5,A,5521.76474,N,03731.92553,E,000.57,071.9,090317,010.2,E,A*32",
                FixType::Invalid,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 500)),
            ),
            (
                "$GPGGA,123308.5,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*55",
                FixType::Gps,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 500)),
            ),
            (
                "$GPVTG,071.9,T,061.7,M,000.57,N,0001.05,K,A*1A",
                FixType::Invalid,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 500)),
            ),
            (
                "$GPRMC,123308.6,A,5521.76474,N,03731.92553,E,000.58,071.9,090317,010.2,E,A*3E",
                FixType::Invalid,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 600)),
            ),
            (
                "$GPGGA,123308.6,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*56",
                FixType::Gps,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 600)),
            ),
            (
                "$GPVTG,071.9,T,061.7,M,000.58,N,0001.08,K,A*18",
                FixType::Invalid,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 600)),
            ),
            (
                "$GPRMC,123308.7,A,5521.76474,N,03731.92553,E,000.59,071.9,090317,010.2,E,A*3E",
                FixType::Invalid,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 700)),
            ),
            (
                "$GPGGA,123308.7,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*57",
                FixType::Gps,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 700)),
            ),
            (
                "$GPVTG,071.9,T,061.7,M,000.59,N,0001.09,K,A*18",
                FixType::Invalid,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 700)),
            ),
        ];

        for (i, item) in log.iter().enumerate() {
            let res = nmea.parse_for_fix(item.0.as_bytes()).unwrap();
            println!("parse result({}): {:?}, {:?}", i, res, nmea.fix_time);
            assert_eq!((&res, &nmea.fix_time), (&item.1, &item.2));
        }
    }

    {
        let mut nmea =
            Nmea::create_for_navigation(&[SentenceType::RMC, SentenceType::GGA]).unwrap();
        let log = [
            (
                "$GPRMC,123308.2,A,5521.76474,N,03731.92553,E,000.48,071.9,090317,010.2,E,A*3B",
                FixType::Invalid,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 200)),
            ),
            (
                "$GPRMC,123308.3,A,5521.76474,N,03731.92553,E,000.51,071.9,090317,010.2,E,A*32",
                FixType::Invalid,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 300)),
            ),
            (
                "$GPGGA,123308.3,5521.76474,N,03731.92553,E,1,08,2.2,211.5,M,13.1,M,,*53",
                FixType::Gps,
                Some(NaiveTime::from_hms_milli(12, 33, 8, 300)),
            ),
        ];

        for (i, item) in log.iter().enumerate() {
            let res = nmea.parse_for_fix(item.0.as_bytes()).unwrap();
            println!("parse result({}): {:?}, {:?}", i, res, nmea.fix_time);
            assert_eq!((&res, &nmea.fix_time), (&item.1, &item.2));
        }
    }
}

#[test]
fn test_some_reciever() {
    let lines = [
        "$GPRMC,171724.000,A,6847.2474,N,03245.8351,E,0.26,140.74,250317,,*02",
        "$GPGGA,171725.000,6847.2473,N,03245.8351,E,1,08,1.0,87.7,M,18.5,M,,0000*66",
        "$GPGSA,A,3,02,25,29,12,31,06,23,14,,,,,2.0,1.0,1.7*3A",
        "$GPRMC,171725.000,A,6847.2473,N,03245.8351,E,0.15,136.12,250317,,*05",
        "$GPGGA,171726.000,6847.2473,N,03245.8352,E,1,08,1.0,87.8,M,18.5,M,,0000*69",
        "$GPGSA,A,3,02,25,29,12,31,06,23,14,,,,,2.0,1.0,1.7*3A",
        "$GPRMC,171726.000,A,6847.2473,N,03245.8352,E,0.16,103.49,250317,,*0E",
        "$GPGGA,171727.000,6847.2474,N,03245.8353,E,1,08,1.0,87.9,M,18.5,M,,0000*6F",
        "$GPGSA,A,3,02,25,29,12,31,06,23,14,,,,,2.0,1.0,1.7*3A",
        "$GPRMC,171727.000,A,6847.2474,N,03245.8353,E,0.49,42.80,250317,,*32",
    ];
    let mut nmea = Nmea::create_for_navigation(&[SentenceType::RMC, SentenceType::GGA]).unwrap();
    println!("start test");
    let mut nfixes = 0_usize;
    for line in &lines {
        match nmea.parse_for_fix(line.as_bytes()) {
            Ok(FixType::Invalid) => {
                println!("invalid");
                continue;
            }
            Err(msg) => {
                println!("update_gnss_info_nmea: parse_for_fix failed: {:?}", msg);
                continue;
            }
            Ok(_) => nfixes += 1,
        }
    }
    assert_eq!(nfixes, 3);
}

#[test]
fn test_gll() {
    use chrono::Timelike;
    let mut nmea = Nmea::default();

    // Example from https://docs.novatel.com/OEM7/Content/Logs/GPGLL.htm
    nmea.parse("$GPGLL,5107.0013414,N,11402.3279144,W,205412.00,A,A*73")
        .unwrap();
    assert_eq!(51. + 7.0013414 / 60., nmea.latitude().unwrap());
    assert_eq!(-(114. + 2.3279144 / 60.), nmea.longitude().unwrap());
    assert_eq!(20, nmea.fix_timestamp().unwrap().hour());
    assert_eq!(54, nmea.fix_timestamp().unwrap().minute());
    assert_eq!(12, nmea.fix_timestamp().unwrap().second());

    // Example from https://www.gpsinformation.org/dale/nmea.htm#GLL
    nmea.parse("$GPGLL,4916.45,N,12311.12,W,225444,A,*1D")
        .unwrap();
    assert_eq!(49. + 16.45 / 60., nmea.latitude().unwrap());
    assert_eq!(-(123. + 11.12 / 60.), nmea.longitude().unwrap());
    assert_eq!(22, nmea.fix_timestamp().unwrap().hour());
    assert_eq!(54, nmea.fix_timestamp().unwrap().minute());
    assert_eq!(44, nmea.fix_timestamp().unwrap().second());
}

/// ensure right order before dump to string
fn format_satellites(mut sats: Vec<Satellite, 46>) -> std::vec::Vec<String> {
    sats.sort_by_key(|s| (s.gnss_type() as u8, s.prn()));
    // to not depend on Debug impl for `Satellite` stability

    sats.iter()
        .map(|s| {
            format!(
                "{{{gnss_type:?} {prn} {elevation:?} {azimuth:?} {snr:?}}}",
                gnss_type = s.gnss_type(),
                prn = s.prn(),
                elevation = s.elevation(),
                azimuth = s.azimuth(),
                snr = s.snr(),
            )
        })
        .collect::<std::vec::Vec<String>>()
}