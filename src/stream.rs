use std::io::Read;

// an iterator-based parser that will parse new messages
// from e.g. streamed/received bytes when \r\n ending is present.
//
// we should 'chunkinate' the bytes based on the \r\n ending

pub struct NmeaStreamParser {
    buffer: Vec<u8>,
    separator: Vec<u8>,
}

impl NmeaStreamParser {
    pub fn new() -> Self {
        NmeaStreamParser {
            buffer: Vec::new(),
            separator: b"\r\n".to_vec(),
        }
    }

    pub fn process_chunk(&mut self, chunk: &[u8]) -> Vec<Vec<u8>> {
        self.buffer.extend_from_slice(chunk);
        let mut messages = Vec::new();

        while let Some(pos) = self
            .buffer
            .windows(self.separator.len())
            .position(|window| window == self.separator.as_slice())
        {
            let message = self.buffer.drain(..pos).collect();
            messages.push(message);
            self.buffer.drain(..self.separator.len());
        }

        messages
    }
}

#[allow(dead_code)]
struct MessageStream<R: Read> {
    reader: R,
    parser: NmeaStreamParser,
    chunk_size: usize,
}

impl<R: Read> MessageStream<R> {
    #[allow(dead_code)]
    fn new(reader: R, chunk_size: usize) -> Self {
        MessageStream {
            reader,
            parser: NmeaStreamParser::new(),
            chunk_size,
        }
    }
}

impl<R: Read> Iterator for MessageStream<R> {
    type Item = Result<Vec<u8>, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut chunk = vec![0; self.chunk_size];
        loop {
            match self.reader.read(&mut chunk) {
                Ok(0) => return None, // End of stream
                Ok(n) => {
                    let mut messages = self.parser.process_chunk(&chunk[..n]);
                    if !messages.is_empty() {
                        return Some(Ok(messages.remove(0)));
                    }
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_parser() {
        let mut parser = NmeaStreamParser::new();
        let chunk = b"$GPGLL,4916.45,N,12311.12,W,225444,A,*1D\r\n$GPGLL,4916.45,N,12311.12,W,225444,A,*1D\r\n";
        let messages = parser.process_chunk(chunk);
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_stream_parser_partial_message() {
        let mut parser = NmeaStreamParser::new();
        let chunk1 = b"$GPGLL,4916.45,N,123";
        let chunk2 = b"11.12,W,225444,A,*1D\r\n";

        let messages = parser.process_chunk(chunk1);
        assert_eq!(messages.len(), 0);

        let messages = parser.process_chunk(chunk2);
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn test_stream_parser_multiple_chunks() {
        let mut parser = NmeaStreamParser::new();
        let chunks = [
            b"$GPGLL,4916.45,N,12311.12,W,2254", 
            b"44,A,*1D\r\n$GPGLL,4916.45,N,12300",
            b"11.12,W,225444,A,*1D\r\n$GPGLL,4..", 
            b"916.45,N,12311.12,W,225444,A,*1D", 
            b"\r\n$GPGLL,4916.45,N,12311.12,W,..", 
            b"225444,A,*1D\r\n..................", 
            b"$GPGLL,4916.45,N,12311.12,W,2254",
            b"44,A,*1D\r\n$GPGLL,4916.45,N,123..", 
            b"11.12,W,225444,A,*1D\r\n..........", 
        ];

        let mut total_messages = 0;
        for chunk in chunks {
            let messages = parser.process_chunk(chunk);
            total_messages += messages.len();
        }
        assert_eq!(total_messages, 6);
    }

    #[test]
    fn test_stream_parser_invalid_data() {
        let mut parser = NmeaStreamParser::new();
        let chunk = b"Invalid data without separators";
        let messages = parser.process_chunk(chunk);
        assert_eq!(messages.len(), 0);
    }

    #[test]
    fn test_stream_parser_empty_chunk() {
        let mut parser = NmeaStreamParser::new();
        let messages = parser.process_chunk(b"");
        assert_eq!(messages.len(), 0);
    }

    #[test]
    fn test_message_stream() {
        let data = b"$GPGLL,4916.45,N,12311.12,W,225444,A,*1D\r\n$GPGLL,4916.45,N,12311.12,W,225444,A,*1D\r\n";
        let stream = MessageStream::new(&data[..], 10);
        let messages: Vec<Result<Vec<u8>, std::io::Error>> = stream.collect();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_empty_message_stream() {
        let data = b"";
        let stream = MessageStream::new(&data[..], 10);

        // check that result is ok
        for message in stream {
            assert!(message.is_ok());
        }
    }
}
