use crate::{
    bus_interface::{BusController, BusException, BusReader, BusWriter},
    clint::Clint,
};

pub const RAM_START: u32 = 0x8000_0000;

pub struct Bus<T, S> {
    pub ram: Vec<u8>,
    pub clint: Clint<T>,
    pub serial: S,
    pub power_off: bool,
    pub reboot: bool,
}

impl<T, S> Bus<T, S> {
    pub fn new(ram: Vec<u8>, clint: Clint<T>, serial: S) -> Self {
        Self {
            ram,
            clint,
            serial,
            power_off: false,
            reboot: false,
        }
    }

    pub fn clint(&self) -> &Clint<T> {
        &self.clint
    }

    pub fn replace_ram(&mut self, ram: Vec<u8>) -> Vec<u8> {
        std::mem::replace(&mut self.ram, ram)
    }
}

impl<T, S> BusController for Bus<T, S>
where
    T: device_interfaces::TimerDriver,
    S: device_interfaces::SerialInterface,
{
    fn step(&mut self, mip: &mut u32) {
        self.clint.step(mip);
    }

    fn power_off(&self) -> bool {
        self.power_off
    }

    fn reboot(&self) -> bool {
        self.reboot
    }
}

impl<T, S> BusReader for Bus<T, S>
where
    T: device_interfaces::TimerDriver,
    S: device_interfaces::SerialInterface,
{
    fn read8(&self, addr: u32) -> Result<u8, BusException> {
        match addr {
            0x1100bffc => Ok(self.clint.read(addr) as u8),
            0x1100bff8 => Ok(self.clint.read(addr) as u8),
            0x10000000..=0x12000000 => {
                let addr = addr & 0xffff;
                Ok(self.serial.read(addr))
            }
            _ => {
                let addr = addr.wrapping_sub(RAM_START);
                Ok(self.ram[addr as usize])
            }
        }
    }

    fn read16(&self, addr: u32) -> Result<u16, BusException> {
        if addr & 1 != 0 {
            return Err(BusException::LoadAddressMisaligned);
        }
        match addr {
            0x1100bffc => Ok(self.clint.read(addr) as u16),
            0x1100bff8 => Ok(self.clint.read(addr) as u16),
            0x10000000..=0x12000000 => {
                let addr = addr & 0xffff;
                Ok(self.serial.read(addr) as u16)
            }
            _ => {
                let addr = addr.wrapping_sub(RAM_START);
                Ok(u16::from_le_bytes([
                    self.ram[addr as usize],
                    self.ram[addr as usize + 1],
                ]))
            }
        }
    }

    fn read32(&self, addr: u32) -> Result<u32, BusException> {
        if addr & 3 != 0 {
            return Err(BusException::LoadAddressMisaligned);
        }
        match addr {
            0x1100bffc => Ok(self.clint.read(addr & 0xffff)),
            0x1100bff8 => Ok(self.clint.read(addr & 0xffff)),
            0x10000000..=0x12000000 => {
                let addr = addr & 0xffff;
                Ok(self.serial.read(addr) as u32)
            }
            _ => {
                let addr = addr.wrapping_sub(RAM_START);
                Ok(u32::from_le_bytes([
                    self.ram[addr as usize],
                    self.ram[addr as usize + 1],
                    self.ram[addr as usize + 2],
                    self.ram[addr as usize + 3],
                ]))
            }
        }
    }
}

impl<T, S> BusWriter for Bus<T, S>
where
    T: device_interfaces::TimerDriver,
    S: device_interfaces::SerialInterface,
{
    fn write8(&mut self, addr: u32, v: u8) -> Result<(), BusException> {
        match addr {
            // msip
            0x11100000 => self.clint.write(addr & 0xffff, v as u32),
            // mtime
            0x11004004 => self.clint.write(addr & 0xffff, v as u32),
            0x11004000 => self.clint.write(addr & 0xffff, v as u32),
            0x10000000..=0x12000000 => {
                let addr = addr & 0xffff;
                self.serial.write(addr, v as u32);
            }
            _ => {
                let addr = addr.wrapping_sub(RAM_START);
                self.ram[addr as usize] = v;
            }
        };
        Ok(())
    }

    fn write16(&mut self, addr: u32, v: u16) -> Result<(), BusException> {
        if addr & 1 != 0 {
            return Err(BusException::StoreAddressMisaligned);
        }
        match addr {
            // syscon
            0x11100000 if v == 0x5555 => self.power_off = true,
            0x11100000 if v == 0x7777 => self.reboot = true,
            // msip
            0x11100000 => self.clint.write(addr & 0xffff, v as u32),
            // mtime
            0x11004004 => self.clint.write(addr & 0xffff, v as u32),
            0x11004000 => self.clint.write(addr & 0xffff, v as u32),
            0x10000000..=0x12000000 => {
                let addr = addr & 0xffff;
                self.serial.write(addr, v as u32);
            }
            _ => {
                let addr = addr.wrapping_sub(RAM_START);
                self.ram[addr as usize..addr as usize + 2].copy_from_slice(&v.to_le_bytes());
            }
        };
        Ok(())
    }

    fn write32(&mut self, addr: u32, v: u32) -> Result<(), BusException> {
        if addr & 3 != 0 {
            return Err(BusException::StoreAddressMisaligned);
        }
        match addr {
            // syscon
            0x11100000 if v == 0x5555 => self.power_off = true,
            0x11100000 if v == 0x7777 => self.reboot = true,
            // msip
            0x11100000 => self.clint.write(addr & 0xffff, v),
            // mtime
            0x11004004 => self.clint.write(addr & 0xffff, v),
            0x11004000 => self.clint.write(addr & 0xffff, v),
            0x10000000..=0x12000000 => {
                let addr = addr & 0xffff;
                self.serial.write(addr, v);
            }
            _ => {
                let addr = addr.wrapping_sub(RAM_START);
                self.ram[addr as usize..addr as usize + 4].copy_from_slice(&v.to_le_bytes());
            }
        };
        Ok(())
    }
}
