use crate::{Error, DeSerialize};

pub struct RxBuffer<const CAP: usize> {
    buf: [u8; CAP],
    head: usize,
    tail: usize,
}

impl<const CAP: usize> RxBuffer<CAP> {
    pub const fn new() -> Self {
        Self { buf: [0; CAP], head: 0, tail: 0 }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.head = 0;
        self.tail = 0;
        &mut self.buf
    }

    pub fn set_head(&mut self, head: usize) {
        self.tail = 0;
        self.head = head;
    }

    pub fn read(&mut self, de_ser: &mut impl DeSerialize) -> Result<(), Error> {
        if self.head == self.tail {
            return Err(Error::BufIsEmpty);
        }
        let mut tail = self.tail;
        let mut start = false;
        while tail != self.head {
            let b = self.buf[tail];
            if b == b'$' {
                start = true;
            }
            if start {
                match de_ser.push(b) {
                    Ok(()) => (),
                    Err(_) => {
                        self.tail = tail;
                        return Err(Error::BufIsFull)
                    }
                }
            }
            tail += 1;
            if tail == CAP {
                tail = 0;
            }
            if b == b'\n' {
                self.tail = tail;
                return Ok(())
            }
        }
        Err(Error::EndNotFound)
    }
}
