use heapless::Vec;
use nmea::Satellite;

/// ensure right order before dump to string
pub fn format_satellites(mut sats: Vec<Satellite, 58>) -> std::vec::Vec<String> {
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
