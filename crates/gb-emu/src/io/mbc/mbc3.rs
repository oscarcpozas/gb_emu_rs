use super::Mbc;
use crate::mmu::{MemRead, MemWrite};

// ---------------------------------------------------------------------------
// MBC3
// ---------------------------------------------------------------------------
// Supports up to 2MB ROM (128 banks) and 32KB RAM (4 banks).
// Has an optional real-time clock (stubbed — reads return 0).
//
// Write registers:
//   0x0000-0x1FFF  RAM/RTC enable
//   0x2000-0x3FFF  ROM bank (7 bits, 0 treated as 1)
//   0x4000-0x5FFF  RAM bank (0-3) or RTC register select (0x08-0x0C)
//   0x6000-0x7FFF  Latch clock data (ignored)

pub(crate) struct Mbc3 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank: u8,
    ram_bank: u8,
    ram_enabled: bool,
}

impl Mbc3 {
    pub(crate) fn new(rom: Vec<u8>) -> Self {
        Self {
            rom,
            ram: vec![0; 0x8000],
            rom_bank: 1,
            ram_bank: 0,
            ram_enabled: false,
        }
    }

    fn rom_read(&self, addr: usize) -> u8 {
        self.rom.get(addr).copied().unwrap_or(0xFF)
    }
}

impl Mbc for Mbc3 {
    fn get_name(&self) -> &str {
        "MBC3"
    }

    fn on_read(&self, addr: u16) -> MemRead {
        match addr {
            0x0000..=0x3FFF => MemRead::Replace(self.rom_read(addr as usize)),
            0x4000..=0x7FFF => {
                let offset = self.rom_bank as usize * 0x4000 + (addr as usize - 0x4000);
                MemRead::Replace(self.rom_read(offset))
            }
            0xA000..=0xBFFF if self.ram_enabled && self.ram_bank <= 0x03 => {
                let offset = self.ram_bank as usize * 0x2000 + (addr as usize - 0xA000);
                MemRead::Replace(self.ram.get(offset).copied().unwrap_or(0xFF))
            }
            0xA000..=0xBFFF if self.ram_enabled => {
                // RTC registers — stub, return 0
                MemRead::Replace(0x00)
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
            0x2000..=0x3FFF => {
                let bank = value & 0x7F;
                self.rom_bank = if bank == 0 { 1 } else { bank };
            }
            0x4000..=0x5FFF => {
                self.ram_bank = value;
            }
            0x6000..=0x7FFF => {
                // RTC latch — ignored
            }
            0xA000..=0xBFFF if self.ram_enabled && self.ram_bank <= 0x03 => {
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
