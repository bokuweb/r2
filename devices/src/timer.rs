#[derive(Debug)]
pub struct Timer {
    last: std::cell::RefCell<std::time::Instant>,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            last: std::cell::RefCell::new(std::time::Instant::now()),
        }
    }
}

impl device_interfaces::TimerDriver for Timer {
    fn as_micros(&self) -> u64 {
        let now = std::time::Instant::now();
        let duration = {
            let last = self.last.borrow();
            now.duration_since(*last)
        };
        *self.last.borrow_mut() = now;
        duration.as_micros() as u64
    }
}
