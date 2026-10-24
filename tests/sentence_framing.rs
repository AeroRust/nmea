use nmea::{Error, SentenceType};
use nom::error::{Error as NomError, ErrorKind};

const SENTENCE: &str = "$GPTXT,01,01,01,hello*2C";

#[test]
fn one_sentence_with_a_normal_terminator_preserves_every_field() {
    for terminator in ["", "\n", "\r\n", "\r"] {
        let input = format!("{SENTENCE}{terminator}");
        assert_eq!(
            nmea::parse_nmea_sentence(&input).map(|sentence| (
                sentence.talker_id,
                sentence.message_id,
                sentence.data,
                sentence.checksum,
            )),
            Ok(("GP", SentenceType::TXT, "01,01,01,hello", 0x2C)),
        );
    }
}

#[test]
fn concatenated_sentences_and_garbage_return_exact_trailing_input() {
    for (suffix, remaining) in [
        (SENTENCE, SENTENCE),
        ("garbage", "garbage"),
        ("\r\ngarbage", "garbage"),
        ("\ngarbage", "garbage"),
        ("\n\n", "\n"),
        (" \n", " \n"),
    ] {
        let input = format!("{SENTENCE}{suffix}");
        let expected = Err(Error::ParsingError(nom::Err::Error(NomError::new(
            remaining,
            ErrorKind::Eof,
        ))));
        assert_eq!(nmea::parse_str(&input), expected);
        assert_eq!(
            nmea::parse_nmea_sentence(&input).map(|sentence| sentence.data),
            Err(Error::ParsingError(nom::Err::Error(NomError::new(
                remaining,
                ErrorKind::Eof
            )))),
        );
    }
}

#[test]
fn sentence_length_error_still_precedes_parsing() {
    let input = "x".repeat(nmea::SENTENCE_MAX_LEN + 1);
    assert_eq!(
        nmea::parse_str(&input),
        Err(Error::SentenceLength(input.len()))
    );
}
