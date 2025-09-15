use std::io::{Cursor, Read};
use nmea::stream::NmeaStreamParser;

fn main() {
    // Example data source: a Cursor over a byte slice
    let data = b"$GPGLL,4916.45,N,12311.12,W,225444,A,*1D\r\n$GPGLL,4916.45,N,12311.12,W,225444,A,*1D\r\n";
    let mut reader = Cursor::new(data);

    let mut parser = NmeaStreamParser::new();

    let chunk_size = 10;
    let mut buffer = vec![0; chunk_size];

    // Read and process data in chunks
    while let Ok(n) = reader.read(&mut buffer) {
        if n == 0 {
            break; // End of stream
        }

        // Process the chunk and get complete messages
        let messages = parser.process_chunk(&buffer[..n]);

        // Handle each complete message
        for message in messages {
            // Convert the message from Vec<u8> to a string for display
            if let Ok(message_str) = String::from_utf8(message) {
                println!("Parsed message: {}", message_str);
            } else {
                eprintln!("Failed to convert message to string");
            }
        }
    }
}