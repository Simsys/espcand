use embassy_time::Instant;
use crate::{Error, Serialize};

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
pub struct IdTimes<const CAP: usize> {
    id_times: [IdTime; CAP],
}

impl<const CAP: usize>  IdTimes<CAP> {
    pub fn new() -> Self {
        let id_time = IdTime { id: u32::MAX, instant: TInstant(0) };       
        Self { id_times: [id_time; CAP] } 
    }

    pub fn check_instant(&mut self, id: u32, instant: TInstant, duration: u32) -> bool {
        if duration == 0 {
            return true;
        }
        for id_time in &mut self.id_times {
            if id_time.id == u32::MAX {
                id_time.id = id;
                id_time.instant = instant;
                return true
            } else if id_time.id == id {
                if id_time.instant.dist(instant) >= duration {
                    id_time.instant = instant;
                    return true
                } else {
                    return false
                }
            }
        }
        false // silently ignore ids, when id-buffer is fullcl
    } 
}

pub fn check(id: u32, ones: u32, zeros: u32, extended: bool) -> bool {
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

pub fn get_ones_zeros(bytes: &[u8]) -> Result<(bool, u32, u32), Error> {
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

pub fn add_ones_zeros(
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
