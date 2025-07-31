/// Erros for use in this application
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    /// Error during serializing
    SerializeError,
    /// Generated when the received command is invalid
    ParseError,
    /// Item was not found
    NotFound,
    /// The buffer is full and data has been lost.
    BufIsFull,
    /// No start character found in the data stream
    NoBeginFound,
    /// Function not supported
    NotSupported,
}
