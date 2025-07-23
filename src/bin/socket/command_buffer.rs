use crate::{
    Error, Duration,
    socket::command_parser::{Vec8, Vec40},
};
use heapless::Deque;

/// Buffer for parsing commands sent to espcand
///
/// CommandBuffer is a ring buffer that contains the commands received by espcand via the TCP
/// interface. The functions of this crate parse the received data. The buffer contains a byte
/// stream with all information. UTF8 characters are not supported. They are also not provided
/// for in the protocol.
pub struct CommandBuffer {
    buf: Deque<u8, 1024>,
}

impl CommandBuffer {
    /// Create a nue commandBuffer
    pub const fn new() -> CommandBuffer {
        CommandBuffer { buf: Deque::new() }
    }

    /// Return the number of characters contained in the buffer
    pub const fn len(&self) -> usize {
        self.buf.len()
    }

    /// Append additional data
    pub fn append(&mut self, s: &[u8]) -> Result<(), Error> {
        for b in s {
            self.buf.push_back(*b).map_err(|_| Error::BufIsFull)?;
        }
        Ok(())
    }

    /// Return the next string. The space is assumed to be the separator.
    pub fn get_vec(&mut self) -> Result<Vec40, Error> {
        let mut s = Vec40::new();
        loop {
            match self.buf.pop_front() {
                None => return Err(Error::NotFound),
                Some(b) => {
                    if b == b' ' {
                        return Ok(s);
                    } else {
                        s.push(b).map_err(|_| Error::BufIsFull)?;
                    }
                }
            }
        }
    }

    /// Interpret the next element as a Boolean
    pub fn get_bool(&mut self) -> Result<bool, Error> {
        let s = self.get_vec()?;
        match s.as_slice() {
            b"0" | b"false" => Ok(false),
            b"1" | b"true" => Ok(true),
            _ => Err(Error::ParseError),
        }
    }

    /// Interpret the next elements as a byte stream with 0 to 8 bytes
    ///
    /// This is how the data of a CAN bus telegram is represented.
    pub fn get_data(&mut self) -> Result<Vec8, Error> {
        let l = self.get_u32()?;
        let mut v = Vec8::new();
        match l {
            0..=8 => {
                for _i in 0..l {
                    v.push(self.find_u8_hex()?).map_err(|_| Error::ParseError)?;
                }
            }
            _ => return Err(Error::ParseError),
        }
        Ok(v)
    }

    /// Interpret the next two elements as duration with a resolution of microseconds
    pub fn get_duration(&mut self) -> Result<Duration, Error> {
        let secs = self.get_u32()?;
        let usecs = self.get_u32()?;
        Ok(Duration::from_secs(secs) + Duration::from_usecs(usecs))
    }

    /// Interpret the next elements as mux data (see protocol definition)
    pub fn get_mux_data(&mut self) -> Result<Vec40, Error> {
        let l = self.get_u32()?;
        let mut v = Vec40::new();
        for _i in 0..(l * 8) {
            v.push(self.find_u8_hex()?).map_err(|_| Error::ParseError)?;
        }
        Ok(v)
    }

    /// Interpret the next element as u32 integer
    ///
    /// Note: excessively large numbers lead to undefined states
    pub fn get_u32(&mut self) -> Result<u32, Error> {
        let bytes = self.get_vec()?;
        if bytes.len() == 0 {
            return Err(Error::ParseError);
        }
        let mut r = 0_u32;
        for b in bytes {
            match b {
                b'0'..=b'9' => r = 10 * r + (b - b'0') as u32,
                _ => return Err(Error::ParseError),
            }
        }
        Ok(r)
    }

    /// Interpret the next element as u16 integer
    ///
    /// Note: excessively large numbers lead to undefined states
    pub fn get_u16(&mut self) -> Result<u16, Error> {
        Ok(self.get_u32()? as u16)
    }

    /// Search for the start of a command
    ///
    /// This start is defined by the characters “< ”. All characters before the “<” are ignored
    /// in case there are remnants of other telegrams in the buffer.
    pub fn is_begin(&mut self) -> Result<(), Error> {
        let mut start_sign_found = false;
        loop {
            match self.buf.pop_front() {
                None => return Err(Error::NoBeginFound),
                Some(b) => {
                    if start_sign_found {
                        if b == b' ' {
                            return Ok(());
                        } else {
                            return Err(Error::ParseError);
                        }
                    } else {
                        if b == b'<' {
                            start_sign_found = true;
                        }
                    }
                }
            }
        }
    }

    /// Check the end of the command
    ///
    /// Note: The end of a command is defined by the “>” character. Additional spaces before it
    /// are interpreted as errors.
    pub fn is_end(&mut self) -> Result<(), Error> {
        if self.buf.pop_front() == Some(b'>') {
            Ok(())
        } else {
            Err(Error::NotFound)
        }
    }

    fn find_u8_hex(&mut self) -> Result<u8, Error> {
        fn get_digit(b: u8) -> Result<u8, Error> {
            match b {
                b'0'..=b'9' => Ok(b - b'0'),
                b'a'..=b'f' => Ok(b - b'a' + 10),
                b'A'..=b'F' => Ok(b - b'A' + 10),
                _ => Err(Error::ParseError),
            }
        }

        let bytes = self.get_vec()?;
        let l = bytes.len();
        if l == 0 || l > 2 {
            return Err(Error::ParseError);
        }
        let mut b = get_digit(bytes[0])?;
        if l == 2 {
            b = b * 16 + get_digit(bytes[1])?;
        }
        Ok(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_cases() {
        let mut buf = CommandBuffer::new();
        let r = buf.append(b"< testcmd 123 456 789 1 >");
        assert!(r == Ok(()));
        assert!(buf.len() == 25);
        assert!(buf.is_begin() == Ok(()));
        let r = buf.get_vec();
        assert!(r.unwrap().as_slice() == b"testcmd");
        let r = buf.get_u32();
        assert!(r.unwrap() == 123);
        let r = buf.get_u16();
        assert!(r.unwrap() == 456);
        let r = buf.get_vec();
        assert!(r.unwrap().as_slice() == b"789");
        let r = buf.get_bool();
        assert!(r.unwrap() == true);
        assert!(buf.is_end() == Ok(()));
        assert!(buf.len() == 0);

        let r = buf.append(b"< 123 456 >");
        assert!(r == Ok(()));
        assert!(buf.len() == 11);
        assert!(buf.is_begin() == Ok(()));
        let r = buf.get_duration().unwrap();
        assert!(r.usecs() == 123_000_456);
        assert!(buf.is_end() == Ok(()));
        assert!(buf.len() == 0);

        let r = buf.append(b"< 3 3f a2 5 >");
        assert!(r == Ok(()));
        assert!(buf.len() == 13);
        assert!(buf.is_begin() == Ok(()));
        let r = buf.get_data().unwrap();
        assert!(r.len() == 3);
        assert!(r[0] == 0x3f);
        assert!(r[1] == 0xa2);
        assert!(r[2] == 0x05);
        assert!(buf.is_end() == Ok(()));
        assert!(buf.len() == 0);

        let r = buf.append(b"< 8 1 2 3 4 5 6 7 ff >");
        assert!(r == Ok(()));
        assert!(buf.len() == 22);
        assert!(buf.is_begin() == Ok(()));
        let r = buf.get_data().unwrap();
        assert!(r.len() == 8);
        assert!(r[0] == 0x01);
        assert!(r[1] == 0x02);
        assert!(r[7] == 0xff);
        assert!(buf.is_end() == Ok(()));
        assert!(buf.len() == 0);
    }

    #[test]
    fn err_cases() {
        let mut buf = CommandBuffer::new();
        let r = buf.append(b"12345678");
        assert!(r == Ok(()));
        let r = buf.get_vec();
        assert!(r == Err(Error::NotFound));

        let s = b"0123456789";
        let mut buf = CommandBuffer::new();
        let mut r = Ok(());
        for _ in 0..103 {
            r = buf.append(s);
        }
        assert!(r == Err(Error::BufIsFull));

        let s = b"0123456789";
        let mut buf = CommandBuffer::new();
        for _ in 0..5 {
            buf.append(s).unwrap();
        }
        buf.append(b" ").unwrap();
        let r = buf.get_vec();
        assert!(r == Err(Error::BufIsFull));

        let s = b"0123456789";
        let mut buf = CommandBuffer::new();
        let _ = buf.append(s);
        assert!(buf.is_begin() == Err(Error::NoBeginFound));

        let s = b"< ";
        let mut buf = CommandBuffer::new();
        let _ = buf.append(s);
        assert!(buf.is_begin() == Ok(()));
        assert!(buf.is_end() == Err(Error::NotFound));
    }
}
