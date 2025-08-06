use crate::{Error, Vec8};
use heapless::Vec;

pub trait Serialize {
    fn add_bool(&mut self, b: bool) -> Result<(), Error> where Self: Sized;
    fn add_byte(&mut self, b: u8) -> Result<(), Error> where Self: Sized;
    fn add_slice(&mut self, slice: &[u8]) -> Result<(), Error> where Self: Sized;
    fn add_slice_hex(&mut self, slice: &[u8]) -> Result <(), Error> where Self: Sized;
    fn add_uint(&mut self, i: impl Into<u32>) -> Result<(), Error> where Self: Sized;
    fn add_uint_hex(&mut self, i: impl Into<u32>, pad_len: usize) -> Result<(), Error> where Self: Sized;
    fn as_slice(&self) -> &[u8];
    fn len(&self) -> usize;
}

pub struct Ser<const CAP: usize> {
    buf: Vec<u8, CAP>,
}

impl<const CAP: usize> Ser<CAP> {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }
}

impl<const CAP: usize> Serialize for Ser<CAP> {
    fn add_bool(&mut self, b: bool) -> Result<(), Error> {
        self.buf.push(b as u8 + b'0').map_err(|_| Error::SerializeError)
    }

    fn add_byte(&mut self, b: u8) -> Result<(), Error> {
        self.buf.push(b).map_err(|_| Error::SerializeError)
    }

    fn add_slice(&mut self, slice: &[u8]) -> Result<(), Error> {
        for b in slice {
            self.buf.push(*b).map_err(|_| Error::SerializeError)?;
        }
        Ok(())
    }

    fn add_slice_hex(&mut self, slice: &[u8]) -> Result <(), Error> {
        #[inline]
        fn to_x(b: u8) -> u8 {
            if b > 9 {
                b - 10 + b'a'
            } else {
                b + b'0'
            }
        }
        for b in slice {
            self.add_byte(to_x(*b/16))?;
            self.add_byte(to_x(*b%16))?;
        }
        Ok(())
    }

    fn add_uint(&mut self, i: impl Into<u32>) -> Result<(), Error> {
        const IBUF_LEN: usize = 10;
        let mut ibuf = [0_u8; IBUF_LEN];
        let mut i = i.into() as u32;

        let mut idx = IBUF_LEN;
        loop {
            idx -= 1;
            ibuf[idx] = (i % 10) as u8 + b'0';
            i = i / 10;
            if i == 0 {                
                break
            }
            if idx == 0 {
                return Err(Error::SerializeError);
            }
        }
        self.buf.extend_from_slice(&ibuf[idx..IBUF_LEN]).map_err(|_| Error::SerializeError)?;
        Ok(())
    }

    fn add_uint_hex(&mut self, i: impl Into<u32>, pad_len: usize) -> Result<(), Error> {
        const IBUF_LEN: usize = 9;
        let mut ibuf = [0_u8; IBUF_LEN];
        let mut i = i.into();
        let mut idx = IBUF_LEN;
        loop {
            idx -= 1;
            let c = (i % 16) as u8;
            if c > 9 {
                ibuf[idx] = c + b'a' - 10;
            } else {
                ibuf[idx] = c + b'0';
            }
            i = i / 16;
            if i == 0 && (IBUF_LEN - idx) >= pad_len {
                break
            }
            if idx == 0 {
                return Err(Error::SerializeError);
            }
        }
        self.buf.extend_from_slice(&ibuf[idx..IBUF_LEN]).map_err(|_| Error::SerializeError)?;
        Ok(())
    }

    fn as_slice(&self) -> &[u8] {
        self.buf.as_slice()
    }

    fn len(&self) -> usize {
        self.buf.len()
    }
}


pub trait DeSerialize {
    fn as_slice(&self) -> &[u8];    fn capacity(&self) -> usize;
    fn get_bool(&mut self) -> Result<bool, Error>;
    fn get_slice(&mut self) -> Result<&[u8], Error>;
    fn get_slice_hex(&mut self) -> Result<Vec8, Error>;
    fn get_u32(&mut self) -> Result<u32, Error>;
    fn get_u32_hex(&mut self) -> Result<u32, Error>;
    fn is_end(&self) -> bool;
    fn push(&mut self, b: u8) -> Result<(), Error>;
}

pub struct DeSer<const CAP: usize> {
    vec: Vec<u8, CAP>,
    head: usize,
    is_end: bool,
}

impl<const CAP: usize> DeSer<CAP> {
    pub fn new() -> Self {
        Self { vec: Vec::new(), head: 0, is_end: false }
    }

    pub fn from_slice(slice: &[u8]) -> Result<Self, Error> {
        let mut vec = Vec::<u8, CAP>::new();
        vec.extend_from_slice(slice).map_err(|_| Error::NotSupported)?;
        Ok(Self { vec, head: 0, is_end: false })

    }

    pub fn clear(&mut self) {
        self.vec.clear();
    }

    pub fn extend_from_slice(&mut self, src: &[u8]) -> Result<(), Error> {
        self.vec.extend_from_slice(src).map_err(|_| Error::BufIsFull)
    }
}

impl<const CAP: usize> DeSerialize for DeSer<CAP> {
    fn as_slice(&self) -> &[u8] {
        self.vec.as_slice()
    }

    fn capacity(&self) -> usize {
        CAP
    }

    fn get_bool(&mut self) -> Result<bool, Error> {
        let slice = &self.get_slice()?[1..];
        if slice.len() == 1 {
            Ok(slice[0] == b'1')
        } else {
            Err(Error::ParseError)
        }
    }

    fn get_slice(&mut self) -> Result<&[u8], Error> {
        let start = self.head;
        loop {
            self.head += 1;
            if self.head >= self.vec.len() {
                return Err(Error::ParseError)
            }
            let b = self.vec[self.head];
            if b == b',' || b == b'\n' {
                if b == b'\n' {
                    self.is_end = true;
                }
                return Ok(&self.vec[start..self.head]);
            }
        }
    }

    fn get_slice_hex(&mut self) -> Result<Vec8, Error> {
        fn get_nibble(b: u8) -> Result<u8, Error> {
            match b {
                b'0'..=b'9' => Ok(b - b'0'),
                b'a'..=b'f' => Ok(b - b'a' + 10),
                _ => return Err(Error::ParseError)
            }
        }
        // slice has at least a len of 1 
        let slice = &self.get_slice()?[1..];

        if slice.len() & 0x01 == 1 {
            return Err(Error::ParseError)
        }
        let mut idx = 0;
        let mut vec = Vec8::new();
        while idx < slice.len() {
            let b = get_nibble(slice[idx])? * 16 + get_nibble(slice[idx+1])?;
            vec.push(b).map_err(|_| Error::ParseError)?;
            idx += 2;
        }
        Ok(vec)
    }

    fn get_u32(&mut self) -> Result<u32, Error> {
        let slice = &self.get_slice()?[1..];
        let mut r = 0_u32;
        for b in slice {
            r *=10;
            match *b {
                b'0'..=b'9' => r += (*b - b'0') as u32,
                _ => return Err(Error::ParseError)
                 
            }
        }
        Ok(r)
    }

    fn get_u32_hex(&mut self) -> Result<u32, Error> {
        let slice = &self.get_slice()?[1..];
        let mut r = 0_u32;
        for b in slice {
            r *=16;
            match *b {
                b'0'..=b'9' => r += (*b - b'0') as u32,
                b'a'..=b'f' => r += (*b - b'a' + 10) as u32,
                _ => return Err(Error::ParseError)
                 
            }
        }
        Ok(r)
    }

    fn is_end(&self) -> bool {
        self.is_end
    }

    fn push(&mut self, b: u8) -> Result<(), Error> {
        self.vec.push(b).map_err(|_| Error::BufIsFull)
    }
}






#[cfg(test)]
mod tests {
    use super::*;

    extern crate std;
    use core::u32;
    use std::println;

    #[test]
    fn ok_deser_simple() {
        let mut de_ser = DeSer::<40>::new();
        de_ser.extend_from_slice(b"$123,456,789\n").unwrap();
        assert_eq!(de_ser.get_slice().unwrap(), b"$123");
        assert_eq!(de_ser.is_end, false);
        assert_eq!(de_ser.get_slice().unwrap(), b",456");
        assert_eq!(de_ser.is_end, false);
        assert_eq!(de_ser.get_slice().unwrap(), b",789");
        assert_eq!(de_ser.is_end, true);

        let mut de_ser = DeSer::<40>::new();
        de_ser.extend_from_slice(b",1a2b,456,1a2b3c4d5e6f7081\n").unwrap();
        assert_eq!(de_ser.get_u32_hex().unwrap(), 0x1a2b);
        assert_eq!(de_ser.is_end, false);
        assert_eq!(de_ser.get_u32().unwrap(), 456);
        assert_eq!(de_ser.is_end, false);
        assert_eq!(de_ser.get_slice_hex().unwrap().as_slice(), b"\x1a\x2b\x3c\x4d\x5e\x6f\x70\x81");
        assert_eq!(de_ser.is_end, true);

        let mut de_ser = DeSer::<40>::new();
        de_ser.extend_from_slice(b",1a2x,45a,001a2b3c4d5e6f7081,1\n").unwrap();
        assert_eq!(de_ser.get_u32_hex(), Err(Error::ParseError));
        assert_eq!(de_ser.is_end, false);
        assert_eq!(de_ser.get_u32(), Err(Error::ParseError));
        assert_eq!(de_ser.is_end, false);
        assert_eq!(de_ser.get_slice_hex(), Err(Error::ParseError));
        assert_eq!(de_ser.is_end, false);
        assert_eq!(de_ser.get_slice_hex(), Err(Error::ParseError));
        assert_eq!(de_ser.is_end, true);

        let mut de_ser = DeSer::<40>::new();
        de_ser.extend_from_slice(b",a2,\n").unwrap();
        assert_eq!(de_ser.get_slice_hex().unwrap().as_slice(), b"\xa2");
        assert_eq!(de_ser.get_slice_hex().unwrap().as_slice(), b"");
    }

    #[test]
    fn ok_ser_simple() {
        let mut ser: Ser<40> = Ser::new();
        ser.add_byte(b'c').unwrap();
        assert_eq!(ser.as_slice(), b"c");

        let mut ser: Ser<40> = Ser::new();
        ser.add_slice(b"Hello world").unwrap();
        assert_eq!(ser.as_slice(), b"Hello world");

        let mut ser: Ser<40> = Ser::new();
        ser.add_uint(4711_u32).unwrap();
        assert_eq!(ser.as_slice(), b"4711");

        let mut ser: Ser<40> = Ser::new();
        ser.add_uint(0_u32).unwrap();
        assert_eq!(ser.as_slice(), b"0");

        let mut ser: Ser<40> = Ser::new();
        ser.add_uint(u32::MAX).unwrap();
        println!("{}", u32::MAX);
        assert_eq!(ser.as_slice(), b"4294967295");

        let mut ser: Ser<40> = Ser::new();
        ser.add_uint_hex(0x3a4b_u32, 6).unwrap();
        assert_eq!(ser.as_slice(), b"003a4b");

        let mut ser: Ser<40> = Ser::new();
        ser.add_uint_hex(0_u32, 0).unwrap();
        assert_eq!(ser.as_slice(), b"0");

        let mut ser: Ser<40> = Ser::new();
        ser.add_uint_hex(u32::MAX, 0).unwrap();
        assert_eq!(ser.as_slice(), b"ffffffff");

        let mut ser: Ser<40> = Ser::new();
        ser.add_slice_hex(b"\x1a\x2b\x3c").unwrap();
        assert_eq!(ser.as_slice(), b"1a2b3c");
    }


}