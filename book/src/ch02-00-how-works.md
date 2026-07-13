# What happens when the emulator runs?

This chapter provides a compact mental model of the emulator loop. It is not a full Game Boy specification, as there 
is already excellent documentation available across various websites, blog posts, and YouTube videos. I will include 
links throughout this documentation, but you can also find all the sources I used to write this [here](ch99-00-references.md).

## So, What's happen?

The emulator’s main loop processes one frame per iteration and [introduces a delay to maintain a stable target frame rate](https://github.com/oscarcpozas/gb_emu_rs/blob/main/emu/src/emu.rs#L54-L62). 
This delay is necessary because the host system running the emulator has a (likely) significantly more powerful CPU than 
the original Game Boy hardware, allowing it to execute the same operations in a fraction of the time required by the original hardware.

## The core. The CPU

[The CPU is an 8-bit Sharp SM83. It's somewhat between the Intel 8080 and the Z80.](https://www.cpcwiki.eu/index.php/GBZ80)

The CPU clock runs at 4.194304 MHz. I'll go into detail about the number of operations per frame in the screen section, 
but for each video frame, it runs CPU steps until it has consumed roughly one frame's worth of cycles. A step means:

1. Execute one CPU instruction, or idle if the CPU is halted.

2. Advance hardware components by the cycles consumed.

3. Collect interrupt requests.

4. Dispatch an interrupt if the CPU can accept one.

So the frame loop is only a pacing mechanism. The real emulation happens at the step level.

## Memory is the bus

Most communication happens through memory addresses. The CPU reads and writes addresses. The MMU decides whether that 
access is plain RAM or whether a hardware component must handle it. [Check out this video to get a better understanding](https://youtu.be/ecTQVa42sJc).

Examples:

- Reads from cartridge ROM go through the cartridge/MBC.
- Reads and writes to `0x8000-0x9FFF` access PPU VRAM.
- Writes to `0xFF46` start an OAM DMA transfer.
- Reads and writes to `0xFF00` access the joypad register.
- Reads and writes to `0xFF04-0xFF07` access timer registers.
- `0xFF0F` and `0xFFFF` are interrupt registers.

This is the core abstraction: [devices are memory handlers](https://github.com/oscarcpozas/gb_emu_rs/blob/main/emu/src/emu.rs#L85-L109).

The CPU does not need to know much about the PPU, timer or joypad. It only performs memory operations. 
The MMU routes those operations to the right component.

## Cartridge, where game lives

The loaded ROM is wrapped in a [`Cartridge`](https://github.com/oscarcpozas/gb_emu_rs/blob/main/emu/src/io/mbc/cartridge.rs).

The cartridge header tells the emulator which Memory Bank Controller is needed. This matters because the CPU can only address `0x0000-0x7FFF` as cartridge ROM space, while many games contain more ROM than that.

The MBC controls which ROM or RAM bank is visible in the switchable address ranges.

Mental model:

- `0x0000-0x3FFF`: usually fixed ROM bank.
- `0x4000-0x7FFF`: switchable ROM bank.
- `0xA000-0xBFFF`: external cartridge RAM, when present and enabled.

When a game writes to certain addresses in ROM space, it is often not trying to modify ROM. It is configuring the MBC.

## Famous Nintendo Boot ROM overlay

<img src="https://raw.githubusercontent.com/oscarcpozas/gb_emu_rs/refs/heads/main/art/gameboy_boot.gif" alt="drawing" width="450"/>

At power-on, the first bytes at `0x0000-0x00FF` ara mapped to the boot ROM, burned inside the CPU.
The boot ROM performs the initial startup sequence and eventually disables itself by writing to `0xFF50`. [Check Pandocs for more info](https://gbdev.io/pandocs/Power_Up_Sequence.html).

After that, the same address range falls through to the cartridge.

Mental model:

- Before `0xFF50`: `0x0000` reads from boot ROM.
- After `0xFF50`: `0x0000` reads from cartridge ROM.

## **OAM DMA Transfer** - Checking if there's a pending DMA transfer in register 0xFF46

The PPU (screen) has an Object Attribute Memory (OAM) located at `0xFE00-0xFE9F` that stores sprite data.
The recommended way to update the OAM is using DMA (Direct Memory Access). [See pandocs](https://gbdev.io/pandocs/OAM.html#writing-data-to-oam)

When you write to register `0xFF46`, you're telling the Game Boy to copy 160 bytes of sprite data into OAM. Here's how it works:

**The register stores a "page" (high byte) of a memory address:**
- Writing `0xC1` to `0xFF46` means "copy from address `0xC100`"
- The address is constructed by shifting the page value: `(page as u16) << 8`
- Example: `0xC1 << 8 = 0xC100`

**Quick mental model:**
- Register value `0xXX` → copies from `0xXX00` to `0xXX9F` (160 bytes)
- Always copies TO: `0xFE00-0xFE9F` (OAM)
- Always copies FROM: `0xXX00-0xXX9F` (where XX is what you wrote to 0xFF46)

**Why only the high byte?**
Since we always copy exactly 160 bytes and the destination is always OAM, we only need to specify where to copy FROM. The low byte is always `0x00` (start of the page), making it simple to just write one byte.

[More info on OAM DMA Transfer](https://gbdev.io/pandocs/OAM_DMA_Transfer.html)

## Fetch and execute next instruction

Each [step executes one CPU instruction unless the CPU is halted](https://github.com/oscarcpozas/gb_emu_rs/blob/main/emu/src/emu.rs#L132-L172).

If the CPU is in `HALT`, it burns a small amount of cycles and waits until an interrupt wakes it up.

Otherwise, the CPU reads the opcode at `PC`:

- Normal opcodes are one byte.
- `0xCB` is a prefix; the following byte selects an extended instruction.

The instruction decoder executes the operation, mutating CPU registers and/or memory. It returns how many cycles the instruction took and how many bytes it occupied.

After execution, `PC` advances by the instruction size unless the instruction changed it explicitly through a jump, call, return or interrupt flow.

## PPU: video timing

The PPU advances through LCD modes using CPU cycles.

For visible scanlines, the rough flow is:

- OAM search.
- Pixel transfer.
- HBlank.

When a scanline finishes, `LY` moves to the next line. When `LY` reaches the VBlank area, the PPU requests a VBlank interrupt.

The emulator renders one scanline when the PPU reaches HBlank. The frame buffer is just the final output. 
The important emulation state is still VRAM, OAM and LCD registers.

## Timer

The timer is another cycle-driven component.

`DIV` is derived from an internal counter. `TIMA` increments according to the frequency selected in `TAC`. 
When `TIMA` overflows, it reloads from `TMA` and requests a timer interrupt.

Many games rely on this for timing, music, gameplay events or delays.

## Joypad

Input is exposed through `0xFF00`.

The game selects which group it wants to read:

- Direction buttons.
- Action buttons.

The result is active-low: `0` means pressed, `1` means released.

When a new key press is detected, the joypad requests an interrupt.

## Interrupts

Interrupts are requested by hardware and later dispatched to the CPU.

Two registers matter:

- `IF` at `0xFF0F`: requested interrupts.
- `IE` at `0xFFFF`: enabled interrupts.

The CPU also has `IME`, the master interrupt enable flag.

At the end of a step, the interrupt controller checks:

```text
pending = IF & IE & 0x1F
```

If an interrupt is pending, it wakes the CPU from `HALT`.

If `IME` is enabled, the CPU services the highest-priority pending interrupt:

- Clear the interrupt request bit.
- Disable `IME`.
- Push the current `PC` to the stack.
- Jump to the interrupt vector.
- Consume extra cycles.

Interrupt vectors:

| Interrupt | Vector   |
|-----------|----------|
| VBlank    | `0x0040` |
| LCD STAT  | `0x0048` |
| Timer     | `0x0050` |
| Serial    | `0x0058` |
| Joypad    | `0x0060` |

## Quick mental model

The emulator works because these parts are connected by cycles and memory:

1. The CPU executes one instruction.
2. The instruction consumes cycles.
3. The PPU, timer and APU advance by those cycles.
4. Input and hardware may request interrupts.
5. The interrupt controller may redirect the CPU to an interrupt vector.
6. Memory reads and writes are routed by the MMU to RAM, cartridge or hardware registers.
