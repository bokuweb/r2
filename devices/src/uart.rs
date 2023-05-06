use std::io::Write;

use crate::keyboard::{is_kb_hit, read_kb_byte};

#[derive(Debug, Default)]
pub struct Uart;

impl Uart {
    pub fn new() -> Self {
        Self::default()
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
            let c = char::from_u32(v).expect("failed to Convert char from u32.");
            print!("{}", c);
            std::io::stdout().flush().expect("failed to flush stdout.");
        }
    }
}
