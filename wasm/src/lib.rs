use core::bus::RAM_START;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn wait(us: usize);
    fn elapsed_us() -> usize;
    fn tx(s: u32);
    fn rx() -> u32;
    fn keydown() -> bool;
}

struct Elapsed;

impl device_interfaces::TimerDriver for Elapsed {
    fn as_micros(&self) -> u64 {
        elapsed_us() as u64
    }
}

struct Term;

impl device_interfaces::SerialInterface for Term {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            0x0000 if keydown() => rx() as u8,
            0x0005 => 0x60 | if keydown() { 1 } else { 0 },
            _ => 0,
        }
    }

    fn write(&self, addr: u32, v: u32) {
        if addr == 0x000 {
            tx(v);
        }
    }
}

#[wasm_bindgen]
pub struct WasmCore(core::cpu::Cpu<core::bus::Bus<Elapsed, Term>>);

#[wasm_bindgen]
#[allow(clippy::new_without_default)]
impl WasmCore {
    pub fn start() {
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
}
