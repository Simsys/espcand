use crate::{DeSerialize, Error, Serialize};

pub struct RxBuffer<const CAP: usize> {
    buf: [u8; CAP],
    head: usize,
    tail: usize,
}

impl<const CAP: usize> Default for RxBuffer<CAP> {
    fn default() -> Self {
        Self {
            buf: [0; CAP],
            head: 0,
            tail: 0,
        }
    }
}

impl<const CAP: usize> RxBuffer<CAP> {
    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
    }

    pub fn en_mut_block(&mut self) -> &mut [u8] {
        self.head = 0;
        self.tail = 0;
        &mut self.buf
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buf[..self.head]
    }

    pub fn len(&self) -> usize {
        self.head - self.tail
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
                        return Err(Error::BufIsFull);
                    }
                }
            }
            tail += 1;
            if tail == CAP {
                return  Err(Error::BufIsFull);
            }
            if b == b'\n' {
                self.tail = tail;
                return Ok(());
            }
        }
        Err(Error::EndNotFound)
    }

    pub fn write(&mut self, ser: &impl Serialize) -> Result<(), Error> {
        let slice = ser.as_slice();
        if slice.len() > CAP - self.head {
            Err(Error::BufIsFull)
        } else {
            let new_head = self.head + slice.len();
            self.buf[self.head..new_head].copy_from_slice(slice);
            self.head = new_head;
            Ok(())
        }
    }
}
