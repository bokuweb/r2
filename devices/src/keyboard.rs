use libc::{poll, pollfd, read, POLLIN};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::os::fd::AsRawFd;

use crate::terminal;

/// Prefix that introduces an emulator command instead of being sent to the guest.
/// The terminal runs in raw mode, so this is the only way to leave the emulator
/// from the keyboard.
const COMMAND_PREFIX: u8 = 0x01; // Ctrl-A

/// Terminates the emulator when it follows [`COMMAND_PREFIX`].
const QUIT_COMMAND: u8 = b'x';

thread_local! {
    static INPUT: RefCell<Input> = const {
        RefCell::new(Input {
            pending: VecDeque::new(),
            prefixed: false,
        })
    };
}

struct Input {
    /// Bytes that are ready to be handed to the guest.
    pending: VecDeque<u8>,
    /// Whether the previous byte was [`COMMAND_PREFIX`].
    prefixed: bool,
}

impl Input {
    fn fill(&mut self) {
        let stdin_fd = std::io::stdin().as_raw_fd();
        while is_readable(stdin_fd) {
            let mut buffer = [0u8; 32];
            let read = unsafe {
                read(
                    stdin_fd,
                    buffer.as_mut_ptr() as *mut libc::c_void,
                    buffer.len(),
                )
            };
            if read <= 0 {
                break;
            }
            for byte in &buffer[..read as usize] {
                self.push(*byte);
            }
        }
    }

    fn push(&mut self, byte: u8) {
        if std::mem::take(&mut self.prefixed) {
            match byte {
                QUIT_COMMAND => {
                    terminal::restore();
                    std::process::exit(0);
                }
                // Ctrl-A Ctrl-A sends a literal Ctrl-A.
                COMMAND_PREFIX => self.pending.push_back(COMMAND_PREFIX),
                _ => self.pending.push_back(byte),
            }
        } else if byte == COMMAND_PREFIX {
            self.prefixed = true;
        } else {
            self.pending.push_back(byte);
        }
    }
}

fn is_readable(fd: i32) -> bool {
    let mut fds = pollfd {
        fd,
        events: POLLIN,
        revents: 0,
    };

    let timeout = 0; // No timeout, return immediately
    let result = unsafe { poll(&mut fds, 1, timeout) };

    result > 0 && fds.revents & POLLIN != 0
}

pub fn read_kb_byte() -> u32 {
    INPUT.with_borrow_mut(|input| {
        input.fill();
        input.pending.pop_front().unwrap_or(0) as u32
    })
}

pub fn is_kb_hit() -> bool {
    INPUT.with_borrow_mut(|input| {
        input.fill();
        !input.pending.is_empty()
    })
}
