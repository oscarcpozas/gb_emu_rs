mod gui;

use gb_core::Emu;
use gui::audio_output::AudioOutput;
use gui::hardware::Hardware;
use gui::window::GUI;
use log::*;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;
use structopt::StructOpt;

const TARGET_FPS: u64 = 60;

#[derive(StructOpt)]
pub struct Opt {
    #[structopt(name = "ROM", parse(from_os_str))]
    rom: Option<PathBuf>,
}

fn load_rom_buffer<P: AsRef<Path>>(path: P) -> Vec<u8> {
    let mut f = File::open(&path).expect("Couldn't open ROM file");
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).expect("Couldn't read ROM file");
    buf
}

fn main() {
    env_logger::init();

    let args: Opt = Opt::from_args();

    let gui = GUI::new();
    let hardware = Hardware::new(&gui);

    match args.rom {
        Some(path) => {
            debug!("Reading cartridge from {:?}", path);
            let rom = load_rom_buffer(&path);
            debug!("ROM size: {} bytes", rom.len());

            std::thread::spawn(move || {
                debug!("Starting emulator thread");
                run_emulator(rom, hardware);
            });

            gui.run(true);
        }

        None => {
            debug!("No ROM provided — waiting for drag & drop or file picker");

            // Clone the Arc so the watcher thread can observe it while the
            // main thread is busy running the winit event loop.
            let dropped_file: Arc<Mutex<Option<PathBuf>>> = gui.dropped_file.clone();

            std::thread::spawn(move || {
                loop {
                    let path = dropped_file.lock().unwrap().clone();
                    if let Some(p) = path {
                        debug!("Loading ROM from {:?}", p);
                        let rom = load_rom_buffer(&p);
                        debug!("ROM size: {} bytes", rom.len());
                        run_emulator(rom, hardware);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            });

            // Show splash until the user provides a ROM, then switch to game loop.
            gui.run(false);
        }
    }
}

fn run_emulator(rom: Vec<u8>, hardware: Hardware) {
    let mut emu = Emu::new(rom, hardware.get_vram(), hardware.get_keys_states());
    let mut audio = AudioOutput::new();
    emu.show_cartridge_info();

    info!("Starting emulation loop");

    while hardware.get_gui_is_alive() {
        let frame_start = Instant::now();

        emu.process_frame();
        let samples = emu.drain_audio_samples();
        if let Some(audio) = audio.as_mut() {
            if hardware.is_muted() {
                audio.push_samples(samples.into_iter().map(|_| 0.0));
            } else {
                audio.push_samples(samples);
            }
        }

        let frame_time = frame_start.elapsed();
        let target_frame_time = Duration::from_micros(1_000_000 / TARGET_FPS);

        if frame_time < target_frame_time {
            sleep(target_frame_time - frame_time);
        } else {
            warn!("Frame took longer than target frame time");
        }
    }
}
