mod can_frame;
mod error;
mod ringbuffer;
mod ser_deser;

pub use can_frame::*;
pub use error::*;
pub use ringbuffer::*;
pub use ser_deser::*;


pub enum ComItem {
    FrameToSend(CanFrame),
    ReceivedFrame(CanFrame),
}

impl ComItem {
    pub fn deserialize(deser: &mut impl DeSerialize) -> Result<Self, Error> {
        let slice = deser.get_slice()?;
        let r = match slice {
            b"$FTS" => ComItem::FrameToSend(CanFrame::deserialize(deser)?),
            b"$RF" => ComItem::ReceivedFrame(CanFrame::deserialize(deser)?),
            _ => return Err(Error::ParseError),
        };
        if deser.is_end() {
            Ok(r)
        } else {
            Err(Error::ParseError)
        }
    }

    pub fn serialize(&self) -> Ser<30> {
        let mut ser = Ser::<30>::new();
        match self {
            Self::FrameToSend(frame) => {
                ser.add_slice(b"$FTS").unwrap();
                frame.serialize(&mut ser).unwrap();
            }
            Self::ReceivedFrame(frame) => {
                ser.add_slice(b"$RF").unwrap();
                frame.serialize(&mut ser).unwrap();
            }
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
        let slice = b"$RF,12a,3,1a2b3c\n";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        let item = ComItem::deserialize(&mut deser).unwrap();
        let ser = item.serialize();
        println!("ComItem {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), slice);

        let slice = b"$FTS,12a,c3,\n";
        let mut deser = DeSer::<40>::from_slice(slice).unwrap();
        let item = ComItem::deserialize(&mut deser).unwrap();
        let ser = item.serialize();
        println!("ComItem {}", str::from_utf8(ser.as_slice()).unwrap());
        assert_eq!(ser.as_slice(), slice);
    }

}