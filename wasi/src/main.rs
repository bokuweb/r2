use core::bus::RAM_START;

fn main() {
    let ram_size = 640 * 1024 * 1024;

    let mut ram = vec![0u8; ram_size];

    let bin = include_bytes!("../../fixtures/linux.bin");
    let dtb = include_bytes!("../../fixtures/default.dtb");

    ram[0..bin.len()].copy_from_slice(bin);

    let dtb_ref = ram_size - dtb.len();
    ram[dtb_ref..dtb_ref + dtb.len()].copy_from_slice(dtb);

    let clint = core::clint::Clint::new(devices::timer::Timer::default());
    let uart = devices::uart::Uart::new();
    let bus = core::bus::Bus::new(ram, clint, uart);

    core::start(
        bus,
        RAM_START,
        dtb_ref as u32 + RAM_START,
        &std::thread::sleep,
    );
}
