pub trait SerialInterface {
    fn read(&self, addr: u32) -> u8;

    fn write(&self, addr: u32, v: u32);
}
