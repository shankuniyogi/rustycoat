use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Memory(Arc<Mutex<MemoryImpl>>);

impl Memory {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(MemoryImpl {
            ram: vec![0; 65536],
            banks: Vec::new(),
            map: [(0, 0); 256],
        })))
    }

    pub fn configure_banks(&self, banks: Vec<Box<dyn MemoryBank + Send>>, configs: &[(u16, u16, usize, u16)]) {
        self.0.lock().unwrap().configure_banks(banks, configs);
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        self.0.lock().unwrap().read_byte(address)
    }

    pub fn write_byte(&self, address: u16, value: u8) {
        self.0.lock().unwrap().write_byte(address, value)
    }

    pub fn read_block(&self, start: u16, data: &mut [u8]) {
        self.0.lock().unwrap().read_block(start, data)
    }

    pub fn write_block(&self, start: u16, data: &[u8]) {
        self.0.lock().unwrap().write_block(start, data)
    }

    #[allow(dead_code)]
    fn read_bank_byte(&self, bank_id: usize, addr: u16, offset: u16) -> u8 {
        let mem = self.0.lock().unwrap();
        mem.banks[bank_id - 1].read_byte(addr, offset, &mem.ram)
    }

    #[allow(dead_code)]
    fn ram(&self, addr: u16) -> u8 {
        self.0.lock().unwrap().ram[addr as usize]
    }
}

pub trait MemoryBank {
    fn size(&self) -> usize;
    fn is_writeable(&self, addr: u16) -> bool;
    fn read_byte(&self, addr: u16, offset: u16, ram: &[u8]) -> u8;
    fn write_byte(&mut self, addr: u16, offset: u16, val: u8, ram: &mut [u8]);
}

struct MemoryImpl {
    ram: Vec<u8>,
    banks: Vec<Box<dyn MemoryBank + Send>>,
    map: [(usize, u16); 256],
}

impl MemoryImpl {
    fn configure_banks(&mut self, banks: Vec<Box<dyn MemoryBank + Send>>, configs: &[(u16, u16, usize, u16)]) {
        self.banks = banks;
        self.map.fill((0, 0));
        for e in configs {
            let (start_addr, length, bank_id, target_offset) = *e;
            assert!(start_addr & 0xFF == 0);
            assert!(length > 0 && length & 0xFF == 0);
            assert!(start_addr >= target_offset);
            let start_page = (start_addr >> 8) as usize;
            let end_page = start_page + (length >> 8) as usize - 1;
            assert!(end_page <= 0xff);
            for page in start_page..=end_page as usize {
                self.map[page] = (bank_id, start_addr - target_offset);
            }
        }
    }

    fn read_byte(&self, address: u16) -> u8 {
        let (bank_id, offset) = self.map[(address >> 8) as usize];
        if bank_id > 0 {
            self.banks[bank_id - 1].read_byte(address, offset, &self.ram)
        } else {
            self.ram[address as usize]
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        let (bank_id, offset) = self.map[(address >> 8) as usize];
        if bank_id > 0 && self.banks[bank_id - 1].is_writeable(address - offset) {
            self.banks[bank_id - 1].write_byte(address, offset, value, &mut self.ram);
        } else {
            self.ram[address as usize] = value;
        }
    }

    fn read_block(&self, start: u16, data: &mut [u8]) {
        for (i, d) in data.iter_mut().enumerate() {
            *d = self.read_byte(start + i as u16);
        }
    }

    fn write_block(&mut self, start: u16, data: &[u8]) {
        for (i, d) in data.iter().enumerate() {
            self.write_byte(start + i as u16, *d);
        }
    }
}


pub struct RomBank {
    bytes: Vec<u8>,
}

impl RomBank {
    pub fn with_bytes(bytes: &[u8]) -> Box<Self> {
        Box::new(Self { bytes: bytes.to_vec() })
    }
}

impl MemoryBank for RomBank {
    fn size(&self) -> usize {
        self.bytes.len()
    }

    fn is_writeable(&self, _addr: u16) -> bool {
        false
    }

    fn read_byte(&self, addr: u16, offset: u16, _ram: &[u8]) -> u8 {
        let addr = (addr - offset) as usize;
        if addr < self.bytes.len() {
            self.bytes[addr]
        } else {
            0
        }
    }

    fn write_byte(&mut self, _addr: u16, _offset: u16, _val: u8, _ram: &mut [u8]) {
        panic!("Attempted to write to ROM bank");
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    struct TestBank {
        mem: Vec<u8>,
        is_writeable: bool,
    }

    impl TestBank {
        fn new_boxed(size: usize, is_writeable: bool) -> Box<Self> {
            Box::new(Self { mem: vec![0; size], is_writeable })
        }
    }

    impl MemoryBank for TestBank {
        fn size(&self) -> usize {
            self.mem.len()
        }

        fn is_writeable(&self, _addr: u16) -> bool {
            self.is_writeable
        }

        fn read_byte(&self, addr: u16, offset: u16, _ram: &[u8]) -> u8 {
            self.mem[addr as usize - offset as usize]
        }

        fn write_byte(&mut self, addr: u16, offset: u16, val: u8, _ram: &mut [u8]) {
            if self.is_writeable {
                self.mem[addr as usize - offset as usize] = val;
            } else {
                panic!("Write to non-writeable memory!");
            }
        }
    }

    #[test]
    fn ram() {
        let mem = Memory::new();
        mem.write_byte(0xBADA, 0xFC);
        assert_eq!(mem.read_byte(0xBADA), 0xFC);
    }

    #[test]
    fn banked_ram() {
        let mem = Memory::new();
        mem.configure_banks(
            vec![TestBank::new_boxed(2048, true)],
            &[(0x3000, 1024, 1, 0x0000), (0x8000, 1024, 1, 0x0400)],
        );

        mem.write_byte(0xBADA, 0xFC);
        assert_eq!(mem.read_byte(0xBADA), 0xFC);

        assert_eq!(mem.read_byte(0x3001), 0x00);
        mem.write_byte(0x3001, 0xCD);
        assert_eq!(mem.read_byte(0x3001), 0xCD);
        assert_eq!(mem.read_bank_byte(1, 0x0001, 0), 0xCD);

        mem.write_byte(0x8001, 0xAB);
        assert_eq!(mem.read_byte(0x8001), 0xAB);
        assert_eq!(mem.read_bank_byte(1, 0x0401, 0), 0xAB);
    }

    #[test]
    fn banked_rom() {
        let mem = Memory::new();
        mem.configure_banks(
            vec![RomBank::with_bytes(&[0xDE, 0xAD, 0xBE, 0xEF])],
            &[(0x3000, 1024, 1, 0x0000)],
        );

        assert_eq!(mem.read_byte(0x3000), 0xDE);
        assert_eq!(mem.read_byte(0x3003), 0xEF);
        mem.write_byte(0x3003, 0xCD);
        assert_eq!(mem.read_byte(0x3003), 0xEF);
        assert_eq!(mem.ram(0x3003), 0xCD);
    }
}
