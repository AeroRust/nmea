use crate::parse::TEXT_PARAMETER_MAX_LEN;

use arrayvec::ArrayString;
use nom::{
    Parser as _,
    bytes::complete::is_not,
    character::complete::{char, one_of},
    combinator::opt,
    number::complete::float,
};

use crate::{Error, SentenceType, parse::NmeaSentence, sentences::utils::array_string};

/// APA - Autopilot Sentence "A"
///
/// This sentence is sent by some GPS receivers to allow them to be used to control an autopilot unit
///
/// This sentence is commonly used by autopilots and contains navigation receiver warning flag status,
/// cross-track-error, waypoint arrival status, initial bearing from origin waypoint to the destination,
/// continuous bearing from present position to destination and recommended heading-to-steer to
/// destination waypoint for the active navigation leg of the journey.
///
/// <https://gpsd.gitlab.io/gpsd/NMEA.html#_apa_autopilot_sentence_a>
///
/// ```text
///        1 2  3   4 5 6 7  8  9 10    11
///        | |  |   | | | |  |  | |     |
/// $--APA,A,A,x.xx,L,N,A,A,xxx,M,c---c*hh<CR><LF>
/// ```
///
/// Field Number:
///
/// 1. Status, BOOLEAN, V = Loran-C Blink or SNR warning A = general warning flag or other navigation systems when a reliable fix is not available
/// 2. Status, BOOLEAN, V = Loran-C Cycle Lock warning flag A = OK or not used
/// 3. Cross Track Error Magnitude
/// 4. Direction to steer, L = Left or R = Right
/// 5. Cross Track Units, N = Nautical miles or K = Kilometers
/// 6. Status, BOOLEAN, A = Arrival Circle Entered, V = Not Entered
/// 7. Status, BOOLEAN, A = Perpendicular passed at waypoint, V = Not Passed
/// 8. Bearing origin to destination
/// 9. M = Magnetic, T = True
/// 10. Destination Waypoint ID
/// 11. Checksum
///
/// Example: `$GPAPA,A,A,0.10,R,N,V,V,011,M,DEST,011,M*82`
/// Where the last "M" is the waypoint name
///
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, PartialEq, Clone)]
pub struct ApaData {
    pub status_warning: Option<bool>,
    pub status_cycle_warning: Option<bool>,
    pub cross_track_error_magnitude: Option<f32>,
    pub steer_direction: Option<SteerDirection>,
    pub cross_track_units: Option<CrossTrackUnits>,
    pub status_arrived: Option<bool>,
    pub status_passed: Option<bool>,
    pub bearing_origin_destination: Option<f32>,
    pub magnetic_true: Option<MagneticTrue>,
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub waypoint_id: Option<ArrayString<TEXT_PARAMETER_MAX_LEN>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SteerDirection {
    Left,
    Right,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CrossTrackUnits {
    Nautical,
    Kilometers,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MagneticTrue {
    Magnetic,
    True,
}

/// Parse APA message
pub fn parse_apa(sentence: NmeaSentence<'_>) -> Result<ApaData, Error<'_>> {
    if sentence.message_id != SentenceType::APA {
        Err(Error::WrongSentenceHeader {
            expected: SentenceType::APA,
            found: sentence.message_id,
        })
    } else {
        Ok(do_parse_apa(sentence.data)?)
    }
}

fn do_parse_apa(i: &str) -> Result<ApaData, Error<'_>> {
    let (i, status_warning) = one_of("AV").parse(i)?;
    let status_warning = match status_warning {
        'A' => Some(true),
        'V' => Some(false),
        _ => unreachable!(),
    };
    let (i, _) = char(',').parse(i)?;

    let (i, status_cycle_warning) = one_of("AV").parse(i)?;
    let status_cycle_warning = match status_cycle_warning {
        'A' => Some(true),
        'V' => Some(false),
        _ => unreachable!(),
    };
    let (i, _) = char(',').parse(i)?;

    let (i, cross_track_error_magnitude) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, steer_direction) = one_of("LR").parse(i)?;
    let steer_direction = match steer_direction {
        'L' => Some(SteerDirection::Left),
        'R' => Some(SteerDirection::Right),
        _ => unreachable!(),
    };
    let (i, _) = char(',').parse(i)?;

    let (i, cross_track_units) = one_of("NK").parse(i)?;
    let cross_track_units = match cross_track_units {
        'N' => Some(CrossTrackUnits::Nautical),
        'K' => Some(CrossTrackUnits::Kilometers),
        _ => unreachable!(),
    };
    let (i, _) = char(',').parse(i)?;

    let (i, status_arrived) = one_of("AV").parse(i)?;
    let status_arrived = match status_arrived {
        'A' => Some(true),
        'V' => Some(false),
        _ => unreachable!(),
    };
    let (i, _) = char(',').parse(i)?;

    let (i, status_passed) = one_of("AV").parse(i)?;
    let status_passed = match status_passed {
        'A' => Some(true),
        'V' => Some(false),
        _ => unreachable!(),
    };
    let (i, _) = char(',').parse(i)?;

    let (i, bearing_origin_destination) = opt(float).parse(i)?;
    let (i, _) = char(',').parse(i)?;

    let (i, magnetic_true) = one_of("MT").parse(i)?;
    let magnetic_true = match magnetic_true {
        'M' => Some(MagneticTrue::Magnetic),
        'T' => Some(MagneticTrue::True),
        _ => unreachable!(),
    };
    let (i, _) = char(',').parse(i)?;

    let (_i, waypoint_id) = opt(is_not("*")).parse(i)?;

    Ok(ApaData {
        status_warning,
        status_cycle_warning,
        cross_track_error_magnitude,
        steer_direction,
        cross_track_units,
        status_arrived,
        status_passed,
        bearing_origin_destination,
        magnetic_true,
        waypoint_id: waypoint_id
            .map(array_string::<TEXT_PARAMETER_MAX_LEN>)
            .transpose()?,
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::{SentenceType, parse::parse_nmea_sentence};

    #[test]
    fn parse_apa_with_nmea_sentence_struct() {
        let data = parse_apa(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::APA,
            data: "A,A,0.10,R,N,V,V,011,M,DEST,011,M*42",
            checksum: 0x3E,
        })
        .unwrap();

        assert!(data.status_warning.unwrap());
        assert!(data.status_cycle_warning.unwrap());
        assert_relative_eq!(data.cross_track_error_magnitude.unwrap(), 0.10);
        assert_eq!(data.steer_direction.unwrap(), SteerDirection::Right);
        assert_eq!(data.cross_track_units.unwrap(), CrossTrackUnits::Nautical);
        assert!(!data.status_arrived.unwrap());
        assert!(!data.status_passed.unwrap());
        assert_relative_eq!(data.bearing_origin_destination.unwrap(), 11.0);
        assert_eq!(data.magnetic_true.unwrap(), MagneticTrue::Magnetic);
        assert_eq!(&data.waypoint_id.unwrap(), "DEST,011,M");
    }

    #[test]
    fn parse_apa_full_sentence() {
        let sentence = parse_nmea_sentence("$GPAPA,A,A,0.10,R,N,V,V,011,M,DEST,011,M*42").unwrap();
        assert_eq!(sentence.checksum, 0x42);
        assert_eq!(sentence.calc_checksum(), 0x42);

        let data = parse_apa(sentence).unwrap();
        assert!(data.status_warning.unwrap());
        assert!(data.status_cycle_warning.unwrap());
        assert_relative_eq!(data.cross_track_error_magnitude.unwrap(), 0.10);
        assert_eq!(data.steer_direction.unwrap(), SteerDirection::Right);
        assert_eq!(data.cross_track_units.unwrap(), CrossTrackUnits::Nautical);
        assert!(!data.status_arrived.unwrap());
        assert!(!data.status_passed.unwrap());
        assert_relative_eq!(data.bearing_origin_destination.unwrap(), 11.0);
        assert_eq!(data.magnetic_true.unwrap(), MagneticTrue::Magnetic);
        assert_eq!(&data.waypoint_id.unwrap(), "DEST,011,M");
    }

    #[test]
    #[should_panic]
    fn parse_apa_with_invalid_status_warning_value() {
        parse_apa(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::APA,
            data: "G,A,0.10,R,N,V,V,011,M,DEST,011,M*4",
            checksum: 0x0,
        })
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn parse_apa_with_invalid_magnetic_true_value() {
        parse_apa(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::APA,
            data: "A,A,0.10,R,N,V,V,011,X,DEST,011,M*4",
            checksum: 0x0,
        })
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn parse_apa_with_invalid_cross_track_units_value() {
        parse_apa(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::APA,
            data: "A,A,0.10,R,C,V,V,011,M,DEST,011,M*4",
            checksum: 0x0,
        })
        .unwrap();
    }

    #[test]
    fn parse_apa_with_wrong_message_id() {
        let error = parse_apa(NmeaSentence {
            talker_id: "GP",
            message_id: SentenceType::ABK,
            data: "A,A,0.10,R,N,V,V,011,M,DEST,011,M*42",
            checksum: 0x43,
        })
        .unwrap_err();

        if let Error::WrongSentenceHeader { expected, found } = error {
            assert_eq!(expected, SentenceType::APA);
            assert_eq!(found, SentenceType::ABK);
        }
    }
}
