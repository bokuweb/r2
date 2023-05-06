use libc::{poll, pollfd, read, POLLIN};
use std::os::fd::AsRawFd;

pub fn read_kb_byte() -> u32 {
    let stdin_fd = std::io::stdin().as_raw_fd();
    let mut buffer: [u8; 1] = [0];
    let result = unsafe { read(stdin_fd, buffer.as_mut_ptr() as *mut libc::c_void, 1) };

    if result > 0 {
        buffer[0] as u32
    } else {
        0
    }
}

pub fn is_kb_hit() -> bool {
    let stdin_fd = std::io::stdin().as_raw_fd();
    let mut fds = pollfd {
        fd: stdin_fd,
        events: POLLIN,
        revents: 0,
    };

    let timeout = 0; // No timeout, return immediately
    let result = unsafe { poll(&mut fds, 1, timeout) };

    result > 0 && fds.revents & POLLIN != 0
}
