mod can_frame;
mod error;
mod rx_buffer;
mod ser_deser;

pub use crate::filter::{NFilter, PrePFilter};
pub use can_frame::*;
pub use error::*;
pub use rx_buffer::*;
pub use ser_deser::*;

#[derive(Debug)]
pub enum ComItem {
    ClearFilters,               // Host  => Bridge              Clear all Filters
    Echo,                       // Host <=> Bridge              Test TCP communicatiion
    End,                        //          Bridge <=> Flash    End of Data
    Error(Error),               // Host <=  Bridge              Show errors
    FrameToSend(CanFrame),      // Host  => Bridge              Send Can Frame
    Magic(bool),                //          Bridge <=> Flash    Start sign
    NFilter(NFilter),           // Host <=> Bridge <=> Flash    Define NFilter 
    PFilter(PrePFilter),        // Host <=> Bridge <=> Flash    Define PFilter 
    ReceivedFrame(CanFrame),    // Host <=  Bridge              Can Frame received
    Save,                       // Host  => Bridge              Save Config to flash
    ShowFilters,                // Host  => Bridge              Show Filters 
}

impl ComItem {
    pub fn deserialize(deser: &mut impl DeSerialize) -> Result<Self, Error> {
        let slice = deser.get_slice()?;
        let r = match slice {
            b"$clearfilt" => ComItem::ClearFilters,
            b"$echo" => ComItem::Echo,
            b"$end" => ComItem::End,
            b"$err" => ComItem::Error(Error::deserialize(deser)?),
            b"$fts" => ComItem::FrameToSend(CanFrame::deserialize(deser)?),
            b"$magic" => ComItem::Magic(Magic::deserialize(deser)?),
            b"$nfilt" => ComItem::NFilter(NFilter::deserialize(deser)?),
            b"$pfilt" => ComItem::PFilter(PrePFilter::deserialize(deser)?),
            b"$rf" => ComItem::ReceivedFrame(CanFrame::deserialize(deser)?),
            b"$save" => ComItem::Save,
            b"$filt?" => ComItem::ShowFilters,
            _ => return Err(Error::ParseError),
        };
        if deser.is_end() {
            Ok(r)
        } else {
            Err(Error::ParseError)
        }
    }

    pub fn serialize(&self) -> Ser<50> {
        let mut ser = Ser::<50>::default();
        match self {
            Self::ClearFilters => ser.add_slice(b"$clearfilt").unwrap(),
            Self::Echo => ser.add_slice(b"$echo").unwrap(),
            Self::End => ser.add_slice(b"$end").unwrap(),
            Self::Error(error) => {
                ser.add_slice(b"$err").unwrap();
                error.serialize(&mut ser).unwrap();
            }
            Self::FrameToSend(frame) => {
                ser.add_slice(b"$fts").unwrap();
                frame.serialize(&mut ser).unwrap();
            }
            Self::Magic(_) => {
                ser.add_slice(b"$magic").unwrap();
                Magic::serialize(&mut ser).unwrap();
            }
            Self::NFilter(nfilter) => {
                ser.add_slice(b"$nfilt").unwrap();
                nfilter.serialize(&mut ser).unwrap();
            }
            Self::PFilter(pre_pfilter) => {
                ser.add_slice(b"$pfilt").unwrap();
                pre_pfilter.serialize(&mut ser).unwrap();
            }
            Self::ReceivedFrame(frame) => {
                ser.add_slice(b"$rf").unwrap();
                frame.serialize(&mut ser).unwrap();
            }
            Self::Save => ser.add_slice(b"$save").unwrap(),
            Self::ShowFilters => ser.add_slice(b"$filt?").unwrap(),
        }
        ser.add_byte(b'\n').unwrap();
        ser
    }
}


const MAGIC: &[u8; 8] = &[0x67, 0xa3, 0x52, 0x84, 0xe6, 0x2a, 0x4b, 0x25];
pub const MAGIC_DATAGRAM: &[u8] = b"$magic,67a35284e62a4b25\n";

struct Magic {}

impl Magic {
    pub fn deserialize(deser: &mut impl DeSerialize) -> Result<bool, Error> {
        let vec = deser.get_slice_hex()?;
        if vec.as_slice() == MAGIC {
            Ok(true)
        } else {
            Err(Error::MagicNotFound)
        }
    }

    pub fn serialize(ser: &mut impl Serialize) -> Result<(), Error> {
        ser.add_byte(b',')?;
        ser.add_slice_hex(MAGIC)?;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use crate::DeSer;

    use super::*;

    extern crate std;
    use std::println;

    #[test]
    fn ok_com_item() {
        let slice = b"$rf,12a,3,1a2b3c\n";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        let item = ComItem::deserialize(&mut deser).unwrap();
        let ser = item.serialize();
        println!("ComItem {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), slice);

        let slice = b"$fts,12a,c3,\n";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        let item = ComItem::deserialize(&mut deser).unwrap();
        let ser = item.serialize();
        println!("ComItem {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), slice);

        let slice = b"$err,EndNotFound\n";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        let item = ComItem::deserialize(&mut deser).unwrap();
        let ser = item.serialize();
        println!("ComItem {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), slice);

        let slice = b"$echo\n";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        let item = ComItem::deserialize(&mut deser).unwrap();
        let ser = item.serialize();
        println!("ComItem {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), slice);

        let slice = b"$end\n";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        let item = ComItem::deserialize(&mut deser).unwrap();
        let ser = item.serialize();
        println!("ComItem {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), slice);

        let slice = b"$clearfilt\n";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        let item = ComItem::deserialize(&mut deser).unwrap();
        let ser = item.serialize();
        println!("ComItem {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), slice);

        let slice = b"$save\n";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        let item = ComItem::deserialize(&mut deser).unwrap();
        let ser = item.serialize();
        println!("ComItem {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), slice);

        let slice = b"$magic,67a35284e62a4b25\n";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        let item = ComItem::deserialize(&mut deser).unwrap();
        let ser = item.serialize();
        println!("ComItem {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), slice);

        let slice = b"$nfilt,111_1111_0000\n";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        let item = ComItem::deserialize(&mut deser).unwrap();
        let ser = item.serialize();
        println!("ComItem {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), slice);

        let slice = b"$pfilt,17,1_1111_0000_1111_0000_11*1_000*_1111\n";
        let mut deser = DeSer::<50>::from_slice(slice).unwrap();
        let item = ComItem::deserialize(&mut deser).unwrap();
        let ser = item.serialize();
        println!("ComItem {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), slice);
    }
}
