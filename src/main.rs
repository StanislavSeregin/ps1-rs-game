#![no_std]
#![no_main]

mod common;
mod spu;

use psx::gpu::VideoMode;
use psx::{dprintln, Framebuffer};

use crate::spu::{SPU};

const SAMPLE_DATA: &[u8] = include_bytes_skip_vag_header!("../samples/3dfx.vag");

#[unsafe(no_mangle)]
fn main() {
    let buf0 = (0, 0);
    let buf1 = (0, 240);
    let res = (320, 240);
    let txt_offset = (0, 8);
    let mut fb = Framebuffer::new(buf0, buf1, res, VideoMode::NTSC, None).unwrap();
    let font = fb.load_default_font();
    let mut txt = font.new_text_box(txt_offset, res);

    let mut spu = SPU::new();
    let sample = spu.sample_manager.load(SAMPLE_DATA).expect("Failed to load sample");
    let mut voice = sample.bind_to_voice::<0>(0x1000, 0x3FFF);
    voice.play();
    
    loop {
        txt.reset();
        dprintln!(txt, "Audio Playing: 3dfx.vag");
        
        fb.draw_sync();
        fb.wait_vblank();
        fb.swap();
    }
}
