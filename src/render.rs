use crate::sentences::FixType;
use core::fmt::{self, Write};
use heapless::String as HeaplessString;

pub(crate) fn fix_type_to_str(fix_type: &FixType) -> HeaplessString<16> {
    let mut result = HeaplessString::new();
    match fix_type {
        FixType::Invalid => result.push_str("0").unwrap(),
        FixType::Gps => result.push_str("1").unwrap(),
        FixType::DGps => result.push_str("2").unwrap(),
        FixType::Pps => result.push_str("3").unwrap(),
        FixType::Rtk => result.push_str("4").unwrap(),
        FixType::FloatRtk => result.push_str("5").unwrap(),
        FixType::Estimated => result.push_str("6").unwrap(),
        FixType::Manual => result.push_str("7").unwrap(),
        FixType::Simulation => result.push_str("8").unwrap(),
    }
    result
}

pub(crate) fn format_float(value: Option<f32>, precision: usize) -> heapless::String<16> {
    let mut result = heapless::String::new();
    if let Some(v) = value {
        let _ = write!(result, "{:.1$}", v, precision);
    }
    result
}

pub(crate) fn format_u32(value: Option<u32>, width: usize) -> heapless::String<16> {
    let mut result = heapless::String::new();
    if let Some(v) = value {
        let _ = write!(result, "{:0width$}", v, width = width);
    }
    result
}

/// Prefixes the talker id part (like `$GP`) into a sentence like `GGA,123519...` and adds checksum at the end, forming a valid NMEA sentence.
pub fn nmea_prefix_sentence<'a>(
    nmea_sentence: &crate::NmeaSentence,
    msg: &str,
) -> Result<HeaplessString<128>, crate::error::Error<'a>> {
    let mut prefixed = HeaplessString::new();
    prefixed
        .push_str("$")
        .map_err(|()| crate::Error::RenderError("push_str error"))?;
    prefixed
        .push_str(nmea_sentence.talker_id)
        .map_err(|()| crate::Error::RenderError("push_str error"))?;
    prefixed
        .push_str(msg)
        .map_err(|()| crate::Error::RenderError("push_str error"))?;
    let checksum = nmea_sentence.calc_checksum();
    let mut checksum_str: HeaplessString<128> = HeaplessString::new();
    write!(&mut checksum_str, "*{:02X}\r\n", checksum)
        .map_err(|ref _e: fmt::Error| crate::Error::RenderError("fmt Error"))?;
    prefixed
        .push_str(&checksum_str)
        .map_err(|()| crate::Error::RenderError("push_str error"))?;
    Ok(prefixed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_nmea_sentence;

    #[test]
    fn test_nmea_prefix_sentence() {
        let nmea_sentence =
            parse_nmea_sentence("$GNGSA,M,1,,,,,,,,,,,,,99.99,99.99,99.99*22").unwrap();
        println!("talker_id: {}", nmea_sentence.talker_id);
        let msg = "GSA,M,1,,,,,,,,,,,,,99.99,99.99,99.99";
        let result = nmea_prefix_sentence(&nmea_sentence, msg).unwrap();
        assert_eq!(result, "$GNGSA,M,1,,,,,,,,,,,,,99.99,99.99,99.99*22\r\n");
    }
}
