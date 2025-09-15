#![no_std]
#![no_main]

mod common;
mod spu;

use psx::gpu::VideoMode;
use psx::{dprintln, Framebuffer};

use crate::spu::{Spu};

const SAMPLE_DATA_1: &[u8] = include_bytes_skip_take!("../samples/file_all.spu", 0, 13500);
const SAMPLE_DATA_2: &[u8] = include_bytes_skip_take!("../samples/file_all.spu", 13600, 8400);
const SAMPLE_DATA_3: &[u8] = include_bytes_skip_take!("../samples/file_all.spu", 22000);

#[unsafe(no_mangle)]
fn main() {
    let buf0 = (0, 0);
    let buf1 = (0, 240);
    let res = (320, 240);
    let txt_offset = (0, 8);
    let mut fb = Framebuffer::new(buf0, buf1, res, VideoMode::NTSC, None).unwrap();
    let font = fb.load_default_font();
    let mut txt = font.new_text_box(txt_offset, res);

    let mut spu = Spu::take().expect("Failed to init SPU");
    let sample = spu.sampler.load(SAMPLE_DATA_3).expect("Failed to load sample");
    let mut voice = sample.bind_to_voice::<0>(0x300, 0x3FFF);
    voice.play();
    
    loop {
        txt.reset();
        dprintln!(txt, "Audio Playing");
        
        fb.draw_sync();
        fb.wait_vblank();
        fb.swap();
    }
}
