use nmea::*;

#[test]
fn test_invalid_datetime() {
    let mut nmea = Nmea::new();
    let res = nmea.parse("$,GRMC,,A,,,,,,,290290GLCR*40");
    println!("parse result {:?}", res);
    assert!(matches!(res, Err(NmeaError::ParsingError(_))));
}
