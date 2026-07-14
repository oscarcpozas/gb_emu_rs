use gb_emu::{Emu, GameBoyKey};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

#[wasm_bindgen]
pub struct WasmEmu {
    emu: Emu,
    frame_buffer: Arc<Mutex<Vec<u32>>>,
    keys: Arc<Mutex<HashMap<GameBoyKey, bool>>>,
}

#[wasm_bindgen]
impl WasmEmu {
    #[wasm_bindgen(constructor)]
    pub fn new(rom: &[u8]) -> Self {
        console_error_panic_hook::set_once();

        let frame_buffer = Arc::new(Mutex::new(vec![0; SCREEN_WIDTH * SCREEN_HEIGHT]));
        let keys = Arc::new(Mutex::new(init_key_states()));
        let emu = Emu::new(rom.to_vec(), frame_buffer.clone(), keys.clone());

        Self {
            emu,
            frame_buffer,
            keys,
        }
    }

    pub fn step_frame(&mut self) {
        self.emu.process_frame();
    }

    pub fn frame_rgba(&self) -> Vec<u8> {
        let frame = self.frame_buffer.lock().unwrap();
        let mut rgba = Vec::with_capacity(frame.len() * 4);

        for color in frame.iter().copied() {
            rgba.push(((color >> 16) & 0xFF) as u8);
            rgba.push(((color >> 8) & 0xFF) as u8);
            rgba.push((color & 0xFF) as u8);
            rgba.push(0xFF);
        }

        rgba
    }

    pub fn drain_audio_samples(&mut self) -> Vec<f32> {
        self.emu.drain_audio_samples()
    }

    pub fn set_key(&mut self, key: &str, pressed: bool) {
        if let Some(key) = parse_key(key) {
            self.keys.lock().unwrap().insert(key, pressed);
        }
    }
}

fn init_key_states() -> HashMap<GameBoyKey, bool> {
    [
        GameBoyKey::Right,
        GameBoyKey::Left,
        GameBoyKey::Up,
        GameBoyKey::Down,
        GameBoyKey::A,
        GameBoyKey::B,
        GameBoyKey::Select,
        GameBoyKey::Start,
    ]
    .into_iter()
    .map(|key| (key, false))
    .collect()
}

fn parse_key(key: &str) -> Option<GameBoyKey> {
    match key {
        "right" => Some(GameBoyKey::Right),
        "left" => Some(GameBoyKey::Left),
        "up" => Some(GameBoyKey::Up),
        "down" => Some(GameBoyKey::Down),
        "a" => Some(GameBoyKey::A),
        "b" => Some(GameBoyKey::B),
        "select" => Some(GameBoyKey::Select),
        "start" => Some(GameBoyKey::Start),
        _ => None,
    }
}
