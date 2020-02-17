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

    // TODO find something that isn't stdin/stdout and use them for tests
}
