#![feature(test)]

extern crate test;

#[cfg(test)]
mod tests {
    use test::{black_box, Bencher};

    use nmea::{Nmea, SentenceType};

    #[bench]
    fn bench_gsv_parser(b: &mut Bencher) {
        let input = [
            "$GPGSV,3,1,12,01,49,196,41,03,71,278,32,06,02,323,27,11,21,196,39*72",
            "$GPGSV,3,2,12,14,39,063,33,17,21,292,30,19,20,310,31,22,82,181,36*73",
            "$GPGSV,3,3,12,23,34,232,42,25,11,045,33,31,45,092,38,32,14,061,39*75",
            "$GLGSV,3,1,10,74,40,078,43,66,23,275,31,82,10,347,36,73,15,015,38*6B",
            "$GLGSV,3,2,10,75,19,135,36,65,76,333,31,88,32,233,33,81,40,302,38*6A",
            "$GLGSV,3,3,10,72,40,075,43,87,00,000,*6F",
        ];

        b.iter(|| {
            let mut nmea = Nmea::default();
            for line in &input {
                let pack_type = black_box(nmea.parse(line).unwrap());
                assert_eq!(pack_type, SentenceType::GSV);
            }
        });
    }
}
