use crate::io::mbc::mbc_none::MbcNone;
use crate::io::mbc::mbc1::Mbc1;
use crate::io::mbc::mbc3::Mbc3;
use crate::io::mbc::mbc5::Mbc5;
use crate::mmu::{MemRead, MemWrite};
use std::fmt;

// Memory Bank Controller
pub trait Mbc {
    fn get_name(&self) -> &str;

    fn on_read(&self, addr: u16) -> MemRead;

    fn on_write(&mut self, addr: u16, value: u8) -> MemWrite;
}

pub fn new(code: u8, rom: Vec<u8>) -> Box<dyn Mbc> {
    match code {
        0x00 => Box::new(MbcNone::new(rom)),
        0x01 | 0x02 | 0x03 => Box::new(Mbc1::new(rom)),
        0x0f | 0x10 | 0x11 | 0x12 | 0x13 => Box::new(Mbc3::new(rom)),
        0x19 | 0x1a | 0x1b | 0x1c | 0x1d | 0x1e => Box::new(Mbc5::new(rom)),
        0x05 | 0x06 => unimplemented!("MBC2: {:02x}", code),
        0x08 | 0x09 => unimplemented!("ROM+RAM: {:02x}", code),
        0x0b | 0x0c | 0x0d => unimplemented!("MMM01: {:02x}", code),
        0xfc => unimplemented!("POCKET CAMERA"),
        0xfd => unimplemented!("BANDAI TAMA5"),
        0xfe => unimplemented!("HuC3"),
        0xff => unimplemented!("HuC1"),
        _ => unreachable!("Invalid cartridge type: {:02x}", code),
    }
}

impl fmt::Debug for dyn Mbc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_name())
    }
}
