//! GBA hardware registers used by the host program.

const REG_DISPCNT: *mut u16 = 0x0400_0000 as *mut u16;
const REG_VCOUNT: *const u16 = 0x0400_0006 as *const u16;
const REG_DMA3SAD: *mut u32 = 0x0400_00d4 as *mut u32;
const REG_DMA3DAD: *mut u32 = 0x0400_00d8 as *mut u32;
const REG_DMA3CNT: *mut u32 = 0x0400_00dc as *mut u32;
const REG_KEYINPUT: *const u16 = 0x0400_0130 as *const u16;
const REG_WAITCNT: *mut u16 = 0x0400_0204 as *mut u16;
const VRAM: *mut u16 = 0x0600_0000 as *mut u16;
const MGBA_LOG_ENABLE: *mut u16 = 0x04ff_f780 as *mut u16;
const MGBA_LOG_FLAGS: *mut u16 = 0x04ff_f700 as *mut u16;
const MGBA_LOG_BUFFER: *mut u8 = 0x04ff_f600 as *mut u8;

const WIDTH: usize = 240;

pub const KEY_A: u16 = 1 << 0;
pub const KEY_B: u16 = 1 << 1;
pub const KEY_SELECT: u16 = 1 << 2;
pub const KEY_START: u16 = 1 << 3;
pub const KEY_R: u16 = 1 << 8;
pub const KEY_L: u16 = 1 << 9;

pub fn init() {
    unsafe {
        // GamePak WS0: 3 cycles first access, 1 cycle sequential, prefetch on.
        REG_WAITCNT.write_volatile(0x4317);
        // mGBA / rgba debug log.
        MGBA_LOG_ENABLE.write_volatile(0xc0de);
    }
}

/// Emit one line through the mGBA debug log (no-op on real hardware).
pub fn log(line: &[u8]) {
    let len = line.len().min(255);
    unsafe {
        for (i, &b) in line[..len].iter().enumerate() {
            MGBA_LOG_BUFFER.add(i).write_volatile(b);
        }
        MGBA_LOG_BUFFER.add(len).write_volatile(0);
        // Level 3 = info, bit 8 = send.
        MGBA_LOG_FLAGS.write_volatile(0x103);
    }
}

pub fn vsync() {
    unsafe {
        while REG_VCOUNT.read_volatile() >= 160 {}
        while REG_VCOUNT.read_volatile() < 160 {}
    }
}

/// Buttons currently held (bit set = pressed).
pub fn keys() -> u16 {
    unsafe { !REG_KEYINPUT.read_volatile() & 0x3ff }
}

pub fn key_name(key: u16) -> &'static str {
    match key {
        KEY_A => "A",
        KEY_B => "B",
        KEY_SELECT => "SELECT",
        KEY_START => "START",
        KEY_R => "R",
        KEY_L => "L",
        _ => "?",
    }
}

/// 240x160 15-bit bitmap mode with BG2 enabled.
pub fn mode3() {
    unsafe { REG_DISPCNT.write_volatile(0x0403) };
}

pub fn put_pixels(x: usize, y: usize, px: &[u16]) {
    let dst = unsafe { VRAM.add(y * WIDTH + x) };
    for (i, &p) in px.iter().enumerate() {
        unsafe { dst.add(i).write_volatile(p) };
    }
}

/// DMA3 word copy.
fn dma3(src: *const u32, dst: *mut u32, words: usize) {
    let ctrl = (1 << 31) | (1 << 26);
    unsafe {
        REG_DMA3SAD.write_volatile(src as u32);
        REG_DMA3DAD.write_volatile(dst as u32);
        REG_DMA3CNT.write_volatile(ctrl | words as u32);
    }
}

pub fn fill_lines(from: usize, to: usize, color: u16) {
    // Plain stores: a DMA fill would read its source word behind the
    // compiler's back, so the store of that word could be optimized away.
    let fill = (color as u32) << 16 | color as u32;
    let dst = unsafe { VRAM.add(from * WIDTH) } as *mut u32;
    for i in 0..(to - from) * WIDTH / 2 {
        unsafe { dst.add(i).write_volatile(fill) };
    }
}

/// Move lines [by, height) up by `by` lines and clear the gap at the bottom.
pub fn scroll_up(by: usize, height: usize, color: u16) {
    let src = unsafe { VRAM.add(by * WIDTH) } as *const u32;
    dma3(src, VRAM as *mut u32, (height - by) * WIDTH / 2);
    fill_lines(height - by, height, color);
}
