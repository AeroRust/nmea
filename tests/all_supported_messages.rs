use std::collections::HashMap;

use nmea::{parse_str, Error, Nmea, SentenceType};

#[test]
fn test_all_supported_messages() {
    let sentences = [
        // AAM
        (SentenceType::AAM, "$GPAAM,A,A,0.10,N,WPTNME*32"),
        // ALM
        (SentenceType::ALM, "$GPALM,1,1,15,1159,00,441D,4E,16BE,FD5E,A10C9F,4A2DA4,686E81,58CBE1,0A4,001*77"),
        // BWC
        (SentenceType::BWC, "$GPBWC,220516,5130.02,N,00046.34,W,213.8,T,218.0,M,0004.6,N,EGLM*21"),
        // GGA
        (SentenceType::GGA, "$GPGGA,133605.0,5521.75946,N,03731.93769,E,0,00,,,M,,M,,*4F"),
        // GLL
        (SentenceType::GLL, "$GPGLL,5107.0013414,N,11402.3279144,W,205412.00,A,A*73"),
        // GNS
        (SentenceType::GNS, "$GPGNS,224749.00,3333.4268304,N,11153.3538273,W,D,19,0.6,406.110,-26.294,6.0,0138,S,*46"),
        // GSA
        (SentenceType::GSA, "$GPGSA,A,3,23,31,22,16,03,07,,,,,,,1.8,1.1,1.4*3E"),
        // GSV
        (SentenceType::GSV, "$GPGSV,3,1,12,01,49,196,41,03,71,278,32,06,02,323,27,11,21,196,39*72"),
        // MDA
        (SentenceType::MDA, "$WIMWV,041.1,R,01.0,N,A*16"),
        // MWV
        (SentenceType::MWV, "$WIMDA,29.7544,I,1.0076,B,35.5,C,,,42.1,,20.6,C,116.4,T,107.7,M,1.2,N,0.6,M*66"),
        // RMC
        (SentenceType::RMC, "$GPRMC,225446.33,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E,A*2B"),
        // TXT
        (SentenceType::TXT, "$GNTXT,01,01,02,u-blox AG - www.u-blox.com*4E"),
        // VTG
        (SentenceType::VTG, "$GPVTG,360.0,T,348.7,M,000.0,N,000.0,K*43"),
    ]
    .into_iter()
    .collect::<HashMap<_, _>>();

    // `parse_str()` test
    {
        let parse_results = sentences
            .values()
            .map(|sentence| parse_str(sentence).map_err(|result| (sentence, result)))
            .collect::<Vec<_>>();

        let errors = parse_results
            .into_iter()
            .filter_map(|result| result.err())
            .collect::<Vec<_>>();

        assert_eq!(
            0,
            errors.len(),
            "All supported messages should be parsable:\n{:?}",
            errors
        )
    }

    // `Nmea::parse()` - does not support all messages!
    {
        let mut nmea = Nmea::default();

        let parse_results = sentences
            .values()
            .map(|sentence| nmea.parse(sentence).map_err(|result| (sentence, result)))
            .collect::<Vec<_>>();

        let errors = parse_results
            .into_iter()
            .filter_map(|result| result.err())
            .map(|(_sentence, error_type)| error_type)
            .collect::<Vec<_>>();

        assert!(errors.contains(&Error::Unsupported(SentenceType::BWC)));
        assert!(errors.contains(&Error::Unsupported(SentenceType::AAM)));
    }
}
