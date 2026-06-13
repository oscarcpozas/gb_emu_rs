use super::Mbc;
use crate::mmu::{MemRead, MemWrite};

// ---------------------------------------------------------------------------
// MBC1
// ---------------------------------------------------------------------------
// Supports up to 2MB ROM (128 banks of 16KB) and 32KB RAM (4 banks of 8KB).
//
// Memory map:
//   0x0000-0x3FFF ROM bank 0 (fixed)
//   0x4000-0x7FFF Switchable ROM bank
//   0xA000-0xBFFF Switchable RAM bank (if enabled)
//
// Write registers:
//   0x0000-0x1FFF RAM enable (0x0A = enable, anything else = disable)
//   0x2000-0x3FFF ROM bank low 5 bits (0 treated as 1)
//   0x4000-0x5FFF Upper 2 bits — RAM bank OR upper ROM bank bits
//   0x6000-0x7FFF Banking mode (0 = ROM mode, 1 = RAM mode)

pub(crate) struct Mbc1 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank_lo: u8, // lower 5 bits of ROM bank
    bank_hi: u8,     // upper 2 bits (RAM bank or upper ROM bank)
    ram_enabled: bool,
    ram_mode: bool, // false = ROM banking mode, true = RAM banking mode
}

impl Mbc1 {
    pub(crate) fn new(rom: Vec<u8>) -> Self {
        Self {
            rom,
            ram: vec![0; 0x8000], // 32KB max
            rom_bank_lo: 1,
            bank_hi: 0,
            ram_enabled: false,
            ram_mode: false,
        }
    }

    /// Effective ROM bank for the 0x4000-0x7FFF window.
    fn rom_bank(&self) -> usize {
        let bank = if self.ram_mode {
            self.rom_bank_lo as usize
        } else {
            ((self.bank_hi as usize) << 5) | (self.rom_bank_lo as usize)
        };
        // Banks 0x00, 0x20, 0x40, 0x60 are remapped to the next bank
        match bank {
            0x00 | 0x20 | 0x40 | 0x60 => bank + 1,
            b => b,
        }
    }

    /// Effective RAM bank.
    fn ram_bank(&self) -> usize {
        if self.ram_mode {
            self.bank_hi as usize
        } else {
            0
        }
    }

    fn rom_read(&self, addr: usize) -> u8 {
        self.rom.get(addr).copied().unwrap_or(0xFF)
    }

    fn ram_read(&self, addr: usize) -> u8 {
        self.ram.get(addr).copied().unwrap_or(0xFF)
    }
}

impl Mbc for Mbc1 {
    fn get_name(&self) -> &str {
        "MBC1"
    }

    fn on_read(&self, addr: u16) -> MemRead {
        match addr {
            0x0000..=0x3FFF => MemRead::Replace(self.rom_read(addr as usize)),
            0x4000..=0x7FFF => {
                let offset = self.rom_bank() * 0x4000 + (addr as usize - 0x4000);
                MemRead::Replace(self.rom_read(offset))
            }
            0xA000..=0xBFFF if self.ram_enabled => {
                let offset = self.ram_bank() * 0x2000 + (addr as usize - 0xA000);
                MemRead::Replace(self.ram_read(offset))
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
                let lo = value & 0x1F;
                self.rom_bank_lo = if lo == 0 { 1 } else { lo };
            }
            0x4000..=0x5FFF => {
                self.bank_hi = value & 0x03;
            }
            0x6000..=0x7FFF => {
                self.ram_mode = value & 0x01 != 0;
            }
            0xA000..=0xBFFF if self.ram_enabled => {
                let offset = self.ram_bank() * 0x2000 + (addr as usize - 0xA000);
                if offset < self.ram.len() {
                    self.ram[offset] = value;
                }
            }
            _ => {}
        }
        MemWrite::Block
    }
}
