//! This reader composes frames of bytes for Redis protocol
//! 

use std::io::{self, Read, Error, ErrorKind};
use std::collections::VecDeque;

//use help;
use value::Value;

#[derive(Debug)]
pub struct FrameReader {
    frames: Frames
}

impl FrameReader {
    pub fn new(max_frame_size: u32) -> FrameReader {
        FrameReader {
            frames: Frames::new(max_frame_size)
        }
    }

    pub fn read<T: Read>(&mut self, reader: &mut T) -> io::Result<usize> {
        self.frames.read(reader)
    }

    pub fn iter_mut(&mut self) -> Iter {
        Iter {
            frames: &mut self.frames
        }
    }
}

pub struct Iter<'a> {
    frames: &'a mut Frames
}

impl<'a> Iterator for Iter<'a> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        self.frames.completed_frames.pop_front()
    }
}

#[derive(Debug)]
struct Frames {
    max_frame_size: u32,
    bytes_read: usize,
    current: Vec<u8>,
    completed_frames: VecDeque<Value>
}

impl Frames {
    pub fn new(max_frame_size: u32) -> Frames {
       let mut buf = Vec::with_capacity(max_frame_size as usize);
       unsafe { buf.set_len(max_frame_size as usize); }

        Frames {
            max_frame_size   : max_frame_size,
            bytes_read       : 0,
            current          : buf,
            completed_frames : VecDeque::new()
        }
    }

    /// Will read as much data as possible and build up frames to be retrieved from the iterator.
    ///
    /// Will stop reading when 0 bytes are retrieved from the latest call to `do_read` or the error
    /// kind is io::ErrorKind::WouldBlock.
    ///
    /// Returns an error or the total amount of bytes read.
    fn read<T: Read>(&mut self, reader: &mut T) -> io::Result<usize> {
        let mut total_bytes_read = 0;
        loop {
            match self.do_read(reader) {
                Ok(0) => {
                    if total_bytes_read == 0 {
                        return Err(Error::new(ErrorKind::UnexpectedEof, "Read 0 bytes"));
                    }
                    return Ok(total_bytes_read);
                },
                Ok(bytes_read) => {
                    total_bytes_read += bytes_read;
                },
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    return Ok(total_bytes_read)
                }
                Err(e) => return Err(e)
            }
        }
    }

    fn do_read<T: Read>(&mut self, reader: &mut T) -> io::Result<usize> {
        let bytes_read = reader.read(&mut self.current[self.bytes_read..])?;
        self.bytes_read += bytes_read;

        //{
        //    let lines = help::hexdump( &self.current[..self.bytes_read] );
        //    for line in lines {
        //        info!("DEBUG {}", line);
        //    }
        //}

        {
            // to test if the message is complete
            let mut offset = 0;
            loop {
                let (val, _offset) = Value::decode(&self.current[..self.bytes_read], offset)?;
                if _offset == 0 {
                    break;
                }
                self.completed_frames.push_back(val);
                offset = _offset;
            }
            if offset > 0 {
                // reset buffer
                let mut k = 0;
                let mut i = offset;
                while i < self.bytes_read {
                    self.current[ k ] = self.current[ i ];
                    i += 1;
                    k += 1;
                }
                self.bytes_read = k;
            }
        }

        Ok(bytes_read)
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::io::Cursor;
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use super::FrameReader;
    use super::super::value::Value;

    #[test]
    fn partial_and_complete_reads() {
        let buf1 = String::from("+Hello World\r\n").into_bytes();
        let buf2 = String::from("-Error\r\n").into_bytes();

        let mut reader = FrameReader::new(64);

        // Write a partial value
        let mut data = Cursor::new(&buf1[0..5]);
        let bytes_read = reader.read(&mut data).unwrap();
        assert_eq!(5, bytes_read);
        assert_eq!(None, reader.iter_mut().next());

        // Complete writing the first value
        let mut data = Cursor::new(&buf1[5..]);
        let bytes_read = reader.read(&mut data).unwrap();
        assert_eq!(9, bytes_read);
        let val = reader.iter_mut().next().unwrap();
        assert_eq!(Value::Status("Hello World".to_string()), val);

        // Write an entire header and value
        let mut data = Cursor::new(Vec::with_capacity(buf1.len() + buf2.len()));
        assert_eq!(buf1.len(), data.write(&buf1).unwrap());
        assert_eq!(buf2.len(), data.write(&buf2).unwrap());
        data.set_position(0);
        let bytes_read = reader.read(&mut data).unwrap();
        assert_eq!(buf1.len() + buf2.len(), bytes_read);
        let val = reader.iter_mut().next().unwrap();
        assert_eq!(Value::Status("Hello World".to_string()), val);
        let val = reader.iter_mut().next().unwrap();
        assert_eq!(Value::Error("Error".to_string()), val);
    }

    const IP: &'static str = "127.0.0.1:5003";
    /// Test that we never get an io error, but instead get Ok(0) when the call to read would block
    #[test]
    fn would_block() {
        let listener = TcpListener::bind(IP).unwrap();
        let h = thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(mut conn) = stream {
                    conn.set_nonblocking(true).unwrap();
                    let mut reader = FrameReader::new(512);
                    //let result = reader.read(&mut conn);
                    //assert_matches!(result, Ok(0));
                    let result = reader.read(&mut conn).unwrap();
                    assert_eq!(result, 0);
                    return;
                }
            }
        });
        // Assign to a variable so the sock isn't dropped early
        // Name it with a preceding underscore so we don't get an unused variable warning
        let _sock = TcpStream::connect(IP).unwrap();
        h.join().unwrap();

    }
}
