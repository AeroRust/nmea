use crate::sentences::FixType;
use core::fmt::Write;

pub(crate) fn fix_type_to_str(fix_type: &FixType) -> String {
    match fix_type {
        FixType::Invalid => "0".to_string(),
        FixType::Gps => "1".to_string(),
        FixType::DGps => "2".to_string(),
        FixType::Pps => "3".to_string(),
        FixType::Rtk => "4".to_string(),
        FixType::FloatRtk => "5".to_string(),
        FixType::Estimated => "6".to_string(),
        FixType::Manual => "7".to_string(),
        FixType::Simulation => "8".to_string(),
    }
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
