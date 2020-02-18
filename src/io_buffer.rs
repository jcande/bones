use std::io;

const INITIAL_OFFSET: u8 = 0;
const LAST_OFFSET: u8 = 8;

#[derive(Debug)]
struct BitBuffer {
    buffer: [u8; 1],
    offset: u8,
}

impl BitBuffer {
    fn new() -> Self {
        BitBuffer {
            buffer: [0],
            offset: INITIAL_OFFSET,
        }
    }
}

#[derive(Debug)]
pub struct IoBuffer<I: std::io::Read, O: std::io::Write> {
    input_buf: BitBuffer,
    output_buf: BitBuffer,

    input: I,
    output: O,
}

impl IoBuffer<std::io::Stdin, std::io::Stdout> {
    pub fn new() -> Self {
        let input = io::stdin();
        let output = io::stdout();

        IoBuffer::with_io(input, output)
    }
}

impl<I: std::io::Read, O: std::io::Write> IoBuffer<I, O> {
    pub fn with_io(input: I, output: O) -> Self {
        IoBuffer {
            input_buf: BitBuffer::new(),
            output_buf: BitBuffer::new(),

            input: input,
            output: output,
        }
    }

    pub fn get(&mut self) -> std::io::Result<bool> {
        if self.input_buf.offset == INITIAL_OFFSET {
            self.input.read_exact(&mut self.input_buf.buffer)?;
        }

        let byte = self.input_buf.buffer[0];
        let bit = byte & (1 << self.input_buf.offset);
        self.input_buf.offset += 1;

        if self.input_buf.offset == LAST_OFFSET {
            self.input_buf = BitBuffer::new();
        }

        Ok(bit != 0)
    }

    pub fn put(&mut self, bit: bool) -> std::io::Result<()> {
        let byte = self.output_buf.buffer[0];
        let bit = if bit { 1 } else { 0 };
        self.output_buf.buffer[0] = byte | (bit << self.output_buf.offset);
        self.output_buf.offset += 1;

        if self.output_buf.offset == LAST_OFFSET {
            self.output.write_all(&self.output_buf.buffer)?;
            self.output.flush()?;
            self.output_buf = BitBuffer::new();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct FixedBuf {
        read_buf: Vec<u8>,
        read_offset: usize,

        write_buf: Vec<u8>,
    }

    impl FixedBuf {
        fn new(read_buf: Vec<u8>) -> Self {
            FixedBuf {
                read_buf: read_buf,
                read_offset: 0,

                write_buf: Vec::new(),
            }
        }
    }

    impl std::io::Read for FixedBuf {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let initial_offset = self.read_offset;
            for slot in &mut *buf {
                if self.read_offset >= self.read_buf.len() {
                    break;
                }

                *slot = self.read_buf[self.read_offset];
                self.read_offset += 1;
            }
            Ok(self.read_offset - initial_offset)
        }
        fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut<'_>]) -> io::Result<usize> {
            let mut nread = 0;
            for buf in bufs {
                nread += self.read(buf)?;
            }
            Ok(nread)
        }
    }

    impl std::io::Write for FixedBuf {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            for slot in buf.iter() {
                self.write_buf.push(*slot);
            }
            Ok(buf.len())
        }
        fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> io::Result<usize> {
            let mut nwritten = 0;
            for buf in bufs {
                nwritten += self.write(buf)?;
            }
            Ok(nwritten)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn get_byte() -> std::io::Result<()> {
        let input_byte = 0xa5;
        let input = FixedBuf::new(vec![input_byte]);
        let output = FixedBuf::new(vec![]);
        let mut io = IoBuffer::with_io(input, output);

        for bit_offset in 0..8 {
            let got_bit = io.get()?;
            let bit = (input_byte & (1 << bit_offset)) > 0;

            assert_eq!(got_bit, bit);
        }

        Ok(())
    }

    #[test]
    fn put_byte() -> std::io::Result<()> {
        let output_byte = 0x5a;
        let input = FixedBuf::new(vec![]);
        let output = FixedBuf::new(vec![output_byte]);
        let mut io = IoBuffer::with_io(input, output);

        for bit_offset in 0..8 {
            let gave_bit = (output_byte & (1 << bit_offset)) > 0;
            io.put(gave_bit)?;
        }

        assert_eq!(output_byte, io.output.write_buf[0]);

        Ok(())
    }
}
