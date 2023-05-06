use std::error::Error;

#[derive(Debug, Clone, Copy)]
pub enum BusException {
    LoadAddressMisaligned,
    LoadAccessFault,
    StoreAddressMisaligned,
    StoreAccessFault,
}

impl std::fmt::Display for BusException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoadAddressMisaligned => write!(f, "LoadAddressMisaligned,"),
            Self::LoadAccessFault => write!(f, "LoadAccessFault,"),
            Self::StoreAddressMisaligned => write!(f, "StoreAddressMisaligned"),
            Self::StoreAccessFault => write!(f, "StoreAccessFault"),
        }
    }
}

impl Error for BusException {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

pub trait BusController {
    fn step(&mut self, mip: &mut u32);
    fn power_off(&self) -> bool;
    fn reboot(&self) -> bool;
}

pub trait BusReader {
    fn read8(&self, addr: u32) -> Result<u8, BusException>;
    fn read16(&self, addr: u32) -> Result<u16, BusException>;
    fn read32(&self, addr: u32) -> Result<u32, BusException>;
}

pub trait BusWriter {
    fn write8(&mut self, addr: u32, v: u8) -> Result<(), BusException>;
    fn write16(&mut self, addr: u32, v: u16) -> Result<(), BusException>;
    fn write32(&mut self, addr: u32, v: u32) -> Result<(), BusException>;
}
