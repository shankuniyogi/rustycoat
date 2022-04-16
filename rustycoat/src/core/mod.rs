pub mod clock;
pub mod memory;

pub trait MemoryBank {
    fn size(&self) -> usize;
    fn is_writeable(&self, addr: u16) -> bool;
    fn read_byte(&self, addr: u16, offset: u16, ram: &[u8]) -> u8;
    fn write_byte(&mut self, addr: u16, offset: u16, val: u8, ram: &mut [u8]);
}
