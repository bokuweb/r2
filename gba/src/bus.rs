//! Guest physical bus: RAM in cartridge PSRAM, CLINT timer, syscon and an
//! 8250 UART wired to the on-screen console.

use r2_core::bus_interface::{BusController, BusException, BusReader, BusWriter};

use crate::console;

pub const RAM_START: u32 = 0x8000_0000;
/// Guest RAM lives in the upper half of the 32MB GamePak window.
pub const GUEST_RAM: *mut u32 = 0x0900_0000 as *mut u32;
pub const RAM_SIZE: u32 = 16 * 1024 * 1024;

/// Guest instructions per microsecond of guest time. The clock is driven by
/// the instruction count, so the guest sees a 16 MIPS machine no matter how
/// slow the host really is. (At 1 MIPS, timer work starves userspace and the
/// boot never reaches the login prompt.)
const STEPS_PER_MICRO: u32 = 16;

/// Check that stores to the GamePak window stick (PSRAM present).
pub fn probe_psram() -> bool {
    let p = unsafe { GUEST_RAM.add(RAM_SIZE as usize / 4 - 1) };
    unsafe {
        p.write_volatile(0x5a5a_a5a5);
        let ok = p.read_volatile() == 0x5a5a_a5a5;
        p.write_volatile(0xa5a5_5a5a);
        ok && p.read_volatile() == 0xa5a5_5a5a
    }
}

pub struct GbaBus {
    /// Instructions left until `mtime` ticks.
    sub: u32,
    /// Re-evaluate the timer interrupt on the next step (mtimecmp changed).
    timer_dirty: bool,
    mtime: u64,
    mtimecmp: u64,
    msip: u32,
    /// 8250 line control register (bit 7 = DLAB selects the divisor latches).
    lcr: u8,
    divisor: u16,
    pub power_off: bool,
    pub reboot: bool,
}

impl GbaBus {
    pub fn new() -> Self {
        Self {
            sub: STEPS_PER_MICRO,
            timer_dirty: false,
            mtime: 0,
            mtimecmp: 0,
            msip: 0,
            lcr: 0,
            divisor: 0,
            power_off: false,
            reboot: false,
        }
    }

    /// Fast-forward guest time to the pending timer interrupt (used on WFI).
    pub fn skip_to_timer(&mut self) {
        self.mtime = self.mtime.max(self.mtimecmp);
        self.sub = 1;
    }

    #[inline(always)]
    fn ram(addr: u32, size: u32) -> Option<*mut u8> {
        let offset = addr.wrapping_sub(RAM_START);
        (offset <= RAM_SIZE - size).then(|| unsafe { (GUEST_RAM as *mut u8).add(offset as usize) })
    }

    fn clint_read(&self, addr: u32) -> u32 {
        match addr & 0xffff {
            0x0000 => self.msip,
            0x4000 => self.mtimecmp as u32,
            0x4004 => (self.mtimecmp >> 32) as u32,
            0xbff8 => self.mtime as u32,
            0xbffc => (self.mtime >> 32) as u32,
            _ => 0,
        }
    }

    fn clint_write(&mut self, addr: u32, v: u32) {
        match addr & 0xffff {
            0x0000 => self.msip = v & 1,
            0x4000 => {
                self.mtimecmp = (self.mtimecmp & !0xffff_ffff) | v as u64;
                self.timer_dirty = true;
            }
            0x4004 => {
                self.mtimecmp = (self.mtimecmp & 0xffff_ffff) | ((v as u64) << 32);
                self.timer_dirty = true;
            }
            0xbff8 => self.mtime = (self.mtime & !0xffff_ffff) | v as u64,
            0xbffc => self.mtime = (self.mtime & 0xffff_ffff) | ((v as u64) << 32),
            _ => {}
        }
    }

    fn uart_read(&self, reg: u32) -> u8 {
        let dlab = self.lcr & 0x80 != 0;
        match reg {
            0 if dlab => self.divisor as u8,
            1 if dlab => (self.divisor >> 8) as u8,
            0 => console::read_byte().unwrap_or(0),
            // LSR: transmitter empty, plus data-ready when a key is queued.
            5 => 0x60 | console::has_input() as u8,
            _ => 0,
        }
    }

    fn uart_write(&mut self, reg: u32, v: u8) {
        let dlab = self.lcr & 0x80 != 0;
        match reg {
            0 if dlab => self.divisor = (self.divisor & 0xff00) | v as u16,
            1 if dlab => self.divisor = (self.divisor & 0x00ff) | ((v as u16) << 8),
            0 => console::write_byte(v),
            3 => self.lcr = v,
            _ => {}
        }
    }

    fn mmio_read(&self, addr: u32) -> Result<u32, BusException> {
        match addr {
            0x1100_bff8 | 0x1100_bffc => Ok(self.clint_read(addr)),
            0x1000_0000..=0x1000_00ff => Ok(self.uart_read(addr & 0x7) as u32),
            0x1000_0100..=0x1200_0000 => Ok(0),
            _ => Err(BusException::LoadAccessFault),
        }
    }

    fn mmio_write(&mut self, addr: u32, v: u32) -> Result<(), BusException> {
        match addr {
            // syscon
            0x1110_0000 if v == 0x5555 => self.power_off = true,
            0x1110_0000 if v == 0x7777 => self.reboot = true,
            0x1110_0000 | 0x1100_4000 | 0x1100_4004 => self.clint_write(addr, v),
            0x1000_0000..=0x1000_00ff => self.uart_write(addr & 0x7, v as u8),
            0x1000_0100..=0x1200_0000 => {}
            _ => return Err(BusException::StoreAccessFault),
        }
        Ok(())
    }
}

impl BusController for GbaBus {
    #[inline(always)]
    fn step(&mut self, mip: &mut u32) {
        self.sub -= 1;
        if self.sub != 0 && !self.timer_dirty {
            return;
        }
        if self.sub == 0 {
            self.sub = STEPS_PER_MICRO;
            self.mtime += 1;
        }
        self.timer_dirty = false;
        if self.mtimecmp != 0 && self.mtime >= self.mtimecmp {
            *mip |= 0x80;
        } else {
            *mip &= !0x80;
        }
    }

    fn power_off(&self) -> bool {
        self.power_off
    }

    fn reboot(&self) -> bool {
        self.reboot
    }
}

// The GamePak bus is 16 bits wide and PSRAM carts generally have no byte
// lanes, so all accesses are done as halfwords (or words).
impl BusReader for GbaBus {
    #[inline(always)]
    fn read8(&self, addr: u32) -> Result<u8, BusException> {
        match Self::ram(addr, 1) {
            Some(p) => {
                let hw = unsafe { (((p as usize) & !1) as *const u16).read() };
                Ok((hw >> ((addr & 1) * 8)) as u8)
            }
            None => self.mmio_read(addr).map(|v| v as u8),
        }
    }

    #[inline(always)]
    fn read16(&self, addr: u32) -> Result<u16, BusException> {
        if addr & 1 != 0 {
            return Err(BusException::LoadAddressMisaligned);
        }
        match Self::ram(addr, 2) {
            Some(p) => Ok(unsafe { (p as *const u16).read() }),
            None => self.mmio_read(addr).map(|v| v as u16),
        }
    }

    #[inline(always)]
    fn read32(&self, addr: u32) -> Result<u32, BusException> {
        if addr & 3 != 0 {
            return Err(BusException::LoadAddressMisaligned);
        }
        match Self::ram(addr, 4) {
            Some(p) => Ok(unsafe { (p as *const u32).read() }),
            None => self.mmio_read(addr),
        }
    }
}

impl BusWriter for GbaBus {
    #[inline(always)]
    fn write8(&mut self, addr: u32, v: u8) -> Result<(), BusException> {
        match Self::ram(addr, 1) {
            Some(p) => {
                let p = ((p as usize) & !1) as *mut u16;
                let shift = (addr & 1) * 8;
                unsafe { p.write((p.read() & !(0xff << shift)) | ((v as u16) << shift)) };
                Ok(())
            }
            None => self.mmio_write(addr, v as u32),
        }
    }

    #[inline(always)]
    fn write16(&mut self, addr: u32, v: u16) -> Result<(), BusException> {
        if addr & 1 != 0 {
            return Err(BusException::StoreAddressMisaligned);
        }
        match Self::ram(addr, 2) {
            Some(p) => {
                unsafe { (p as *mut u16).write(v) };
                Ok(())
            }
            None => self.mmio_write(addr, v as u32),
        }
    }

    #[inline(always)]
    fn write32(&mut self, addr: u32, v: u32) -> Result<(), BusException> {
        if addr & 3 != 0 {
            return Err(BusException::StoreAddressMisaligned);
        }
        match Self::ram(addr, 4) {
            Some(p) => {
                unsafe { (p as *mut u32).write(v) };
                Ok(())
            }
            None => self.mmio_write(addr, v),
        }
    }
}
