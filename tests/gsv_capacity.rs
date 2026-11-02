#![cfg(feature = "GSV")]

use nmea::Nmea;

fn sentence(body: &str) -> String {
    let checksum = body.bytes().fold(0, |value, byte| value ^ byte);
    format!("${body}*{checksum:02X}")
}

/// If the initial "number of space vehicles" message gets truncated but
/// still produces a valid checksum, then the array can be incorrectly
/// sized. Ensure it does not cause a panic.
#[test]
fn gsv_sentence_number_past_capacity_does_not_panic() {
    let mut nmea = Nmea::default();
    for number in [15, 16, 99] {
        let _ = nmea.parse(&sentence(&format!("GPGSV,{number},{number},64,01,49,196,41")));
        for _ in 0..20 {
            nmea.parse("$GPGSV,3,1,11,10,63,137,17,07,61,098,15,05,59,290,20,08,54,157,30*70")
                .unwrap();
        }
    }
}
