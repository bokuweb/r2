use core::bus::RAM_START;

extern "C" {
    fn elapsed_us() -> usize;
    fn tx(s: u32);
    fn rx() -> u32;
    fn keydown() -> bool;
}

struct Elapsed;

impl device_interfaces::TimerDriver for Elapsed {
    fn as_micros(&self) -> u64 {
        unsafe { elapsed_us() as u64 }
    }
}

struct Term;

impl device_interfaces::SerialInterface for Term {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            0x0000 if unsafe { keydown() } => unsafe { rx() as u8 },
            0x0005 => 0x60 | if unsafe { keydown() } { 1 } else { 0 },
            _ => 0,
        }
    }

    fn write(&self, addr: u32, v: u32) {
        if addr == 0x000 {
            unsafe {
                tx(v);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn start() {
    let ram_size = 640 * 1024 * 1024;

    let mut ram = vec![0u8; ram_size];

    let bin = include_bytes!("../../fixtures/linux.bin");
    let dtb = include_bytes!("../../fixtures/default.dtb");

    ram[0..bin.len()].copy_from_slice(bin);

    let dtb_ref = ram_size - dtb.len();
    ram[dtb_ref..dtb_ref + dtb.len()].copy_from_slice(dtb);

    let clint = core::clint::Clint::new(Elapsed);
    let term = Term;
    let bus = core::bus::Bus::new(ram, clint, term);

    let sleep = |_u: std::time::Duration| {};

    core::start(bus, RAM_START, dtb_ref as u32 + RAM_START, &sleep);
}
