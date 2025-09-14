#![no_std]
#![no_main]

mod common;
mod spu;
use psx::gpu::VideoMode;
use psx::{dprintln, Framebuffer};

use crate::spu::{Voice, SPU};

const SAMPLE_DATA: &[u8] = include_bytes!("3dfx.vag");

#[unsafe(no_mangle)]
fn main() {
    let buf0 = (0, 0);
    let buf1 = (0, 240);
    let res = (320, 240);
    let txt_offset = (0, 8);
    let mut fb = Framebuffer::new(buf0, buf1, res, VideoMode::NTSC, None).unwrap();
    let font = fb.load_default_font();
    let mut txt = font.new_text_box(txt_offset, res);

    play_audio();
    
    loop {
        txt.reset();
        dprintln!(txt, "Audio Playing: 3dfx.vag");
        
        fb.draw_sync();
        fb.wait_vblank();
        fb.swap();
    }
}

fn play_audio() {
    let sample_rate: u16 = 0x1000;
    let volume: u16 = 0x3FFF;

    let mut spu = SPU::new();
    let sample0 = spu.load_sample(SAMPLE_DATA).expect("Failed to load sample");
    let sample1 = spu.load_sample(SAMPLE_DATA).expect("Failed to load sample");

    let mut voice0 = Voice::<0>::new(sample0.spu_addr, sample_rate, volume);
    voice0.play();

    let mut voice1 = Voice::<1>::new(sample1.spu_addr, sample_rate, volume);
    voice1.play();
}
