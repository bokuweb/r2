pub mod bus;
pub mod bus_interface;
pub mod clint;
pub mod cpu;

use bus_interface::{BusController, BusReader, BusWriter};
use cpu::{Cpu, CpuState};

pub fn start<B: BusController + BusReader + BusWriter>(
    bus: B,
    pc: u32,
    dtb_ref: u32,
    sleep: &dyn Fn(std::time::Duration),
) {
    'reboot: {
        let mut core = Cpu::new(bus);

        // https://github.com/torvalds/linux/blob/89d77f71f493a3663b10fa812d17f472935d24be/arch/riscv/kernel/head.S#LL153C1-L153C1
        // Pass hart id and ref to dtb.
        core.a0(0x00) // hart id
            .a1(dtb_ref) // ref to dtb
            .pc(pc);

        loop {
            match core.step() {
                CpuState::Active if core.bus().power_off() => return,
                CpuState::Active if core.bus().reboot() => break 'reboot,
                CpuState::Idle => {
                    sleep(core::time::Duration::from_micros(100));
                    core.add_cycles(1);
                }
                _ => {}
            }
        }
    }
}
