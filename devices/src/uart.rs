use std::io::Write;

use crate::keyboard::{is_kb_hit, read_kb_byte};
use crate::terminal::RawMode;

/// Binds the guest serial port to the host terminal.
#[derive(Debug)]
pub struct Uart {
    _raw_mode: RawMode,
}

impl Uart {
    pub fn new() -> Self {
        Self {
            _raw_mode: RawMode::enable(),
        }
    }
}

impl Default for Uart {
    fn default() -> Self {
        Self::new()
    }
}

impl device_interfaces::SerialInterface for Uart {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            0x0005 => 0x60 | if is_kb_hit() { 1 } else { 0 },
            0x0000 if is_kb_hit() => read_kb_byte() as u8,
            _ => 0,
        }
    }

    fn write(&self, addr: u32, v: u32) {
        if addr == 0x000 {
            let mut stdout = std::io::stdout().lock();
            // The guest drives the terminal directly, so bytes must be passed through
            // untouched instead of being re-encoded as UTF-8.
            stdout
                .write_all(&[v as u8])
                .expect("failed to write stdout.");
            stdout.flush().expect("failed to flush stdout.");
        }
    }
}
