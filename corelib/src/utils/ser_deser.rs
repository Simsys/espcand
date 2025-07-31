use crate::utils::Error;
use heapless::Vec;

pub trait Serialize {
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
    fn add_byte(&mut self, b: u8) -> Result<(), Error> {
        self.buf.push(b).map_err(|_| Error::SerializeError)?;
        Ok(())
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


#[cfg(test)]
mod tests {
    use super::*;

    extern crate std;
    use core::u32;
    use std::println;

    #[test]
    fn ok_simple() {
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