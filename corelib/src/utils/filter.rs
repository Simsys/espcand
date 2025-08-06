use core::u32;

use heapless::Vec;
use crate::{DeSerialize, Error, Serialize};

#[derive(PartialEq, Clone, Copy, Debug)]
struct IdTime {
    pub id: u32,
    pub instant: u64,
}

#[derive(PartialEq, Clone, Copy, Debug)]
struct IdTimes<const CAP: usize> {
    id_times: [IdTime; CAP],
}

impl<const CAP: usize>  IdTimes<CAP> {
    fn new() -> Self {
        let id_time = IdTime { id: u32::MAX, instant: 0 };       
        Self { id_times: [id_time; CAP] } 
    }

    fn check_instant(&mut self, id: u32, instant: u64, duration: u64) -> bool {
        if duration == 0 {
            return true;
        }
        for id_time in &mut self.id_times {
            if id_time.id == u32::MAX {
                id_time.id = id;
                id_time.instant = instant;
                return false
            } else if id_time.id == id {
                if id_time.instant + duration < instant {
                    id_time.instant = instant;
                    return true
                } else {
                    return false
                }
            }
        }
        false // silently ignor, that id-buffer is full and id will be ignored
    } 
}

fn check(id: u32, ones: u32, zeros: u32, extended: bool) -> bool {
    let mut r = true;
    let p1 = (id ^ if extended {
        0b1_1111_1111_1111_1111_1111_1111_1111
    } else {
        0b111_1111_1111
    }) & zeros;
    if p1 != zeros {
        r = false;
    }

    let p2 = id & ones;
    if p2 != ones {
        r = false;
    }
    r
}

fn get_ones_zeros(bytes: &[u8]) -> Result<(bool, u32, u32), Error> {
    let mut bit_cnt = 0_usize;
    for b in bytes {
        match *b {
            b'0' | b'1' | b'*' => bit_cnt += 1,
            b'_' => (),
            _  => return Err(Error::ParseError),
        }
    }
    let extended = if bit_cnt == 11 {
        false
    } else if bit_cnt == 29 {
        true
    } else {
        return Err(Error::ParseError);
    };

    let mut ones = 0_u32;
    let mut zeros = 0_u32;
    for b in bytes {
        if *b != b'_' {
            ones <<= 1;
            zeros <<= 1;
        }
        match *b {
            b'0' => zeros |= 1,
            b'1' => ones |= 1,
            _  => (),
        }
    }
    Ok((extended, ones, zeros))
}


#[derive(PartialEq, Debug)]
pub struct PFilter {
    extended: bool,
    duration: u64,
    ones: u32,
    zeros: u32,
    id_times: IdTimes<32>
}

impl PFilter {
    pub fn new(
        duration: u64,
        bytes: &[u8]
    ) -> Result<Self, Error> {
        let (extended, ones, zeros) = get_ones_zeros(bytes)?;
        Ok(Self { extended, duration, ones, zeros, id_times: IdTimes::new() })
    }

    pub fn check(&mut self, id: u32, instant: u64) -> bool {
        if !self.id_times.check_instant(id, instant, self.duration) {
            return false
        }
        check(id, self.ones, self.zeros, self.extended)
    }

    pub fn deserialize(deser: &mut impl DeSerialize) -> Result<Self, Error> {
        let extended = deser.get_bool()?;
        let duration = deser.get_slice_hex()?;
        let mut buf = [0_u8; 8];
        buf.copy_from_slice(&duration[..duration.len()]);
        let duration = u64::from_le_bytes(buf);
        let ones = deser.get_u32_hex()?;
        let zeros = deser.get_u32_hex()?;
        Ok(Self { extended, duration, ones, zeros, id_times: IdTimes::new() })
    }

    pub fn serialize(&self, ser: &mut impl Serialize) -> Result<(), Error> {
        ser.add_byte(b',')?;
        ser.add_bool(self.extended)?;
        ser.add_byte(b',')?;
        ser.add_slice_hex(&self.duration.to_le_bytes())?;
        ser.add_byte(b',')?;
        ser.add_uint_hex(self.ones, 0)?;
        ser.add_byte(b',')?;
        ser.add_uint_hex(self.zeros, 0)?;
        ser.add_byte(b',')
    }
}


pub struct PFilters<const CAP: usize> {
    pfilters: Vec<PFilter, CAP>,
}

impl<const CAP: usize> PFilters<CAP> {
    pub fn new() -> Self {
        Self { pfilters: Vec::new() }
    }

    pub fn add(&mut self, duration: u64, bytes: &[u8]) -> Result<(), Error> {
        let pfilter = PFilter::new(duration, bytes)?;
        self.pfilters.push(pfilter).map_err(|_| Error::BufIsFull)
    }

    pub fn check(&mut self, id: u32, instant: u64) -> bool {
        if self.pfilters.len() == 0 {
            return true
        } else {
            for pfilter in &mut self.pfilters {
                if pfilter.check(id, instant) {
                    return true
                }
            }
        }
        false
    }
}


#[derive(PartialEq, Debug)]
pub struct NFilter {
    extended: bool,
    ones: u32,
    zeros: u32,
}

impl NFilter {
    pub fn new(
        bytes: &[u8]
    ) -> Result<Self, Error> {
        let (extended, ones, zeros) = get_ones_zeros(bytes)?;
        Ok(Self { extended, ones, zeros })
    }

    pub fn check(&mut self, id: u32) -> bool {
        check(id, self.ones, self.zeros, self.extended)
    }
}


pub struct NFilters<const CAP: usize> {
    nfilters: Vec<NFilter, CAP>,
}

impl<const CAP: usize> NFilters<CAP> {
    pub fn new() -> Self {
        Self { nfilters: Vec::new() }
    }

    pub fn add(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let nfilter = NFilter::new(bytes)?;
        self.nfilters.push(nfilter).map_err(|_| Error::BufIsFull)
    }

    pub fn check(&mut self, id: u32) -> bool {
        if self.nfilters.len() == 0 {
            return false
        } else {
            for nfilter in &mut self.nfilters {
                if nfilter.check(id) {
                    return true
                }
            }
        }
        false
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    //extern crate std;
    //use std::println;

    #[test]
    fn new_pfilter() {
        assert_eq!(PFilter::new(0, b"asdf"), Err(Error::ParseError));
        assert_eq!(PFilter::new(0, b"1100"), Err(Error::ParseError));
        assert_eq!(PFilter::new(0, b"11_*00"), Err(Error::ParseError));
        assert_eq!(
            PFilter::new(0, b"110_0110_0011"), 
            Ok(PFilter {
                extended: false,
                duration: 0,
                ones: 0b110_0110_0011,
                zeros: 0b1_1001_1100,
                id_times: IdTimes::new(),
            })
        );
        assert_eq!(
            PFilter::new(0, b"1*0_0110_0**1"), 
            Ok(PFilter {
                extended: false,
                duration: 0,
                ones: 0b100_0110_0001,
                zeros: 0b1_1001_1000,
                id_times: IdTimes::new(),
            })
        );
        assert_eq!(PFilter::new(0, b"1*0_0110_0**1_*"), Err(Error::ParseError));
        assert_eq!(
            PFilter::new(123, b"1_0000_1111_0000_1111_0000_1111_0000"), 
            Ok(PFilter {
                extended: true,
                duration: 123,
                ones: 0b1_0000_1111_0000_1111_0000_1111_0000,
                zeros: 0b1111_0000_1111_0000_1111_0000_1111,
                id_times: IdTimes::new(),
            })
        );
    }

    #[test]
    fn new_nfilter() {
        assert_eq!(NFilter::new(b"asdf"), Err(Error::ParseError));
        assert_eq!(NFilter::new(b"1100"), Err(Error::ParseError));
        assert_eq!(NFilter::new(b"11_*00"), Err(Error::ParseError));
        assert_eq!(
            NFilter::new(b"110_0110_0011"), 
            Ok(NFilter {
                extended: false,
                ones: 0b110_0110_0011,
                zeros: 0b1_1001_1100,
            })
        );
        assert_eq!(
            NFilter::new(b"1*0_0110_0**1"), 
            Ok(NFilter {
                extended: false,
                ones: 0b100_0110_0001,
                zeros: 0b1_1001_1000,
            })
        );
        assert_eq!(NFilter::new(b"1*0_0110_0**1_*"), Err(Error::ParseError));
        assert_eq!(
            NFilter::new(b"1_0000_1111_0000_1111_0000_1111_0000"), 
            Ok(NFilter {
                extended: true,
                ones: 0b1_0000_1111_0000_1111_0000_1111_0000,
                zeros: 0b1111_0000_1111_0000_1111_0000_1111,
            })
        );
    }

    #[test]
    fn check_pfilter() {
        let mut filter = PFilter::new(0, b"1*0_0110_0**1").unwrap();
        assert_eq!(filter.check(0b110_0110_0111, 0), true);
        assert_eq!(filter.check(0b100_0110_0001, 0), true);
        assert_eq!(filter.check(0b110_0110_0110, 0), false);
        assert_eq!(filter.check(0b110_0110_1111, 0), false);

        let mut filter = PFilter::new(1000, b"1*0_0110_0**1").unwrap();
        assert_eq!(filter.check(0b110_0110_0111, 500), false);
        assert_eq!(filter.check(0b110_0110_0111, 1000), false);
        assert_eq!(filter.check(0b110_0110_0111, 1501), true);

        assert_eq!(filter.check(0b100_0110_0001, 500), false);
        assert_eq!(filter.check(0b100_0110_0001, 1000), false);
        assert_eq!(filter.check(0b100_0110_0001, 1501), true);

        let mut filter = PFilter::new(0, b"1_0000_1111_0000_1111_0000_1111_0000").unwrap();
        assert_eq!(filter.check(0b1_0000_1111_0000_1111_0000_1111_0000, 0), true);
    }

    #[test]
    fn check_pfilters() {
        let mut pfilters = PFilters::<10>::new();
        pfilters.add(0, b"110_0110_0000").unwrap();
        pfilters.add(0, b"110_0110_0001").unwrap();
        assert_eq!(pfilters.check(0b110_0110_0000, 0), true);
        assert_eq!(pfilters.check(0b110_0110_0001, 0), true);
        assert_eq!(pfilters.check(0b110_0110_0011, 0), false);
    }
    
    #[test]
    fn check_nfilter() {
        let mut filter = NFilter::new(b"1*0_0110_0**1").unwrap();
        assert_eq!(filter.check(0b110_0110_0111), true);
        assert_eq!(filter.check(0b100_0110_0001), true);
        assert_eq!(filter.check(0b110_0110_0110), false);
        assert_eq!(filter.check(0b110_0110_1111), false);

        let mut filter = NFilter::new(b"1*0_0110_0**1").unwrap();
        assert_eq!(filter.check(0b110_0110_0111), true);
        assert_eq!(filter.check(0b100_0110_0001), true);
        assert_eq!(filter.check(0b110_0110_0111), true);
        assert_eq!(filter.check(0b100_0110_0001), true);

        let mut filter = NFilter::new(b"1_0000_1111_0000_1111_0000_1111_0000").unwrap();
        assert_eq!(filter.check(0b1_0000_1111_0000_1111_0000_1111_0000), true);
    }

    #[test]
    fn check_nfilters() {
        let mut nfilters = NFilters::<10>::new();
        nfilters.add(b"110_0110_0000").unwrap();
        nfilters.add(b"110_0110_0001").unwrap();
        assert_eq!(nfilters.check(0b110_0110_0000), true);
        assert_eq!(nfilters.check(0b110_0110_0001), true);
        assert_eq!(nfilters.check(0b110_0110_0011), false);
    }
    
}