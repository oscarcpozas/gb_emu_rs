use super::Mbc;
use crate::mmu::{MemRead, MemWrite};

// ---------------------------------------------------------------------------
// MBC5
// ---------------------------------------------------------------------------
// Supports up to 8MB ROM (512 banks of 16KB) and 128KB RAM (16 banks of 8KB).
// Unlike MBC1, bank 0 is accessible in the 0x4000-0x7FFF window.
//
// Write registers:
//   0x0000-0x1FFF  RAM enable
//   0x2000-0x2FFF  ROM bank lower 8 bits
//   0x3000-0x3FFF  ROM bank bit 8 (upper bit)
//   0x4000-0x5FFF  RAM bank (0-0x0F)

pub(crate) struct Mbc5 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank: u16, // 9-bit bank number (0-511)
    ram_bank: u8,
    ram_enabled: bool,
}

impl Mbc5 {
    pub(crate) fn new(rom: Vec<u8>) -> Self {
        Self {
            rom,
            ram: vec![0; 0x20000], // 128KB max
            rom_bank: 1,
            ram_bank: 0,
            ram_enabled: false,
        }
    }

    fn rom_read(&self, addr: usize) -> u8 {
        self.rom.get(addr).copied().unwrap_or(0xFF)
    }
}

impl Mbc for Mbc5 {
    fn get_name(&self) -> &str {
        "MBC5"
    }

    fn on_read(&self, addr: u16) -> MemRead {
        match addr {
            0x0000..=0x3FFF => MemRead::Replace(self.rom_read(addr as usize)),
            0x4000..=0x7FFF => {
                let offset = self.rom_bank as usize * 0x4000 + (addr as usize - 0x4000);
                MemRead::Replace(self.rom_read(offset))
            }
            0xA000..=0xBFFF if self.ram_enabled => {
                let offset = self.ram_bank as usize * 0x2000 + (addr as usize - 0xA000);
                MemRead::Replace(self.ram.get(offset).copied().unwrap_or(0xFF))
            }
            0xA000..=0xBFFF => MemRead::Replace(0xFF),
            _ => MemRead::PassThrough,
        }
    }

    fn on_write(&mut self, addr: u16, value: u8) -> MemWrite {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enabled = value & 0x0F == 0x0A;
            }
            0x2000..=0x2FFF => {
                self.rom_bank = (self.rom_bank & 0x100) | value as u16;
            }
            0x3000..=0x3FFF => {
                self.rom_bank = (self.rom_bank & 0x0FF) | ((value as u16 & 0x01) << 8);
            }
            0x4000..=0x5FFF => {
                self.ram_bank = value & 0x0F;
            }
            0xA000..=0xBFFF if self.ram_enabled => {
                let offset = self.ram_bank as usize * 0x2000 + (addr as usize - 0xA000);
                if offset < self.ram.len() {
                    self.ram[offset] = value;
                }
            }
            _ => {}
        }
        MemWrite::Block
    }
}
