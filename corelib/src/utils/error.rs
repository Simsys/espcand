use crate::{DeSerialize, Serialize};

/// Erros for use in this application
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    /// Error during serializing
    SerializeError,
    /// Generated when the received command is invalid
    ParseError,
    /// Datagram end char not found
    EndNotFound,
    /// The buffer is full and data has been lost.
    BufIsFull,
    /// No Data in Buffer
    BufIsEmpty,
    /// Magic number not found
    MagicNotFound,
    /// No start character found in the data stream
    NoBeginFound,
    /// Function not supported
    NotSupported,
    /// Unknown command
    UnknownCommand,
    /// Unknown error
    UnknownError,
}

impl From<&[u8]> for Error {
    fn from(value: &[u8]) -> Self {
        match value {
            b"SerializeError" => Self::SerializeError,
            b"ParseError" => Self::ParseError,
            b"EndNotFound" => Self::EndNotFound,
            b"BufIsFull" => Self::BufIsFull,
            b"BufIsEmpty" => Self::BufIsEmpty,
            b"MagicNotFound" => Self::MagicNotFound,
            b"NoBeginFound" => Self::NoBeginFound,
            b"NotSupported" => Self::NotSupported,
            b"UnknownCommand" => Self::UnknownCommand,
            _ => Self::UnknownError,
        }
    }
}

impl Error {
    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            Self::SerializeError => b"SerializeError",
            Self::ParseError => b"ParseError",
            Self::EndNotFound => b"EndNotFound",
            Self::BufIsFull => b"BufIsFull",
            Self::BufIsEmpty => b"BufIsEmpty",
            Self::MagicNotFound => b"MagicNotFound",
            Self::NoBeginFound => b"NoBeginFound",
            Self::NotSupported => b"NotSupported",
            Self::UnknownCommand => b"UnknownCommand",
            Self::UnknownError => b"UnknownError",
        }
    }

    pub fn deserialize(deser: &mut impl DeSerialize) -> Result<Self, Error> {
        let error_slice = &deser.get_slice()?[1..];
        Ok(Error::from(error_slice))
    }

    pub fn serialize(&self, ser: &mut impl Serialize) -> Result<(), Error> {
        ser.add_byte(b',')?;
        ser.add_slice(self.as_bytes())
    }
}
