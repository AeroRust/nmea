
use std::io::{self, BufRead};

#[derive(Debug)]
struct ABP {
    latitude: f64,
    longitude: f64,
    altitude: f64,
    fix_type: u8,
    satellites_used: u8,
}

fn parse_abp(sentence: &str) -> Option<ABP> {
    let fields: Vec<&str> = sentence.split(',').collect();

    if fields.len() >= 8 && fields[0] == "$GPABP" {
        let latitude = fields[1].parse().ok()?;
        let longitude = fields[3].parse().ok()?;
        let altitude = fields[5].parse().ok()?;
        let fix_type = fields[7].parse().ok()?;
        let satellites_used = fields[8].parse().ok()?;

        Some(ABP {
            latitude,
            longitude,
            altitude,
            fix_type,
            satellites_used,
        })
    } else {
        None
    }
}

fn main() {
    let input_data = "$GPABP,4807.038,N,01131.000,E,545.4,M,MODE*31";

    match parse_abp(input_data) {
        Some(abp_data) => println!("{:?}", abp_data),
        None => println!("Invalid ABP sentence"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_abp() {
        let input_data = "$GPABP,4807.038,N,01131.000,E,545.4,M,1,8*31";
        let abp_data = parse_abp(input_data).expect("Failed to parse ABP sentence");

        assert_eq!(abp_data.latitude, 4807.038);
        assert_eq!(abp_data.longitude, 1131.0);
        assert_eq!(abp_data.altitude, 545.4);
        assert_eq!(abp_data.fix_type, 1);
        assert_eq!(abp_data.satellites_used, 8);
    }

    #[test]
    fn test_parse_invalid_abp() {
        let input_data = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47";
        let abp_data = parse_abp(input_data);

        assert!(abp_data.is_none());
    }
}
