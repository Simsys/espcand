#![allow(dead_code)]

use core::fmt::{Display, Formatter};

use embedded_can::{ExtendedId, Frame, Id, StandardId};
use heapless::Vec;
use modular_bitfield::{
    bitfield,
    specifiers::{B2, B4},
};

use crate::{DeSerialize, Error, Serialize};

pub type Vec8 = Vec<u8, 8>;
pub type Vec30 = Vec<u8, 30>;

#[bitfield]
#[derive(Debug, Clone, Copy, PartialEq)]
struct Info {
    dlc: B4,
    #[allow(non_snake_case)]
    _empty: B2,
    remote: bool,
    extended: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
            mydata[..l].copy_from_slice(data);

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
            Some(CanFrame { id, info, data })
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
        let mut info = Info::new().with_extended(extended);
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
        CanFrame { id, info, data }
    }

    pub fn deserialize(deser: &mut impl DeSerialize) -> Result<Self, Error> {
        let id = deser.get_u32_hex()?;
        let info = Info::from_bytes([deser.get_u32_hex()? as u8]);

        let vec = deser.get_slice_hex()?;
        if !info.remote() && vec.len() != info.dlc() as usize {
            return Err(Error::ParseError);
        }
        let mut data = [0_u8; 8];
        data[..vec.len()].copy_from_slice(&vec);
        Ok(Self { id, info, data })
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

impl Display for CanFrame {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut ser = crate::Ser::<30>::default();
        let _ = self.serialize(&mut ser);
        let _ = write!(f, "CanFrame{}", str::from_utf8(ser.as_slice()).unwrap());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{DeSer, Ser};

    use super::*;

    extern crate std;
    use std::println;

    #[test]
    fn ok_can_frames() {
        let slice = b",12a,3,1a2b3c,";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        let frame = CanFrame::deserialize(&mut deser).unwrap();
        let mut ser = Ser::<40>::default();
        frame.serialize(&mut ser).unwrap();
        println!("frame {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), &slice[..slice.len() - 1]);

        let slice = b",12a4,88,1a2b3c4d5e6f7081,";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        let frame = CanFrame::deserialize(&mut deser).unwrap();
        let mut ser = Ser::<40>::default();
        frame.serialize(&mut ser).unwrap();
        println!("frame {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), &slice[..slice.len() - 1]);

        let slice = b",12a,2,1a2b3c,";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        assert_eq!(CanFrame::deserialize(&mut deser), Err(Error::ParseError));
    }
}
