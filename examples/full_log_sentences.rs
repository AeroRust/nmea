use std::fs::read_to_string;

use log::{error, info};
use nmea::{parse, ParseResult};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init()
        .expect("Failed to init logger");

    let full_log = read_to_string("examples/full_log_sentences.log")?;

    let all_lines_count = full_log.lines().count();

    let sentences_parsed = full_log
        .lines()
        .enumerate()
        .filter_map(|(line_index, line)| {
            if line.is_empty() {
                return None;
            }

            match parse(line.as_bytes()) {
                Ok(parse_result) => Some(parse_result),
                Err(err) => {
                    error!("Error parsing sentence on line {line_index}:\n{line}\n\n Error: {err}");
                    None
                }
            }
        })
        .collect::<Vec<ParseResult>>();

    info!("Successfully parsed {sentences_parsed_len} sentences out of {all_lines_count} total lines:\n", sentences_parsed_len = sentences_parsed.len());

    info!("{:#?}", sentences_parsed);

    assert_eq!(
        518,
        sentences_parsed.len(),
        "Sentences should be 518 in total!"
    );

    Ok(())
}
