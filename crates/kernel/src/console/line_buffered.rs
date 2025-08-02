use core::fmt;

use super::Console;

pub(super) struct LineBufferedConsole<C> {
    buffer: [u8; 1024],
    filled: usize,
    console: C,
}

impl<C> LineBufferedConsole<C>
where
    C: Console,
{
    pub(super) const fn new(console: C) -> Self {
        Self {
            buffer: [0; 1024],
            filled: 0,
            console,
        }
    }

    fn flush(&mut self) -> Result<(), C::Error> {
        while self.filled > 0 {
            let nwritten = self.console.write_bytes(&self.buffer[..self.filled])?;
            if nwritten == 0 {
                break;
            }
            self.filled -= nwritten;
        }
        Ok(())
    }

    fn write_str(&mut self, mut s: &str) -> Result<(), C::Error> {
        while let Some(n) = s.find('\n') {
            let (line, rest) = s.split_at(n + 1);
            self.fill_buf(line.as_bytes())?;
            self.flush()?;
            s = rest;
        }
        self.fill_buf(s.as_bytes())?;
        Ok(())
    }

    fn fill_buf(&mut self, mut bytes: &[u8]) -> Result<(), C::Error> {
        while !bytes.is_empty() {
            let copy_len = usize::min(bytes.len(), self.buffer.len() - self.filled);
            self.buffer[self.filled..][..copy_len].copy_from_slice(&bytes[..copy_len]);
            self.filled += copy_len;
            bytes = &bytes[copy_len..];
            if !bytes.is_empty() {
                self.flush()?;
            }
        }
        Ok(())
    }
}

impl<C> fmt::Write for LineBufferedConsole<C>
where
    C: Console,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let Err(_e) = self.write_str(s) {
            return Err(fmt::Error);
        }
        Ok(())
    }
}
