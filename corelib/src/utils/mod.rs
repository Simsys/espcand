mod can_frame;
mod error;
mod rx_buffer;
mod ser_deser;

pub use can_frame::*;
pub use error::*;
pub use crate::filter::{NFilter, PFilter};
pub use rx_buffer::*;
pub use ser_deser::*;

#[derive(Debug)]
pub enum ComItem {
    ClearFilters,
    Echo,
    Error(Error),
    FrameToSend(CanFrame),
    NFilter(NFilter),
    PFilter(PFilter),
    ReceivedFrame(CanFrame),
    ShowFilters,
}

impl ComItem {
    pub fn deserialize(deser: &mut impl DeSerialize) -> Result<Self, Error> {
        let slice = deser.get_slice()?;
        let r = match slice {
            b"$clearfilt" => ComItem::ClearFilters,
            b"$echo" => ComItem::Echo,
            b"$err" => ComItem::Error(Error::deserialize(deser)?),
            b"$fts" => ComItem::FrameToSend(CanFrame::deserialize(deser)?),
            b"$nfilt" => ComItem::NFilter(NFilter::deserialize(deser)?),
            b"$pfilt" => ComItem::PFilter(PFilter::deserialize(deser)?),
            b"$rf" => ComItem::ReceivedFrame(CanFrame::deserialize(deser)?),
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
        let mut ser = Ser::<50>::new();
        match self {
            Self::ClearFilters => ser.add_slice(b"$clearfilt").unwrap(),
            Self::Echo => ser.add_slice(b"$echo").unwrap(),
            Self::Error(error) => {
                ser.add_slice(b"$err").unwrap();
                error.serialize(&mut ser).unwrap();
            }
            Self::FrameToSend(frame) => {
                ser.add_slice(b"$fts").unwrap();
                frame.serialize(&mut ser).unwrap();
            }
            Self::NFilter(nfilter) => {
                ser.add_slice(b"$nfilt").unwrap();
                nfilter.serialize(&mut ser).unwrap();
            }
            Self::PFilter(pfilter) => {
                ser.add_slice(b"$pfilt").unwrap();
                pfilter.serialize(&mut ser).unwrap();
            }
            Self::ReceivedFrame(frame) => {
                ser.add_slice(b"$rf").unwrap();
                frame.serialize(&mut ser).unwrap();
            }
            Self::ShowFilters => ser.add_slice(b"$filt?").unwrap(),
        }
        ser.add_byte(b'\n').unwrap();
        ser
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

        let slice = b"$clearfilt\n";
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