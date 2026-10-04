//! Guest serial console: a 60x26 text terminal on the Mode 3 bitmap (4x6
//! font), mirrored line by line to the mGBA debug log. Input comes from the
//! GBA buttons, each of which types a canned shell command.

use core::fmt;

use crate::font::GLYPHS;
use crate::gba;

const COLS: usize = 240 / 4;
const ROWS: usize = 160 / 6;
const FG: u16 = 0x6f7b; // light grey
const BG: u16 = 0x0000;

/// What each button types into the guest.
const KEYMAP: [(u16, &[u8]); 6] = [
    (gba::KEY_A, b"uname -a\n"),
    (gba::KEY_B, b"ls /\n"),
    (gba::KEY_L, b"cat /proc/cpuinfo\n"),
    (gba::KEY_R, b"free\n"),
    (gba::KEY_START, b"uptime\n"),
    (gba::KEY_SELECT, b"ps\n"),
];

/// Typed once the login prompt shows up.
const AUTOLOGIN: &[u8] = b"root\n";
/// Typed at the first shell prompt (login flushes anything typed ahead).
const AUTORUN: &[u8] = b"uname -a\n";

struct Console {
    col: usize,
    row: usize,
    escape: Escape,
    /// Current output line, flushed to the debug log on newline.
    line: [u8; 255],
    line_len: usize,
    /// Last few bytes written, to spot the login prompt.
    tail: [u8; 7],
    input: [u8; 64],
    input_head: usize,
    input_len: usize,
    keys: u16,
    autorun_done: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Escape {
    None,
    Esc,
    Csi,
}

static mut CONSOLE: Console = Console {
    col: 0,
    row: 0,
    escape: Escape::None,
    line: [0; 255],
    line_len: 0,
    tail: [0; 7],
    input: [0; 64],
    input_head: 0,
    input_len: 0,
    keys: 0,
    autorun_done: false,
};

fn console() -> &'static mut Console {
    unsafe { &mut *core::ptr::addr_of_mut!(CONSOLE) }
}

pub fn init() {
    gba::mode3();
    gba::fill_lines(0, 160, BG);
}

/// Print which button types which command.
pub fn print_help() {
    for (key, cmd) in KEYMAP {
        for b in gba::key_name(key).bytes().chain(*b": ") {
            write_byte(b);
        }
        cmd.iter().copied().for_each(write_byte);
    }
}

pub fn write_byte(b: u8) {
    let c = console();
    c.tail.copy_within(1.., 0);
    c.tail[c.tail.len() - 1] = b;
    if &c.tail == b"login: " {
        c.push_input(AUTOLOGIN);
    }
    if !c.autorun_done && c.tail.ends_with(b"~ # ") {
        c.autorun_done = true;
        c.push_input(AUTORUN);
    }

    match (c.escape, b) {
        // Swallow ANSI escape sequences.
        (Escape::None, 0x1b) => c.escape = Escape::Esc,
        (Escape::Esc, b'[') => c.escape = Escape::Csi,
        (Escape::Esc, _) => c.escape = Escape::None,
        (Escape::Csi, 0x40..=0x7e) => c.escape = Escape::None,
        (Escape::Csi, _) => {}
        (Escape::None, b'\n') => {
            c.flush_log();
            c.newline();
        }
        (Escape::None, b'\r') => c.col = 0,
        (Escape::None, 0x08) => c.col = c.col.saturating_sub(1),
        (Escape::None, b'\t') => {
            for _ in 0..(8 - c.col % 8) {
                c.put(b' ');
            }
        }
        (Escape::None, 0x20..=0x7e) => c.put(b),
        _ => {}
    }
}

impl Console {
    fn put(&mut self, b: u8) {
        if self.col == COLS {
            self.newline();
        }
        draw_glyph(self.col, self.row, b);
        self.col += 1;
        if self.line_len < self.line.len() {
            self.line[self.line_len] = b;
            self.line_len += 1;
        }
    }

    fn newline(&mut self) {
        self.col = 0;
        if self.row + 1 < ROWS {
            self.row += 1;
        } else {
            gba::scroll_up(6, ROWS * 6, BG);
        }
    }

    fn flush_log(&mut self) {
        gba::log(&self.line[..self.line_len]);
        self.line_len = 0;
    }

    fn push_input(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if self.input_len == self.input.len() {
                return;
            }
            self.input[(self.input_head + self.input_len) % self.input.len()] = b;
            self.input_len += 1;
        }
    }

    /// Turn newly pressed buttons into typed commands.
    fn poll_keys(&mut self) {
        let keys = gba::keys();
        let pressed = keys & !self.keys;
        self.keys = keys;
        if pressed != 0 && self.input_len == 0 {
            if let Some((_, cmd)) = KEYMAP.iter().find(|(key, _)| pressed & key != 0) {
                self.push_input(cmd);
            }
        }
    }
}

fn draw_glyph(col: usize, row: usize, b: u8) {
    let glyph = &GLYPHS[(b - 0x20) as usize];
    for (y, bits) in glyph.iter().enumerate() {
        let mut px = [BG; 4];
        for (x, p) in px.iter_mut().enumerate() {
            if bits & (8 >> x) != 0 {
                *p = FG;
            }
        }
        gba::put_pixels(col * 4, row * 6 + y, &px);
    }
}

/// Called whenever the guest polls the UART line status.
pub fn has_input() -> bool {
    let c = console();
    c.poll_keys();
    c.input_len != 0
}

pub fn read_byte() -> Option<u8> {
    let c = console();
    if c.input_len == 0 {
        return None;
    }
    let b = c.input[c.input_head];
    c.input_head = (c.input_head + 1) % c.input.len();
    c.input_len -= 1;
    Some(b)
}

pub struct Writer;

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        s.bytes().for_each(write_byte);
        Ok(())
    }
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = writeln!($crate::console::Writer, $($arg)*);
    }};
}
