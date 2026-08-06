use crate::{
    bus_interface::{BusController, BusException, BusReader, BusWriter},
    clint::Clint,
};

pub const RAM_START: u32 = 0x8000_0000;

pub struct Bus<T, S> {
    pub ram: Vec<u8>,
    pub clint: Clint<T>,
    pub serial: S,
    /// 8250 line control register. Its top bit (DLAB) turns registers 0 and 1 into the
    /// baud rate divisor latches, so it has to be tracked to keep the guest from
    /// mistaking a divisor for a character.
    lcr: u8,
    divisor: u16,
    pub power_off: bool,
    pub reboot: bool,
}

impl<T, S> Bus<T, S> {
    pub fn new(ram: Vec<u8>, clint: Clint<T>, serial: S) -> Self {
        Self {
            ram,
            clint,
            serial,
            lcr: 0,
            divisor: 0,
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

    /// Translates a physical address into a RAM offset, rejecting anything the guest
    /// should not be able to reach. Stray accesses have to be reported to the guest as a
    /// fault rather than taking the emulator down with it.
    fn ram_offset(&self, addr: u32, size: u32) -> Option<usize> {
        let offset = addr.checked_sub(RAM_START)? as usize;
        (offset + size as usize <= self.ram.len()).then_some(offset)
    }

    fn dlab(&self) -> bool {
        self.lcr & 0x80 != 0
    }
}

const UART_RBR_THR: u32 = 0x0;
const UART_IER: u32 = 0x1;
const UART_LCR: u32 = 0x3;

impl<T, S: device_interfaces::SerialInterface> Bus<T, S> {
    fn serial_read(&self, reg: u32) -> u8 {
        match reg {
            UART_RBR_THR if self.dlab() => self.divisor as u8,
            UART_IER if self.dlab() => (self.divisor >> 8) as u8,
            _ => self.serial.read(reg),
        }
    }

    fn serial_write(&mut self, reg: u32, v: u8) {
        match reg {
            UART_RBR_THR if self.dlab() => self.divisor = (self.divisor & 0xff00) | v as u16,
            UART_IER if self.dlab() => self.divisor = (self.divisor & 0x00ff) | ((v as u16) << 8),
            UART_LCR => {
                self.lcr = v;
                self.serial.write(reg, v as u32);
            }
            _ => self.serial.write(reg, v as u32),
        }
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
            0x1100bffc => Ok(self.clint.read(addr & 0xffff) as u8),
            0x1100bff8 => Ok(self.clint.read(addr & 0xffff) as u8),
            0x10000000..=0x100000ff => Ok(self.serial_read(addr & 0x7)),
            0x10000100..=0x12000000 => Ok(0),
            _ => {
                let offset = self
                    .ram_offset(addr, 1)
                    .ok_or(BusException::LoadAccessFault)?;
                Ok(self.ram[offset])
            }
        }
    }

    fn read16(&self, addr: u32) -> Result<u16, BusException> {
        if addr & 1 != 0 {
            return Err(BusException::LoadAddressMisaligned);
        }
        match addr {
            0x1100bffc => Ok(self.clint.read(addr & 0xffff) as u16),
            0x1100bff8 => Ok(self.clint.read(addr & 0xffff) as u16),
            0x10000000..=0x100000ff => Ok(self.serial_read(addr & 0x7) as u16),
            0x10000100..=0x12000000 => Ok(0),
            _ => {
                let offset = self
                    .ram_offset(addr, 2)
                    .ok_or(BusException::LoadAccessFault)?;
                Ok(u16::from_le_bytes([self.ram[offset], self.ram[offset + 1]]))
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
            0x10000000..=0x100000ff => Ok(self.serial_read(addr & 0x7) as u32),
            0x10000100..=0x12000000 => Ok(0),
            _ => {
                let offset = self
                    .ram_offset(addr, 4)
                    .ok_or(BusException::LoadAccessFault)?;
                Ok(u32::from_le_bytes([
                    self.ram[offset],
                    self.ram[offset + 1],
                    self.ram[offset + 2],
                    self.ram[offset + 3],
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
            0x10000000..=0x100000ff => self.serial_write(addr & 0x7, v),
            0x10000100..=0x12000000 => {}
            _ => {
                let offset = self
                    .ram_offset(addr, 1)
                    .ok_or(BusException::StoreAccessFault)?;
                self.ram[offset] = v;
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
            0x10000000..=0x100000ff => self.serial_write(addr & 0x7, v as u8),
            0x10000100..=0x12000000 => {}
            _ => {
                let offset = self
                    .ram_offset(addr, 2)
                    .ok_or(BusException::StoreAccessFault)?;
                self.ram[offset..offset + 2].copy_from_slice(&v.to_le_bytes());
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
            0x10000000..=0x100000ff => self.serial_write(addr & 0x7, v as u8),
            0x10000100..=0x12000000 => {}
            _ => {
                let offset = self
                    .ram_offset(addr, 4)
                    .ok_or(BusException::StoreAccessFault)?;
                self.ram[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
            }
        };
        Ok(())
    }
}
