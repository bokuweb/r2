//! r2 on the Game Boy Advance.
//!
//! The GBA only has 288KB of RAM, far too little for Linux. Flash carts that
//! run games out of PSRAM expose the GamePak ROM window as writable memory, so
//! the upper 16MB of it (0x09000000-) is used as the RISC-V guest's RAM. The
//! kernel image and DTB are embedded in the ROM and copied there at boot.
#![no_std]
#![no_main]

mod bus;
mod console;
mod font;
mod gba;

use core::arch::global_asm;

use r2_core::cpu::{Cpu, CpuState};

use bus::{GbaBus, GUEST_RAM, RAM_SIZE, RAM_START};

#[repr(C, align(4))]
struct Aligned<T: ?Sized>(T);

#[cfg(not(feature = "baremetal"))]
static IMAGE: &Aligned<[u8]> = &Aligned(*include_bytes!("../../fixtures/linux.bin"));
#[cfg(not(feature = "baremetal"))]
static DTB: Option<&Aligned<[u8]>> = Some(&Aligned(*include_bytes!("../../fixtures/default.dtb")));

#[cfg(feature = "baremetal")]
static IMAGE: &Aligned<[u8]> = &Aligned(*include_bytes!("../../fixtures/baremetal/baremetal.bin"));
#[cfg(feature = "baremetal")]
static DTB: Option<&Aligned<[u8]>> = None;

// Entry point: GBA cartridge header, stacks, IWRAM/BSS setup, then `main`.
global_asm!(
    r#"
    .section .text.crt0, "ax"
    .arm
    .global _start
_start:
    b 1f
    .space 0xbc
1:
    mov r0, #0x12
    msr CPSR_c, r0
    ldr sp, =0x03007fa0
    mov r0, #0x1f
    msr CPSR_c, r0
    ldr sp, =0x03007f00

    ldr r0, =__iwram_lma
    ldr r1, =__iwram_start
    ldr r2, =__iwram_end
2:
    cmp r1, r2
    ldrlo r3, [r0], #4
    strlo r3, [r1], #4
    blo 2b

    ldr r1, =__bss_start
    ldr r2, =__bss_end
    mov r3, #0
3:
    cmp r1, r2
    strlo r3, [r1], #4
    blo 3b

    ldr r0, =main
    bx r0
    .ltorg
    .arm
"#
);

#[no_mangle]
extern "C" fn main() -> ! {
    gba::init();
    console::init();
    println!("r2-gba: RISC-V emulator on GBA");

    if !bus::probe_psram() {
        println!("PSRAM not found: the ROM area is read-only.");
        println!("Use a flash cart with PSRAM (or RGBA_PSRAM=1 on rgba).");
        halt();
    }

    load_guest();
    let dtb_ref = place_dtb();
    println!("image {} bytes, RAM {}MB", IMAGE.0.len(), RAM_SIZE >> 20);
    console::print_help();

    let mut cpu = Cpu::new(GbaBus::new());
    cpu.a0(0).a1(dtb_ref).pc(RAM_START);
    run(&mut cpu);

    println!("\npower off");
    halt();
}

/// Copy the kernel image to the start of guest RAM.
fn load_guest() {
    let src = IMAGE.0.as_ptr() as *const u32;
    let words = IMAGE.0.len().div_ceil(4);
    for i in 0..words {
        unsafe { GUEST_RAM.add(i).write(src.add(i).read()) };
    }
}

/// Put the DTB at the end of guest RAM and tell the kernel that RAM ends
/// where the DTB begins (same trick as mini-rv32ima). Returns its guest address.
fn place_dtb() -> u32 {
    let Some(dtb) = DTB else { return 0 };
    let len = dtb.0.len().next_multiple_of(4);
    let offset = RAM_SIZE as usize - len;
    let src = dtb.0.as_ptr() as *const u32;
    let dst = unsafe { GUEST_RAM.add(offset / 4) };
    for i in 0..len / 4 {
        unsafe { dst.add(i).write(src.add(i).read()) };
    }
    // /memory/reg size cell, big-endian. Only patch the default placeholder.
    let size_cell = unsafe { dst.add(0x13c / 4) };
    if unsafe { size_cell.read() } == 0x00c0_ff03 {
        unsafe { size_cell.write((offset as u32).swap_bytes()) };
    }
    RAM_START + offset as u32
}

/// The emulation loop. Lives in IWRAM (32-bit, zero wait state) together with
/// the CPU core inlined into it.
#[inline(never)]
#[link_section = ".iwram.run"]
fn run(cpu: &mut Cpu<GbaBus>) {
    loop {
        match cpu.step() {
            CpuState::Active => {
                let bus = cpu.bus();
                if bus.power_off || bus.reboot {
                    return;
                }
            }
            // WFI: nothing to do until the next timer interrupt, so jump the
            // clock straight to it.
            CpuState::Idle => cpu.bus_mut().skip_to_timer(),
        }
    }
}

fn halt() -> ! {
    loop {
        gba::vsync();
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("\npanic: {}", info);
    halt();
}
