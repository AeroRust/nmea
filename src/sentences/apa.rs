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


use std::fmt;

#[derive(Debug)]
struct APASentence {
    sentence_type: String,
    cross_track_error: f64,
    direction_to_steer: char,
    status: char,
    cross_track_units: char,
    waypoint_id: String,
    bearing_to_destination: f64,
    heading_to_destination: char,
    arrival_circle_entered: char,
    perimeter_laid: char,
    steer_to_heading: char,
}

impl fmt::Display for APASentence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{\"sentence_type\":\"{}\",\"cross_track_error\":{},\"direction_to_steer\":\"{}\",\"status\":\"{}\",\"cross_track_units\":\"{}\",\"waypoint_id\":\"{}\",\"bearing_to_destination\":{:0.1},\"heading_to_destination\":\"{}\",\"arrival_circle_entered\":\"{}\",\"perimeter_laid\":\"{}\",\"steer_to_heading\":\"{}\"}}",
            self.sentence_type,
            self.cross_track_error,
            self.direction_to_steer,
            self.status,
            self.cross_track_units,
            self.waypoint_id,
            self.bearing_to_destination,
            self.heading_to_destination,
            self.arrival_circle_entered,
            self.perimeter_laid,
            self.steer_to_heading,
        )
    }
}

fn main() {
    let apa_sentence = APASentence {
        sentence_type: String::from("$APA"),
        cross_track_error: 1.5,
        direction_to_steer: 'L',
        status: 'A',
        cross_track_units: 'N',
        waypoint_id: String::from("WPT001"),
        bearing_to_destination: 45.0,
        heading_to_destination: 'T',
        arrival_circle_entered: 'A',
        perimeter_laid: 'N',
        steer_to_heading: 'N',
    };

    let nmea_sentence = apa_sentence.to_string();
    println!("NMEA APA Sentence: {}", nmea_sentence);
}

#[test]
fn test_apa_sentence_serialization() {
    let apa_sentence = APASentence {
        sentence_type: String::from("$APA"),
        cross_track_error: 1.5,
        direction_to_steer: 'L',
        status: 'A',
        cross_track_units: 'N',
        waypoint_id: String::from("WPT001"),
        bearing_to_destination: 45.0,
        heading_to_destination: 'T',
        arrival_circle_entered: 'A',
        perimeter_laid: 'N',
        steer_to_heading: 'N',
    };

    let expected_json = r#"{"sentence_type":"$APA","cross_track_error":1.5,"direction_to_steer":"L","status":"A","cross_track_units":"N","waypoint_id":"WPT001","bearing_to_destination":45.0,"heading_to_destination":"T","arrival_circle_entered":"A","perimeter_laid":"N","steer_to_heading":"N"}"#;

    let actual_json = apa_sentence.to_string();
    assert_eq!(actual_json, expected_json);
}