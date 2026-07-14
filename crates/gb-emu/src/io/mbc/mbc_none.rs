use super::Mbc;
use crate::mmu::{MemRead, MemWrite};

pub(crate) struct MbcNone {
    rom: Vec<u8>,
}

impl MbcNone {
    pub(crate) fn new(rom: Vec<u8>) -> Self {
        Self { rom }
    }
}

impl Mbc for MbcNone {
    fn get_name(&self) -> &str {
        "ROM Only"
    }

    fn on_read(&self, addr: u16) -> MemRead {
        if addr <= 0x7FFF {
            MemRead::Replace(self.rom[addr as usize])
        } else {
            MemRead::PassThrough
        }
    }

    fn on_write(&mut self, _addr: u16, _value: u8) -> MemWrite {
        // Writes to ROM space are silently ignored (no MBC to handle bank switching)
        MemWrite::Block
    }
}
