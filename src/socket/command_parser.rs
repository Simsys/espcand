use crate::socket::command_buffer::CommandBuffer;
use crate::{Error, Duration};
use heapless::Vec;

pub type Vec8 = Vec<u8, 8>;
pub type Vec40 = Vec<u8, 40>;

/// Commands recognized by the parser
#[derive(Clone, PartialEq, Debug)]
pub enum Commands {
    None,
    Echo,
    CanTiming(CanTiming),
    CanControl(CanControl),
    Add(Add),
    Update(Update),
    Delete(Delete),
    Send(Send),
    Filter(Filter),
    MuxFilter(MuxFilter),
    Subscribe(Subscribe),
    Unsubscribe(Unsubscribe),
    Statistics(Statistics),
}

/// Working modes of espcand
#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    NoBus,
    Bcm,
    Raw,
    Control,
    IsoTp,
}

/// Parser for all commands received
///
/// The data received is evaluated and, if necessary, the commands found are output. Extensive
/// error handling ensures that no invalid commands are accepted.
/// 
/// ```
/// let mut cmd_parser = CommandParser::new();
/// let mut buf = CommandBuffer::new();
/// buf.append(b"< open can0 >< filter 0 0 123 0 >< send 124 8 11 22 33 44 55 66 7a 8b >")
///     .unwrap();
///
/// let _ = cmd_parser.parse(&mut buf);
/// assert!(cmd_parser.mode() == Mode::Bcm);
///
/// let cmd = cmd_parser.parse(&mut buf).unwrap();
/// assert!(
///     cmd == Commands::Filter(Filter {
///         duration: Duration::from_secs(0),
///         id: 123,
///         data: Vec8::from_slice(b"").unwrap()
///     })
/// );
///
/// let cmd = cmd_parser.parse(&mut buf).unwrap();
/// assert!(
///     cmd == Commands::Send(Send {
///         id: 124,
///         data: Vec8::from_slice(b"\x11\x22\x33\x44\x55\x66\x7a\x8b").unwrap()
///     })
/// );
/// assert!(buf.len() == 0);
/// ```
pub struct CommandParser {
    mode: Mode,
}

impl CommandParser {
    /// Create a new parser
    pub fn new() -> CommandParser {
        CommandParser { mode: Mode::NoBus }
    }

    /// Parse the data contained inside the cmd buffer
    pub fn parse(&mut self, buf: &mut CommandBuffer) -> Result<Commands, Error> {
        buf.is_begin()?;
        let r = match self.mode {
            Mode::NoBus => self.parse_no_bus(buf)?,
            Mode::Bcm => self.parse_bcm(buf)?,
            Mode::Raw => self.parse_raw(buf)?,
            Mode::Control => self.parse_control(buf)?,
            Mode::IsoTp => self.parse_iso_tp(buf)?,
        };
        buf.is_end()?;
        Ok(r)
    }

    /// Return current operating mode
    pub fn mode(&self) -> Mode {
        self.mode
    }

    fn parse_no_bus(&mut self, buf: &mut CommandBuffer) -> Result<Commands, Error> {
        let r = buf.get_vec()?;
        match r.as_slice() {
            ECHO => Ok(Commands::Echo),
            CAN_0 => {
                let sc = buf.get_vec()?;
                match sc.as_slice() {
                    b"B" => Ok(Commands::CanTiming(CanTiming::try_from(buf)?)),
                    b"C" => Ok(Commands::CanControl(CanControl::try_from(buf)?)),
                    _ => Err(Error::ParseError),
                }
            }
            OPEN => {
                if buf.get_vec()?.as_slice() != b"can0" {
                    return Err(Error::ParseError);
                }
                self.set_mode(Mode::Bcm)
            }
            _ => Err(Error::ParseError),
        }
    }

    fn parse_bcm(&mut self, buf: &mut CommandBuffer) -> Result<Commands, Error> {
        let r = buf.get_vec()?;
        match r.as_slice() {
            ECHO => Ok(Commands::Echo),
            ADD => Ok(Commands::Add(Add::try_from(buf)?)),
            UPDATE => Ok(Commands::Update(Update::try_from(buf)?)),
            DELETE => Ok(Commands::Delete(Delete::try_from(buf)?)),
            SEND => Ok(Commands::Send(Send::try_from(buf)?)),
            FILTER => Ok(Commands::Filter(Filter::try_from(buf)?)),
            MUX_FILTER => Ok(Commands::MuxFilter(MuxFilter::try_from(buf)?)),
            SUBSCRIBE => Ok(Commands::Subscribe(Subscribe::try_from(buf)?)),
            UNSUBSCRIBE => Ok(Commands::Unsubscribe(Unsubscribe::try_from(buf)?)),

            RAW_MODE => self.set_mode(Mode::Raw),
            CONTROL_MODE => self.set_mode(Mode::Control),
            ISO_TP_MODE => self.set_mode(Mode::IsoTp),
            _ => Err(Error::ParseError),
        }
    }

    fn parse_raw(&mut self, buf: &mut CommandBuffer) -> Result<Commands, Error> {
        let r = buf.get_vec()?;
        match r.as_slice() {
            ECHO => Ok(Commands::Echo),
            SEND => Ok(Commands::Send(Send::try_from(buf)?)),

            BCM_MODE => self.set_mode(Mode::Bcm),
            CONTROL_MODE => self.set_mode(Mode::Control),
            ISO_TP_MODE => self.set_mode(Mode::IsoTp),
            _ => Err(Error::ParseError),
        }
    }

    fn parse_control(&mut self, buf: &mut CommandBuffer) -> Result<Commands, Error> {
        let r = buf.get_vec()?;
        match r.as_slice() {
            ECHO => Ok(Commands::Echo),
            STATISTICS => Ok(Commands::Statistics(Statistics::try_from(buf)?)),

            BCM_MODE => self.set_mode(Mode::Bcm),
            RAW_MODE => self.set_mode(Mode::Raw),
            ISO_TP_MODE => self.set_mode(Mode::IsoTp),
            _ => Err(Error::ParseError),
        }
    }

    fn parse_iso_tp(&mut self, buf: &mut CommandBuffer) -> Result<Commands, Error> {
        let r = buf.get_vec()?;
        match r.as_slice() {
            ECHO => Ok(Commands::Echo),
            PDU => Err(Error::NotSupported),
            SENDPDU => Err(Error::NotSupported),

            BCM_MODE => self.set_mode(Mode::Bcm),
            RAW_MODE => self.set_mode(Mode::Raw),
            CONTROL_MODE => self.set_mode(Mode::IsoTp),
            _ => Err(Error::ParseError),
        }
    }

    fn set_mode(&mut self, mode: Mode) -> Result<Commands, Error> {
        self.mode = mode;
        Ok(Commands::None)
    }
}

// Modes
const OPEN: &[u8] = b"open";
const BCM_MODE: &[u8] = b"bcmmode";
const RAW_MODE: &[u8] = b"rawmode";
const CONTROL_MODE: &[u8] = b"controlmode";
const ISO_TP_MODE: &[u8] = b"isotpmode";

const ECHO: &[u8] = b"echo";
const CAN_0: &[u8] = b"can0";
const ADD: &[u8] = b"add";
const UPDATE: &[u8] = b"update";
const DELETE: &[u8] = b"delete";
const SEND: &[u8] = b"send";
const FILTER: &[u8] = b"filter";
const MUX_FILTER: &[u8] = b"muxfilter";
const SUBSCRIBE: &[u8] = b"subscribe";
const UNSUBSCRIBE: &[u8] = b"unsubscribe";
const STATISTICS: &[u8] = b"statistics";
const SENDPDU: &[u8] = b"sendpdu";
const PDU: &[u8] = b"pdu";

/// Contents of the CanTiming command
#[allow(unused)]
#[derive(Clone, PartialEq, Debug)]
pub struct CanTiming {
    pub bit_rate: u32,
    pub sample_point: u16,
    pub tq: u16,
    pub prop_seg: u16,
    pub phase_seg1: u16,
    pub phase_seg2: u16,
    pub sjw: u16,
    pub brp: u16,
}

impl TryFrom<&mut CommandBuffer> for CanTiming {
    type Error = Error;

    fn try_from(buf: &mut CommandBuffer) -> Result<Self, Self::Error> {
        let bit_rate = buf.get_u32()?;
        let sample_point = buf.get_u16()?;
        let tq = buf.get_u16()?;
        let prop_seg = buf.get_u16()?;
        let phase_seg1 = buf.get_u16()?;
        let phase_seg2 = buf.get_u16()?;
        let sjw = buf.get_u16()?;
        let brp = buf.get_u16()?;
        Ok(CanTiming {
            bit_rate,
            sample_point,
            tq,
            prop_seg,
            phase_seg1,
            phase_seg2,
            sjw,
            brp,
        })
    }
}

/// Contents of the CanControl command
#[allow(unused)]
#[derive(Clone, PartialEq, Debug)]
pub struct CanControl {
    pub listen_only: bool,
    pub loopback: bool,
    pub three_samples: bool,
}

impl TryFrom<&mut CommandBuffer> for CanControl {
    type Error = Error;

    fn try_from(buf: &mut CommandBuffer) -> Result<Self, Self::Error> {
        let listen_only = buf.get_bool()?;
        let loopback = buf.get_bool()?;
        let three_samples = buf.get_bool()?;
        Ok(CanControl {
            listen_only,
            loopback,
            three_samples,
        })
    }
}

/// Contents of the Add command
#[allow(unused)]
#[derive(Clone, PartialEq, Debug)]
pub struct Add {
    pub duration: Duration,
    pub id: u32,
    pub data: Vec8,
}

impl TryFrom<&mut CommandBuffer> for Add {
    type Error = Error;

    fn try_from(buf: &mut CommandBuffer) -> Result<Self, Self::Error> {
        let duration = buf.get_duration()?;
        let id = buf.get_u32()?;
        let data = buf.get_data()?;
        Ok(Add { duration, id, data })
    }
}

/// Contents of the Update command
#[allow(unused)]
#[derive(Clone, PartialEq, Debug)]
pub struct Update {
    pub id: u32,
    pub data: Vec8,
}

impl TryFrom<&mut CommandBuffer> for Update {
    type Error = Error;

    fn try_from(buf: &mut CommandBuffer) -> Result<Self, Self::Error> {
        let id = buf.get_u32()?;
        let data = buf.get_data()?;
        Ok(Update { id, data })
    }
}

/// Contents of the Delete command
#[allow(unused)]
#[derive(Clone, PartialEq, Debug)]
pub struct Delete {
    id: u32,
}

impl TryFrom<&mut CommandBuffer> for Delete {
    type Error = Error;

    fn try_from(buf: &mut CommandBuffer) -> Result<Self, Self::Error> {
        let id = buf.get_u32()?;
        Ok(Delete { id })
    }
}

/// Contents of the Send command
#[allow(unused)]
#[derive(Clone, PartialEq, Debug)]
pub struct Send {
    pub id: u32,
    pub data: Vec8,
}

impl TryFrom<&mut CommandBuffer> for Send {
    type Error = Error;

    fn try_from(buf: &mut CommandBuffer) -> Result<Self, Self::Error> {
        let id = buf.get_u32()?;
        let data = buf.get_data()?;
        Ok(Send { id, data })
    }
}

/// Contents of the Filter command
#[allow(unused)]
#[derive(Clone, PartialEq, Debug)]
pub struct Filter {
    pub duration: Duration,
    pub id: u32,
    pub data: Vec8,
}

impl TryFrom<&mut CommandBuffer> for Filter {
    type Error = Error;

    fn try_from(buf: &mut CommandBuffer) -> Result<Self, Self::Error> {
        let duration = buf.get_duration()?;
        let id = buf.get_u32()?;
        let data = buf.get_data()?;
        Ok(Filter { duration, id, data })
    }
}

/// Contents of the MuxFilter command
#[allow(unused)]
#[derive(Clone, PartialEq, Debug)]
pub struct MuxFilter {
    pub duration: Duration,
    pub id: u32,
    pub mux_data: Vec40,
}

impl TryFrom<&mut CommandBuffer> for MuxFilter {
    type Error = Error;

    fn try_from(buf: &mut CommandBuffer) -> Result<Self, Self::Error> {
        let duration = buf.get_duration()?;
        let id = buf.get_u32()?;
        let mux_data = buf.get_mux_data()?;
        Ok(MuxFilter {
            duration,
            id,
            mux_data,
        })
    }
}

/// Contents of the Subscribe command
#[allow(unused)]
#[derive(Clone, PartialEq, Debug)]
pub struct Subscribe {
    pub duration: Duration,
    pub id: u32,
}

impl TryFrom<&mut CommandBuffer> for Subscribe {
    type Error = Error;

    fn try_from(buf: &mut CommandBuffer) -> Result<Self, Self::Error> {
        let duration = buf.get_duration()?;
        let id = buf.get_u32()?;
        Ok(Subscribe { duration, id })
    }
}

/// Contents of the Unsubscribe command
#[allow(unused)]
#[derive(Clone, PartialEq, Debug)]
pub struct Unsubscribe {
    pub id: u32,
}

impl TryFrom<&mut CommandBuffer> for Unsubscribe {
    type Error = Error;

    fn try_from(buf: &mut CommandBuffer) -> Result<Self, Self::Error> {
        let id = buf.get_u32()?;
        Ok(Unsubscribe { id })
    }
}

/// Contents of the Statistics command
#[allow(unused)]
#[derive(Clone, PartialEq, Debug)]
pub struct Statistics {
    pub duration: Duration,
}

impl TryFrom<&mut CommandBuffer> for Statistics {
    type Error = Error;

    fn try_from(buf: &mut CommandBuffer) -> Result<Self, Self::Error> {
        let msecs = buf.get_u32()?;
        Ok(Statistics {
            duration: Duration::from_msecs(msecs),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_bcm_cases() {
        let mut cmd_parser = CommandParser::new();

        let mut buf = CommandBuffer::new();
        buf.append(b"< echo >").unwrap();
        let cmd = cmd_parser.parse(&mut buf).unwrap();
        assert!(cmd == Commands::Echo);

        buf.append(b"< can0 C 0 1 0 >").unwrap();
        let cmd = cmd_parser.parse(&mut buf).unwrap();
        let can_control = CanControl {
            listen_only: false,
            loopback: true,
            three_samples: false,
        };
        assert!(cmd == Commands::CanControl(can_control));

        buf.append(b"< can0 B 1 2 3 4 5 6 7 8 >").unwrap();
        let cmd = cmd_parser.parse(&mut buf).unwrap();
        let can_timing = CanTiming {
            bit_rate: 1,
            sample_point: 2,
            tq: 3,
            prop_seg: 4,
            phase_seg1: 5,
            phase_seg2: 6,
            sjw: 7,
            brp: 8,
        };
        assert!(cmd == Commands::CanTiming(can_timing));

        buf.append(b"< open can0 >").unwrap();
        let _cmd = cmd_parser.parse(&mut buf).unwrap();
        assert!(cmd_parser.mode() == Mode::Bcm);
        assert!(buf.len() == 0);

        buf.append(b"< add 1 0 123 8 11 22 33 44 55 66 77 88 >")
            .unwrap();
        let cmd = cmd_parser.parse(&mut buf).unwrap();
        let add = Add {
            duration: Duration::from_usecs(1_000_000),
            id: 123,
            data: Vec8::from_slice(b"\x11\x22\x33\x44\x55\x66\x77\x88").unwrap(),
        };
        assert!(cmd == Commands::Add(add));
        assert!(buf.len() == 0);

        buf.append(b"< update 123 3 11 22 33 >").unwrap();
        let cmd = cmd_parser.parse(&mut buf).unwrap();
        let update = Update {
            id: 123,
            data: Vec8::from_slice(b"\x11\x22\x33").unwrap(),
        };
        assert!(cmd == Commands::Update(update));
        assert!(buf.len() == 0);

        buf.append(b"< delete 123 >").unwrap();
        let cmd = cmd_parser.parse(&mut buf).unwrap();
        let delete = Delete { id: 123 };
        assert!(cmd == Commands::Delete(delete));
        assert!(buf.len() == 0);

        buf.append(b"< send 123 1 ff >").unwrap();
        let cmd = cmd_parser.parse(&mut buf).unwrap();
        let send = Send {
            id: 123,
            data: Vec8::from_slice(b"\xff").unwrap(),
        };
        assert!(cmd == Commands::Send(send));
        assert!(buf.len() == 0);

        buf.append(b"< filter 0 0 123 8 FF 00 F8 00 00 00 00 00 >")
            .unwrap();
        let cmd = cmd_parser.parse(&mut buf).unwrap();
        let filter = Filter {
            duration: Duration::from_usecs(0),
            id: 123,
            data: Vec8::from_slice(b"\xff\x00\xf8\x00\x00\x00\x00\x00").unwrap(),
        };
        assert!(cmd == Commands::Filter(filter));
        assert!(buf.len() == 0);

        buf.append(b"< muxfilter 0 0 123 2 FF 00 00 00 00 00 00 00 33 FF FF FF FF FF FF FF >")
            .unwrap();
        let cmd = cmd_parser.parse(&mut buf).unwrap();
        let mux_filter = MuxFilter {
            duration: Duration::from_usecs(0),
            id: 123,
            mux_data: Vec40::from_slice(
                b"\xff\x00\x00\x00\x00\x00\x00\x00\x33\xff\xff\xff\xff\xff\xff\xff",
            )
            .unwrap(),
        };
        assert!(cmd == Commands::MuxFilter(mux_filter));
        assert!(buf.len() == 0);

        buf.append(b"< subscribe 0 0 123 >").unwrap();
        let cmd = cmd_parser.parse(&mut buf).unwrap();
        let subscribe = Subscribe {
            duration: Duration::from_usecs(0),
            id: 123,
        };
        assert!(cmd == Commands::Subscribe(subscribe));
        assert!(buf.len() == 0);

        buf.append(b"< unsubscribe 123 >").unwrap();
        let cmd = cmd_parser.parse(&mut buf).unwrap();
        let unsubscribe = Unsubscribe { id: 123 };
        assert!(cmd == Commands::Unsubscribe(unsubscribe));
        assert!(buf.len() == 0);
    }

    #[test]
    fn ok_raw_cases() {
        let mut cmd_parser = CommandParser::new();

        let mut buf = CommandBuffer::new();
        buf.append(b"< open can0 >").unwrap();
        let _cmd = cmd_parser.parse(&mut buf).unwrap();
        assert!(cmd_parser.mode() == Mode::Bcm);
        assert!(buf.len() == 0);

        buf.append(b"< rawmode >").unwrap();
        let _cmd = cmd_parser.parse(&mut buf).unwrap();
        assert!(cmd_parser.mode() == Mode::Raw);
        assert!(buf.len() == 0);

        buf.append(b"< send 123 1 ff >").unwrap();
        let cmd = cmd_parser.parse(&mut buf).unwrap();
        let send = Send {
            id: 123,
            data: Vec8::from_slice(b"\xff").unwrap(),
        };
        assert!(cmd == Commands::Send(send));
        assert!(buf.len() == 0);

        let mut buf = CommandBuffer::new();
        buf.append(b"< bcmmode >").unwrap();
        let _cmd = cmd_parser.parse(&mut buf).unwrap();
        assert!(cmd_parser.mode() == Mode::Bcm);
        assert!(buf.len() == 0);
    }

    #[test]
    fn ok_control_cases() {
        let mut cmd_parser = CommandParser::new();

        let mut buf = CommandBuffer::new();
        buf.append(b"< open can0 >").unwrap();
        let _cmd = cmd_parser.parse(&mut buf).unwrap();
        assert!(cmd_parser.mode() == Mode::Bcm);
        assert!(buf.len() == 0);

        buf.append(b"< controlmode >").unwrap();
        let _cmd = cmd_parser.parse(&mut buf).unwrap();
        assert!(cmd_parser.mode() == Mode::Control);
        assert!(buf.len() == 0);

        buf.append(b"< statistics 1000 >").unwrap();
        let cmd = cmd_parser.parse(&mut buf).unwrap();
        let statistics = Statistics {
            duration: Duration::from_msecs(1000),
        };
        assert!(cmd == Commands::Statistics(statistics));
        assert!(buf.len() == 0);

        let mut buf = CommandBuffer::new();
        buf.append(b"< bcmmode >").unwrap();
        let _cmd = cmd_parser.parse(&mut buf).unwrap();
        assert!(cmd_parser.mode() == Mode::Bcm);
        assert!(buf.len() == 0);
    }

    #[test]
    fn ok_iso_tp_cases() {
        let mut cmd_parser = CommandParser::new();

        let mut buf = CommandBuffer::new();
        buf.append(b"< open can0 >").unwrap();
        let _cmd = cmd_parser.parse(&mut buf).unwrap();
        assert!(cmd_parser.mode() == Mode::Bcm);
        assert!(buf.len() == 0);

        buf.append(b"< isotpmode >").unwrap();
        let _cmd = cmd_parser.parse(&mut buf).unwrap();
        assert!(cmd_parser.mode() == Mode::IsoTp);
        assert!(buf.len() == 0);

        buf.append(b"< sendpdu xxx >").unwrap();
        let cmd = cmd_parser.parse(&mut buf);
        println!("{:?}", cmd);
        assert!(cmd == Err(Error::NotSupported));

        let mut buf = CommandBuffer::new();
        buf.append(b"< bcmmode >").unwrap();
        let _cmd = cmd_parser.parse(&mut buf).unwrap();
        assert!(cmd_parser.mode() == Mode::Bcm);
        assert!(buf.len() == 0);
    }
}
