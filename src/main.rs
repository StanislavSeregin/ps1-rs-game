#![no_std]
#![no_main]

mod common;
mod spu;

use core::borrow;
use core::cell::RefCell;

use psx::gpu::VideoMode;
use psx::{dprintln, Framebuffer, TextBox};
use psx::include_words;
use spu::SpuUpload;

#[unsafe(no_mangle)]
fn main() {
    let mut debug_func = init_debug();

    debug_func("srak!!!");

    let audio_sample = include_words!("./../assets/audio/test.adpcm");
    let mut spu_upload = SpuUpload::new();
    let mut spu_sample = spu_upload.load(audio_sample, &debug_func);
    spu_sample.play(&debug_func);

    loop {
        // TODO
    }
}

fn init_debug() -> impl Fn(&str) {
    let buf0 = (0, 0);
    let buf1 = (0, 240);
    let res = (320, 240);
    let txt_offset = (0, 64);
    let mut fb = Framebuffer::new(buf0, buf1, res, VideoMode::NTSC, None).unwrap();
    let font = fb.load_default_font();
    let txt = RefCell::new((font.new_text_box(txt_offset, res), fb));

    move |message: &str| {
        let mut txt = txt.borrow_mut();
        txt.0.reset();
        for ch in message.as_bytes() {
            txt.0.print_char(ch.clone());
            txt.0.move_right(1);
        }
        
        txt.1.draw_sync();
        txt.1.wait_vblank();
        txt.1.swap();
    }
}
