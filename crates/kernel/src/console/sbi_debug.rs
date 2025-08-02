use sbi::{SbiError, debug_console};

use super::Console;

pub(super) struct SbiDebugConsole {}

impl Console for SbiDebugConsole {
    type Error = SbiError;

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        debug_console::write(bytes)
    }
}
