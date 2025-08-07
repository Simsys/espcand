use embassy_time::Instant;
use embedded_can::Id;
use heapless::Vec;
use crate::{DeSerialize, Error, Serialize};

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct TInstant(u32);

impl TInstant {
    fn dist(&self, other: TInstant) -> u32 {
        let d1 = self.0.wrapping_sub(other.0);
        let d2 = other.0.wrapping_sub(self.0);
        if d1 < d2 {
            d1
        } else {
            d2
        }
    }
}

impl From<Instant> for TInstant {
    fn from(value: Instant) -> Self {
        Self(value.as_millis() as u32)
    }
}

impl From<i32> for TInstant {
    fn from(value: i32) -> Self {
        Self(value as u32)
    }
}


#[derive(PartialEq, Clone, Copy, Debug)]
struct IdTime {
    pub id: u32,
    pub instant: TInstant,
}

#[derive(PartialEq, Clone, Copy, Debug)]
struct IdTimes<const CAP: usize> {
    id_times: [IdTime; CAP],
}

impl<const CAP: usize>  IdTimes<CAP> {
    fn new() -> Self {
        let id_time = IdTime { id: u32::MAX, instant: TInstant(0) };       
        Self { id_times: [id_time; CAP] } 
    }

    fn check_instant(&mut self, id: u32, instant: TInstant, duration: u32) -> bool {
        if duration == 0 {
            return true;
        }
        for id_time in &mut self.id_times {
            if id_time.id == u32::MAX {
                id_time.id = id;
                id_time.instant = instant;
                return false
            } else if id_time.id == id {
                if id_time.instant.dist(instant) >= duration {
                    id_time.instant = instant;
                    return true
                } else {
                    return false
                }
            }
        }
        false // silently ignore, that id-buffer is full and id will be ignored
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

fn add_ones_zeros(
    ser: &mut impl Serialize,
    extended: bool,
    mut ones: u32,
    mut zeros: u32,
) {
    let mut q: heapless::Deque<u8, 40> = heapless::Deque::new();
    let len = if extended {
        29
    } else {
        11
    };
    let mut idx = 0;
    while idx < len {
        idx += 1;
        let one = ones & 0x01;
        let zero = zeros & 0x01;
        let b = if one == 1 {
            b'1'
        } else if zero == 1 {
            b'0'
        } else {
            b'*'
        };
        ones >>= 1;
        zeros >>= 1;
        q.push_front(b).unwrap();
        if idx & 0x03 == 0 {
            q.push_front(b'_').unwrap();
        }
    }
    loop {
        match q.pop_front() {
            Some(b) => ser.add_byte(b).unwrap(),
            None => break,
        }
    }
}


#[derive(PartialEq, Debug, Copy, Clone)]
pub struct PFilter {
    extended: bool,
    duration: u32,
    ones: u32,
    zeros: u32,
    id_times: IdTimes<8>
}

impl PFilter {
    pub fn new(
        duration: u32,
        bytes: &[u8]
    ) -> Result<Self, Error> {
        let (extended, ones, zeros) = get_ones_zeros(bytes)?;
        Ok(Self { extended, duration, ones, zeros, id_times: IdTimes::new() })
    }

    pub fn check(&mut self, id: Id, instant: TInstant) -> bool {
        let id = match id {
            Id::Extended(id) => if self.extended {
                id.as_raw()
            } else {
                return false
            }
            Id::Standard(id) => if self.extended {
                return false
            } else {
                id.as_raw() as u32
            }
        };
        if !self.id_times.check_instant(id, instant, self.duration) {
            return false
        }
        check(id, self.ones, self.zeros, self.extended)
    }

    pub fn deserialize(deser: &mut impl DeSerialize) -> Result<Self, Error> {
        let duration = deser.get_u32()?;
        let slice = &deser.get_slice()?[1..];
        let (extended, ones, zeros) = get_ones_zeros(slice)?;
        Ok(Self { extended, duration, ones, zeros, id_times: IdTimes::new() })
    }

    pub fn serialize(&self, ser: &mut impl Serialize) -> Result<(), Error> {
        ser.add_byte(b',')?;
        ser.add_uint(self.duration)?;
        ser.add_byte(b',')?;
        add_ones_zeros(ser, self.extended, self.ones, self.zeros);
        Ok(())
    }
}




pub struct PFilters<const CAP: usize> {
    pfilters: Vec<PFilter, CAP>,
}

impl<const CAP: usize> PFilters<CAP> {
    pub fn new() -> Self {
        Self { pfilters: Vec::new() }
    }

    pub fn add(&mut self, pfilter: PFilter) -> Result<(), Error> {
        self.pfilters.push(pfilter).map_err(|_| Error::BufIsFull)
    }

    pub fn check(&mut self, id: Id, instant: Instant) -> bool {
        let instant = instant.into();
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

    pub fn clear(&mut self) {
        self.pfilters.clear();
    }
}


#[derive(PartialEq, Debug, Copy, Clone)]
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

    pub fn check(&mut self, id: Id) -> bool {
        let id = match id {
            Id::Extended(id) => if self.extended {
                id.as_raw()
            } else {
                return false
            }
            Id::Standard(id) => if self.extended {
                return false
            } else {
                id.as_raw() as u32
            }
        };
        check(id, self.ones, self.zeros, self.extended)
    }

    pub fn deserialize(deser: &mut impl DeSerialize) -> Result<Self, Error> {
        let slice = &deser.get_slice()?[1..];
        let (extended, ones, zeros) = get_ones_zeros(slice)?;
        Ok(Self { extended, ones, zeros })
    }

    pub fn serialize(&self, ser: &mut impl Serialize) -> Result<(), Error> {
        ser.add_byte(b',')?;
        add_ones_zeros(ser, self.extended, self.ones, self.zeros);
        Ok(())
    }
}


pub struct NFilters<const CAP: usize> {
    nfilters: Vec<NFilter, CAP>,
}

impl<const CAP: usize> NFilters<CAP> {
    pub fn new() -> Self {
        Self { nfilters: Vec::new() }
    }

    pub fn add(&mut self, nfilter: NFilter) -> Result<(), Error> {
        self.nfilters.push(nfilter).map_err(|_| Error::BufIsFull)
    }

    pub fn check(&mut self, id: Id) -> bool {
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

    pub fn clear(&mut self) {
        self.nfilters.clear();
    }
}


#[cfg(test)]
mod tests {
    use embedded_can::{ExtendedId, StandardId};

    use super::*;
    use crate::{Ser, DeSer};
    extern crate std;
    use std::println;

    fn s_id(id: u32) -> Id {
        Id::Standard(StandardId::new(id as u16).unwrap())
    }

    fn e_id(id: u32) -> Id {
        Id::Extended(ExtendedId::new(id).unwrap())
    }

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
        assert_eq!(filter.check(s_id(0b110_0110_0111), 0.into()), true);
        assert_eq!(filter.check(s_id(0b100_0110_0001), 0.into()), true);
        assert_eq!(filter.check(s_id(0b110_0110_0110), 0.into()), false);
        assert_eq!(filter.check(s_id(0b110_0110_1111), 0.into()), false);

        let mut filter = PFilter::new(1000, b"1*0_0110_0**1").unwrap();
        assert_eq!(filter.check(s_id(0b110_0110_0111), 500.into()), false);
        assert_eq!(filter.check(s_id(0b110_0110_0111), 1000.into()), false);
        assert_eq!(filter.check(s_id(0b110_0110_0111), 1501.into()), true);

        assert_eq!(filter.check(s_id(0b100_0110_0001), 500.into()), false);
        assert_eq!(filter.check(s_id(0b100_0110_0001), 1000.into()), false);
        assert_eq!(filter.check(s_id(0b100_0110_0001), 1501.into()), true);

        let mut filter = PFilter::new(0, b"1_0000_1111_0000_1111_0000_1111_0000").unwrap();
        assert_eq!(filter.check(e_id(0b1_0000_1111_0000_1111_0000_1111_0000), 0.into()), true);
    }

    #[test]
    fn check_pfilters() {
        let mut pfilters = PFilters::<10>::new();
        let filter = PFilter::new(0, b"110_0110_0000").unwrap();
        pfilters.add(filter).unwrap();
        let filter = PFilter::new(0, b"110_0110_0001").unwrap();
        pfilters.add(filter).unwrap();
        assert_eq!(pfilters.check(s_id(0b110_0110_0000), Instant::from_millis(0)), true);
        assert_eq!(pfilters.check(s_id(0b110_0110_0001), Instant::from_millis(0)), true);
        assert_eq!(pfilters.check(s_id(0b110_0110_0011), Instant::from_millis(0)), false);
    }
    
    #[test]
    fn check_nfilter() {
        let mut filter = NFilter::new(b"1*0_0110_0**1").unwrap();
        assert_eq!(filter.check(s_id(0b110_0110_0111)), true);
        assert_eq!(filter.check(s_id(0b100_0110_0001)), true);
        assert_eq!(filter.check(s_id(0b110_0110_0110)), false);
        assert_eq!(filter.check(s_id(0b110_0110_1111)), false);

        let mut filter = NFilter::new(b"1*0_0110_0**1").unwrap();
        assert_eq!(filter.check(s_id(0b110_0110_0111)), true);
        assert_eq!(filter.check(s_id(0b100_0110_0001)), true);
        assert_eq!(filter.check(s_id(0b110_0110_0111)), true);
        assert_eq!(filter.check(s_id(0b100_0110_0001)), true);

        let mut filter = NFilter::new(b"1_0000_1111_0000_1111_0000_1111_0000").unwrap();
        assert_eq!(filter.check(e_id(0b1_0000_1111_0000_1111_0000_1111_0000)), true);
    }

    #[test]
    fn check_nfilters() {
        let mut nfilters = NFilters::<10>::new();
        let filter = NFilter::new(b"110_0110_0000").unwrap();
        nfilters.add(filter).unwrap();
        let filter = NFilter::new(b"110_0110_0001").unwrap();
        nfilters.add(filter).unwrap();
        assert_eq!(nfilters.check(s_id(0b110_0110_0000)), true);
        assert_eq!(nfilters.check(s_id(0b110_0110_0001)), true);
        assert_eq!(nfilters.check(s_id(0b110_0110_0011)), false);
    }
    
    #[test]
    fn nfilter_serialize() {
        let slice = b",111_1111_0000,";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        let nfilter = NFilter::deserialize(&mut deser).unwrap();
        let mut ser = Ser::<40>::new();
        nfilter.serialize(&mut ser).unwrap();
        println!("nfilter {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), &slice[..slice.len()-1]);

        let slice = b",1_1111_0000_1111_0000_1111_0000_1111,";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        let nfilter = NFilter::deserialize(&mut deser).unwrap();
        let mut ser = Ser::<40>::new();
        nfilter.serialize(&mut ser).unwrap();
        println!("nfilter {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), &slice[..slice.len()-1]);
    }

    #[test]
    fn pfilter_serialize() {
        let slice = b",17,1_1111_0000_1111_0000_11*1_000*_1111,";
        let mut deser = DeSer::<50>::from_slice(slice).unwrap();
        let pfilter = PFilter::deserialize(&mut deser).unwrap();
        let mut ser = Ser::<40>::new();
        pfilter.serialize(&mut ser).unwrap();
        println!("pfilter {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), &slice[..slice.len()-1]);

    }
}