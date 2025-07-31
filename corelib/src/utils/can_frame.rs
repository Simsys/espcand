#![allow(dead_code)]

use embedded_can::{ExtendedId, Frame, Id, StandardId};
use heapless::Vec;
use modular_bitfield::{bitfield, specifiers::{B4, B2}};

use crate::{Serialize, Error};

pub type Vec8 = Vec<u8, 8>;
pub type Vec40 = Vec<u8, 40>;

#[bitfield]
struct Info {
    dlc: B4,
    #[allow(non_snake_case)]
    _empty: B2,
    remote: bool,
    extended: bool,
}

pub struct CanFrame {
    id: u32,
    info: Info,
    data: [u8; 8],
}

impl embedded_can::Frame for CanFrame {

    fn new(id: impl Into<Id>, data: &[u8]) -> Option<Self> {
        let id: Id = id.into();
        let (id, extended) = match id {
            Id::Standard(id) => (id.as_raw() as u32, false),
            Id::Extended(id) => (id.as_raw(), true),
        };
        let l = data.len();
        if l > 8 {
            None
        } else {
            let info = Info::new()
                .with_extended(extended)
                .with_remote(false)
                .with_dlc(l as u8);
            let mut mydata = [0_u8; 8];
            mydata[..l].copy_from_slice(&data); 

            Some(CanFrame {
                id,
                info,
                data: mydata,
            })
        }
    }

    fn new_remote(id: impl Into<Id>, dlc: usize) -> Option<Self> {
        let id: Id = id.into();
        let (id, extended) = match id {
            Id::Standard(id) => (id.as_raw() as u32, false),
            Id::Extended(id) => (id.as_raw(), true),
        };
        if dlc > 8 {
            None
        } else {
            let info = Info::new()
                .with_extended(extended)
                .with_remote(true)
                .with_dlc(dlc as u8);
            let data = [0_u8; 8];
            Some(CanFrame {
                id,
                info,
                data,
            })
        }
    }

    fn is_extended(&self) -> bool {
        self.info.extended()
    }

    fn is_remote_frame(&self) -> bool {
        self.info.remote()
    }

    fn id(&self) -> Id {
        if self.info.extended() {
            Id::Extended(ExtendedId::new(self.id).unwrap())
        } else {
            Id::Standard(StandardId::new(self.id as u16).unwrap())
        }
    }

    fn dlc(&self) -> usize {
        self.info.dlc() as usize
    }

    fn data(&self) -> &[u8] {
        &self.data[..self.info.dlc() as usize]
    }
}

impl CanFrame {
    pub fn from_frame(frame: impl Frame) -> Self {
        let (id, extended) = match frame.id() {
            Id::Standard(id) => (id.as_raw() as u32, false),
            Id::Extended(id) => (id.as_raw(), true),
        };
        let mut info = Info::new()
            .with_extended(extended);
        let mut data = [0_u8; 8];

        if frame.is_remote_frame() {
            info.set_remote(true);
            info.set_dlc(frame.dlc() as u8);
        } else {
            info.set_remote(false);
            let l = frame.data().len();
            info.set_dlc(l as u8);
            data[..l].copy_from_slice(frame.data());
        }
        CanFrame {
            id,
            info,
            data
        }
    }

    pub fn serialize(&self, ser: &mut impl Serialize) -> Result<(), Error> {
        let info = self.info.bytes[0];
        let l = self.info.dlc() as usize;
        ser.add_byte(b',')?;
        ser.add_uint_hex(self.id, 0)?;
        ser.add_byte(b',')?;
        ser.add_uint_hex(info, 0)?;
        ser.add_byte(b',')?;
        if !self.info.remote() {
            ser.add_slice_hex(&self.data[..l])?;
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use crate::Ser;

    use super::*;

    extern crate std;
    use std::println;

    #[test]
    fn ok_can_frames() {
        let id = StandardId::new(0x12a).unwrap();
        let frame = CanFrame::new(id, b"\x1a\x2b\x3c").unwrap();
        let mut ser = Ser::<40>::new();
        frame.serialize(&mut ser).unwrap();
        println!("frame {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), b",12a,3,1a2b3c");

        let id = ExtendedId::new(0x12a4).unwrap();
        let frame = CanFrame::new(id, b"\x1a\x2b\x3c\x4d\x5e\x6f\x70\x81").unwrap();
        let mut ser = Ser::<40>::new();
        frame.serialize(&mut ser).unwrap();
        println!("frame {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), b",12a4,88,1a2b3c4d5e6f7081");

        let id = StandardId::new(0xaa).unwrap();
        let frame = CanFrame::new_remote(id, 5).unwrap();
        let mut ser = Ser::<40>::new();
        frame.serialize(&mut ser).unwrap();
        println!("frame {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), b",aa,45,");

        let id = StandardId::new(0x12a).unwrap();
        let frame = CanFrame::new(id, b"\x1a\x2b\x3c").unwrap();
        let mut ser = Ser::<40>::new();
        ser.add_slice(b"$FR").unwrap();
        frame.serialize(&mut ser).unwrap();
        ser.add_byte(b'\n').unwrap();
        println!("frame {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), b"$FR,12a,3,1a2b3c\n");
    }

}