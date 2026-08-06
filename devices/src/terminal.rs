//! Host terminal configuration for the guest serial console.
//!
//! Without this the host line discipline keeps canonical mode, echo and signal generation
//! enabled, so keystrokes are only handed to the guest once a newline is typed and control
//! characters such as `Ctrl-C` terminate the emulator instead of reaching the guest.

#[cfg(unix)]
mod imp {
    use std::os::fd::AsRawFd;
    use std::sync::Mutex;

    static SAVED: Mutex<Option<libc::termios>> = Mutex::new(None);

    /// Puts the controlling terminal into raw mode and restores it when dropped.
    #[derive(Debug)]
    pub struct RawMode;

    impl RawMode {
        pub fn enable() -> Self {
            if let Some(saved) = get_termios() {
                *SAVED.lock().unwrap() = Some(saved);

                let mut raw = saved;
                unsafe { libc::cfmakeraw(&mut raw) };
                set_termios(&raw);

                // A panic would otherwise leave the terminal unusable.
                let previous_hook = std::panic::take_hook();
                std::panic::set_hook(Box::new(move |info| {
                    restore();
                    previous_hook(info);
                }));
            }
            Self
        }
    }

    impl Drop for RawMode {
        fn drop(&mut self) {
            restore();
        }
    }

    /// Restores the settings captured by [`RawMode::enable`], if any.
    pub fn restore() {
        if let Some(saved) = SAVED.lock().unwrap().take() {
            set_termios(&saved);
        }
    }

    fn get_termios() -> Option<libc::termios> {
        let fd = std::io::stdin().as_raw_fd();
        if unsafe { libc::isatty(fd) } != 1 {
            return None;
        }
        let mut termios = std::mem::MaybeUninit::<libc::termios>::uninit();
        if unsafe { libc::tcgetattr(fd, termios.as_mut_ptr()) } != 0 {
            return None;
        }
        Some(unsafe { termios.assume_init() })
    }

    fn set_termios(termios: &libc::termios) {
        let fd = std::io::stdin().as_raw_fd();
        unsafe { libc::tcsetattr(fd, libc::TCSANOW, termios) };
    }
}

#[cfg(not(unix))]
mod imp {
    #[derive(Debug)]
    pub struct RawMode;

    impl RawMode {
        pub fn enable() -> Self {
            Self
        }
    }

    pub fn restore() {}
}

pub use imp::{restore, RawMode};
