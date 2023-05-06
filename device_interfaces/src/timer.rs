pub trait TimerDriver {
    fn as_micros(&self) -> u64;
}
