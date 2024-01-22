/// APA - Autopilot Sentence "A"
///
/// https://gpsd.gitlab.io/gpsd/NMEA.html#_apa_autopilot_sentence_a
///
/// '''text
///         1 2  3   4 5 6 7  8  9 10    11
///         | |  |   | | | |  |  | |     |
///  $--APA,A,A,x.xx,L,N,A,A,xxx,M,c---c*hh<CR><LF>
/// '''
/// Field Number:
///
/// 1. Status V = Loran-C Blink or SNR warning V = general warning flag or other navigation systems when a reliable fix is not available
/// 2. Status V = Loran-C Cycle Lock warning flag A = OK or not used
/// 3. Cross Track Error Magnitude
/// 4. Direction to steer, L or R
/// 5. Cross Track Units (Nautical miles or kilometers)
/// 6. Status A = Arrival Circle Entered
/// 7. Status A = Perpendicular passed at waypoint
/// 8. Bearing origin to destination
/// 9. M = Magnetic, T = True
/// 10. Destination Waypoint ID
/// 11. Checksum


use std::io::{self, BufRead};

#[derive(Debug)]
struct APA {
    cross_track_error: f64,
    steer_to: String,
    heading_deviation: f64,
    heading_deviation_dir: String,
    cross_track_units: String,
    arrival_circle_entered: bool,
    perpendicular_passed: bool,
}

fn parse_apa(sentence: &str) -> Option<APA> {
    let fields: Vec<&str> = sentence.split(',').collect();

    if fields.len() >= 11 && fields[0] == "$APA" {
        let cross_track_error = fields[1].parse().ok()?;
        let steer_to = fields[2].to_string();
        let heading_deviation = fields[3].parse().ok()?;
        let heading_deviation_dir = fields[4].to_string();
        let cross_track_units = fields[5].to_string();
        let arrival_circle_entered = fields[6] == "A";
        let perpendicular_passed = fields[7] == "P";

        Some(APA {
            cross_track_error,
            steer_to,
            heading_deviation,
            heading_deviation_dir,
            cross_track_units,
            arrival_circle_entered,
            perpendicular_passed,
        })
    } else {
        None
    }
}

fn main() {
    let input_data = "$APA,3.4,R,1.2,N,ACTIVE*32";

    match parse_apa(input_data) {
        Some(apa_data) => println!("{:?}", apa_data),
        None => println!("Invalid APA sentence"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_apa() {
        let input_data = "$APA,3.4,R,1.2,N,ACTIVE*32";
        let apa_data = parse_apa(input_data);

        assert!(apa_data.is_some(), "Failed to parse APA sentence: {}", input_data);

        if let Some(apa_data) = apa_data {
            assert_eq!(apa_data.cross_track_error, 3.4);
            assert_eq!(apa_data.steer_to, "R");
            assert_eq!(apa_data.heading_deviation, 1.2);
            assert_eq!(apa_data.heading_deviation_dir, "N");
            assert_eq!(apa_data.cross_track_units, "ACTIVE");
            assert_eq!(apa_data.arrival_circle_entered, true);
            assert_eq!(apa_data.perpendicular_passed, false);
        }
    }


    #[test]
    fn test_parse_invalid_apa() {
        let input_data = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47";
        let apa_data = parse_apa(input_data);

        assert!(apa_data.is_none());
    }
}
