use GnssType;
use Satellite;
use Nmea;
use Result;

pub struct NmeaSentence<'a> {
    pub talker_id: &'a str,
    pub message_id: &'a str,
    pub data: &'a str,
}

pub struct GsvData {
    pub gnss_type: GnssType,
    pub number_of_sentences: u16,
    pub sentence_num: u16,
    pub _sats_in_view: u16,
    pub sats_info: [Option<Satellite>; 4],
}

pub fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0, |c, x| c ^ x)
}


macro_rules! map_not_empty {
    ($StrName: ident, $Expr: expr) => {
        if !$StrName.is_empty() {
            Some($Expr)
        } else {
            None
        }
    }
}

/// Parsin one GSV sentence
/// from gpsd/driver_nmea0183.c:
/// $IDGSV,2,1,08,01,40,083,46,02,17,308,41,12,07,344,39,14,22,228,45*75
/// 2           Number of sentences for full data
/// 1           Sentence 1 of 2
/// 08          Total number of satellites in view
/// 01          Satellite PRN number
/// 40          Elevation, degrees
/// 083         Azimuth, degrees
/// 46          Signal-to-noise ratio in decibels
/// <repeat for up to 4 satellites per sentence>
///
/// Can occur with talker IDs:
///   BD (Beidou),
///   GA (Galileo),
///   GB (Beidou),
///   GL (GLONASS),
///   GN (GLONASS, any combination GNSS),
///   GP (GPS, SBAS, QZSS),
///   QZ (QZSS).
///
/// GL may be (incorrectly) used when GSVs are mixed containing
/// GLONASS, GN may be (incorrectly) used when GSVs contain GLONASS
/// only.  Usage is inconsistent.
pub fn parse_gsv(sentence: &NmeaSentence) -> Result<GsvData> {    
    if sentence.message_id != "GSV" {
        return Err("GSV sentence not starts with $..GSV".into());
    }

    let gnss_type = match sentence.talker_id {
        "GP" => GnssType::Gps,
        "GL" => GnssType::Glonass,
        _ => return Err("Unknown GNSS type in GSV sentence".into()),
    };

    let mut field = sentence.data.split(",");
    let number_of_sentences = Nmea::parse_numeric::<u16>(field.next().ok_or("no sentence amount")?, 1)?;
    let sentence_num = Nmea::parse_numeric::<u16>(field.next().ok_or("no sentence number")?, 1)?;
    let sats_in_view = Nmea::parse_numeric::<u16>(field.next().ok_or("no number of satellites in view")?, 1)?;

    let rest_field_num = field.clone().count();
    let nsats_info = rest_field_num / 4;
    if rest_field_num % 4 != 0 || nsats_info > 4 {
        return Err("mailformed sattilite info in GSV".into());
    }
    let mut sats_info = [None, None, None, None];
    for idx in 0..nsats_info {
        let prn = Nmea::parse_numeric::<u32>(field.next().ok_or("no prn")?, 1)?;
        let elevation = field.next().ok_or("no elevation")?;
        let elevation = map_not_empty!(elevation, Nmea::parse_numeric::<f32>(elevation, 1.0)?);
        let azimuth = field.next().ok_or("no aizmuth")?;
        let azimuth = map_not_empty!(azimuth, Nmea::parse_numeric::<f32>(azimuth, 1.0)?);
        let snr = field.next().ok_or("no SNR")?;
        let snr = map_not_empty!(snr, Nmea::parse_numeric::<f32>(snr, 1.0)?);
        sats_info[idx] = Some(Satellite{gnss_type: gnss_type.clone(), prn: prn, elevation: elevation, azimuth: azimuth, snr: snr});
    }
    Ok(GsvData {
        gnss_type: gnss_type,
        number_of_sentences: number_of_sentences,
        sentence_num: sentence_num,
        _sats_in_view: sats_in_view,
        sats_info: sats_info,
    })
}

#[test]
fn test_parse_gsv_full() {
    let data = parse_gsv(&NmeaSentence {
        talker_id: "GP",
        message_id: "GSV",
        data: "2,1,08,01,40,083,46,02,17,308,41,12,07,344,39,14,22,228,45",
    }).unwrap();
    assert_eq!(data.gnss_type, GnssType::Gps);
    assert_eq!(data.number_of_sentences, 2);
    assert_eq!(data.sentence_num, 1);
    assert_eq!(data._sats_in_view, 8);
    assert_eq!(data.sats_info[0].clone().unwrap(), Satellite{gnss_type: data.gnss_type.clone(), prn: 1, elevation: Some(40.), azimuth: Some(83.), snr: Some(46.)});
    assert_eq!(data.sats_info[1].clone().unwrap(), Satellite{gnss_type: data.gnss_type.clone(), prn: 2, elevation: Some(17.), azimuth: Some(308.), snr: Some(41.)});
    assert_eq!(data.sats_info[2].clone().unwrap(), Satellite{gnss_type: data.gnss_type.clone(), prn: 12, elevation: Some(7.), azimuth: Some(344.), snr: Some(39.)});
    assert_eq!(data.sats_info[3].clone().unwrap(), Satellite{gnss_type: data.gnss_type.clone(), prn: 14, elevation: Some(22.), azimuth: Some(228.), snr: Some(45.)});
}
