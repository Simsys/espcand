mod socket;
mod utils;

use socket::command_buffer::CommandBuffer;
use socket::command_parser::{CommandParser, Commands, Filter, Mode, Send, Vec8};
use utils::*;

fn main() {
    let mut cmd_parser = CommandParser::new();
    let mut buf = CommandBuffer::new();
    buf.append(b"< open can0 >< filter 0 0 123 0 >< send 124 8 11 22 33 44 55 66 7a 8b >")
        .unwrap();

    let _ = cmd_parser.parse(&mut buf);
    assert!(cmd_parser.mode() == Mode::Bcm);

    let cmd = cmd_parser.parse(&mut buf).unwrap();
    assert!(
        cmd == Commands::Filter(Filter {
            duration: Duration::from_secs(0),
            id: 123,
            data: Vec8::from_slice(b"").unwrap()
        })
    );

    let cmd = cmd_parser.parse(&mut buf).unwrap();
    assert!(
        cmd == Commands::Send(Send {
            id: 124,
            data: Vec8::from_slice(b"\x11\x22\x33\x44\x55\x66\x7a\x8b").unwrap()
        })
    );
    assert!(buf.len() == 0);
}
