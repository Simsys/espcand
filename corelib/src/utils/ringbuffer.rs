use crate::{Error, DeSerialize};

pub struct RingBuffer<const CAP: usize> {
    buf: [u8; CAP],
    head: usize,
    tail: usize,
}

impl<const CAP: usize> RingBuffer<CAP> {
    pub fn new() -> Self {
        Self { buf: [0u8; CAP], head: 0, tail: 0 }
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
    }

    pub fn len(&self) -> usize {
        if self.head >= self.tail {
            self.head - self.tail
        } else {
            CAP - self.tail + self.head
        }
    }

    pub fn write(&mut self, slice: &[u8]) -> Result<(), Error> {
        let src_len = slice.len();
        if self.head >= self.tail {
            let dst_len = if self.tail == 0 {
                CAP - self.head - 1
            } else {
                CAP - self.head
            };
            if dst_len >= src_len {
                self.buf[self.head..self.head+src_len].copy_from_slice(slice);
                self.head += src_len;
                if self.head == CAP {
                    self.head = 0;
                }
                Ok(())
            } else {
                self.buf[self.head..self.head+dst_len].copy_from_slice(&slice[..dst_len]);
                if self.tail == 0 {
                    return Err(Error::BufIsFull)
                } else {
                    self.head = 0;
                    self.write(&slice[dst_len..])
                }
            }
        } else {
            if self.tail == 0 {
                Err(Error::BufIsFull)
            } else if (self.tail - self.head) > src_len {
                self.buf[..src_len].copy_from_slice(slice);
                self.head += src_len;
                Ok(())
            } else {
                Err(Error::BufIsFull)
            }
        }
    }

    pub fn read(&mut self, de_ser: &mut impl DeSerialize) -> Result<(), Error> {
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



#[cfg(test)]
mod tests {

    use super::*;
    use crate::DeSer;

    #[test]
    fn fill() {
        let mut r_buf = RingBuffer::<60>::new();
        r_buf.write(b"$RF,125,8,d747b0408ba8c340\n").unwrap();
        assert_eq!(r_buf.len(), 27);
        r_buf.write(b"$RF,125,8,d747b0408ba8c340\n").unwrap();
        assert_eq!(r_buf.len(), 54);
        let r = r_buf.write(b"$RF,125,8,d747b0408ba8c340\n");
        assert_eq!(r, Err(Error::BufIsFull));

        let mut r_buf = RingBuffer::<60>::new();
        let mut de_ser = DeSer::<30>::new();
        r_buf.write(b"xxx$RF,125,8,d747b0408ba8c340\n").unwrap();
        assert_eq!(r_buf.len(), 30);
        r_buf.read(&mut de_ser).unwrap();
        assert_eq!(de_ser.as_slice(), b"$RF,125,8,d747b0408ba8c340\n");
        assert_eq!(r_buf.len(), 0);

        let mut r_buf = RingBuffer::<60>::new();
        let mut de_ser = DeSer::<30>::new();
        r_buf.write(b"$RF,125,8,d747b0408ba8c340\n").unwrap();
        assert_eq!(r_buf.len(), 27);
        r_buf.write(b"$RF,125,8,d747b0408ba8c340\n").unwrap();
        assert_eq!(r_buf.len(), 54);
        r_buf.read(&mut de_ser).unwrap();
        assert_eq!(de_ser.as_slice(), b"$RF,125,8,d747b0408ba8c340\n");
        r_buf.write(b"$RF,125,8,d747b0408ba8c340\n").unwrap();
        assert_eq!(r_buf.len(), 54);
        de_ser.clear();
        r_buf.read(&mut de_ser).unwrap();
        assert_eq!(de_ser.as_slice(), b"$RF,125,8,d747b0408ba8c340\n");
        de_ser.clear();
        r_buf.read(&mut de_ser).unwrap();
        assert_eq!(de_ser.as_slice(), b"$RF,125,8,d747b0408ba8c340\n");
        assert_eq!(r_buf.len(), 0);

        let mut r_buf = RingBuffer::<60>::new();
        r_buf.write(b"1234567890").unwrap();
        r_buf.write(b"1234567890").unwrap();
        r_buf.write(b"1234567890").unwrap();
        r_buf.write(b"1234567890").unwrap();
        r_buf.write(b"1234567890").unwrap();
        r_buf.write(b"123456789").unwrap();
        assert_eq!(r_buf.write(b"0"), Err(Error::BufIsFull));
        assert_eq!(r_buf.len(), 59);

        let mut r_buf = RingBuffer::<60>::new();
        r_buf.write(b"12345").unwrap();
        r_buf.write(b"1234567890").unwrap();
        r_buf.write(b"1234567890").unwrap();
        r_buf.write(b"1234567890").unwrap();
        r_buf.write(b"1234567890").unwrap();
        r_buf.write(b"1234567890").unwrap();
        r_buf.write(b"1234").unwrap();
        assert_eq!(r_buf.write(b"0"), Err(Error::BufIsFull));
        assert_eq!(r_buf.len(), 59);

        let mut r_buf = RingBuffer::<60>::new();
        let mut de_ser = DeSer::<30>::new();
        r_buf.write(b"1234\n").unwrap();
        r_buf.read(&mut de_ser).unwrap();
        r_buf.write(b"1234567890").unwrap();
        r_buf.write(b"1234567890").unwrap();
        r_buf.write(b"1234567890").unwrap();
        r_buf.write(b"1234567890").unwrap();
        r_buf.write(b"1234567890").unwrap();
        r_buf.write(b"123456789").unwrap();
        assert_eq!(r_buf.len(), 59);
        assert_eq!(r_buf.write(b"0"), Err(Error::BufIsFull));
        assert_eq!(r_buf.len(), 59);
    }
}