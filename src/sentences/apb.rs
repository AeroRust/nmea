use std::fmt;
use std::str::FromStr;


#[derive(Debug)]
struct APBSentence {
    sentence_type: String,
    status: char,
    cross_track_error: f64,
    direction_to_steer: char,
    waypoint_id: String,
    bearing_to_destination: f64,
    destination_id: String,
    arrival_circle_entered: char,
    perp_bisector_origin_id: String,
    bearing_to_perpendicular_bisector: f64,
    heading_to_perpendicular_bisector: char,
}

impl fmt::Display for APBSentence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{\"sentence_type\":\"{}\",\"status\":\"{}\",\"cross_track_error\":{},\"direction_to_steer\":\"{}\",\"waypoint_id\":\"{}\",\"bearing_to_destination\":{},\"destination_id\":\"{}\",\"arrival_circle_entered\":\"{}\",\"perp_bisector_origin_id\":\"{}\",\"bearing_to_perpendicular_bisector\":{},\"heading_to_perpendicular_bisector\":\"{}\"}}",
            self.sentence_type,
            self.status,
            self.cross_track_error,
            self.direction_to_steer,
            self.waypoint_id,
            self.bearing_to_destination,
            self.destination_id,
            self.arrival_circle_entered,
            self.perp_bisector_origin_id,
            self.bearing_to_perpendicular_bisector,
            self.heading_to_perpendicular_bisector,
        )
    }
}

impl FromStr for APBSentence {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: serde_json::Value = serde_json::from_str(s).map_err(|_| ())?;
        Ok(APBSentence {
            sentence_type: v["sentence_type"].as_str().ok_or(())?.to_string(),
            status: v["status"].as_str().ok_or(())?.chars().next().ok_or(())?,
            cross_track_error: v["cross_track_error"].as_f64().ok_or(())?,
            direction_to_steer: v["direction_to_steer"].as_str().ok_or(())?.chars().next().ok_or(())?,
            waypoint_id: v["waypoint_id"].as_str().ok_or(())?.to_string(),
            bearing_to_destination: v["bearing_to_destination"].as_f64().ok_or(())?,
            destination_id: v["destination_id"].as_str().ok_or(())?.to_string(),
            arrival_circle_entered: v["arrival_circle_entered"].as_str().ok_or(())?.chars().next().ok_or(())?,
            perp_bisector_origin_id: v["perp_bisector_origin_id"].as_str().ok_or(())?.to_string(),
            bearing_to_perpendicular_bisector: v["bearing_to_perpendicular_bisector"].as_f64().ok_or(())?,
            heading_to_perpendicular_bisector: v["heading_to_perpendicular_bisector"].as_str().ok_or(())?.chars().next().ok_or(())?,
        })
    }
}

fn main() {
    let apb_sentence = APBSentence {
        sentence_type: String::from("$APB"),
        status: 'A',
        cross_track_error: 1.5,
        direction_to_steer: 'L',
        waypoint_id: String::from("WPT001"),
        bearing_to_destination: 45.0,
        destination_id: String::from("DEST001"),
        arrival_circle_entered: 'A',
        perp_bisector_origin_id: String::from("ORIGIN001"),
        bearing_to_perpendicular_bisector: 30.0,
        heading_to_perpendicular_bisector: 'T',
    };

    let nmea_sentence = apb_sentence.to_string();
    println!("NMEA APB Sentence: {}", nmea_sentence);

    // Example of parsing
    let parsed_sentence: APBSentence = nmea_sentence.parse().unwrap();
    println!("Parsed Sentence: {:?}", parsed_sentence);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apb_sentence_serialization() {
        let apb_sentence = APBSentence {
            sentence_type: String::from("$APB"),
            status: 'A',
            cross_track_error: 1.5,
            direction_to_steer: 'L',
            waypoint_id: String::from("WPT001"),
            bearing_to_destination: 45.0,
            destination_id: String::from("DEST001"),
            arrival_circle_entered: 'A',
            perp_bisector_origin_id: String::from("ORIGIN001"),
            bearing_to_perpendicular_bisector: 30.0,
            heading_to_perpendicular_bisector: 'T',
        };

        let expected_json = r#"{"sentence_type":"$APB","status":"A","cross_track_error":1.5,"direction_to_steer":"L","waypoint_id":"WPT001","bearing_to_destination":45.0,"destination_id":"DEST001","arrival_circle_entered":"A","perp_bisector_origin_id":"ORIGIN001","bearing_to_perpendicular_bisector":30.0,"heading_to_perpendicular_bisector":"T"}"#;

        let actual_json = apb_sentence.to_string();
        assert_eq!(actual_json, expected_json);
    }


    #[test]
    fn test_apb_sentence_deserialization() {
        let nmea_sentence = r#"{"sentence_type":"$APB","status":"A","cross_track_error":1.5,"direction_to_steer":"L","waypoint_id":"WPT001","bearing_to_destination":45.0,"destination_id":"DEST001","arrival_circle_entered":"A","perp_bisector_origin_id":"ORIGIN001","bearing_to_perpendicular_bisector":30.0,"heading_to_perpendicular_bisector":"T"}"#;

        let parsed_sentence: APBSentence = nmea_sentence.parse().unwrap();

        assert_eq!(parsed_sentence.sentence_type, "$APB");
        assert_eq!(parsed_sentence.status, 'A');
        assert_eq!(parsed_sentence.cross_track_error, 1.5);
        assert_eq!(parsed_sentence.direction_to_steer, 'L');
        assert_eq!(parsed_sentence.waypoint_id, "WPT001");
        assert_eq!(parsed_sentence.bearing_to_destination, 45.0);
        assert_eq!(parsed_sentence.destination_id, "DEST001");
        assert_eq!(parsed_sentence.arrival_circle_entered, 'A');
        assert_eq!(parsed_sentence.perp_bisector_origin_id, "ORIGIN001");
        assert_eq!(parsed_sentence.bearing_to_perpendicular_bisector, 30.0);
        assert_eq!(parsed_sentence.heading_to_perpendicular_bisector, 'T');
    }
}
