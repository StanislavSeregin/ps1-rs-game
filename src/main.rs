#![no_std]
#![no_main]

mod common;

use psx::gpu::VideoMode;
use psx::{dprintln, Framebuffer};

#[unsafe(no_mangle)]
fn main() {
    let buf0 = (0, 0);
    let buf1 = (0, 240);
    let res = (320, 240);
    let txt_offset = (0, 8);
    let mut fb = Framebuffer::new(buf0, buf1, res, VideoMode::NTSC, None).unwrap();
    let font = fb.load_default_font();
    let mut txt = font.new_text_box(txt_offset, res);

    let mut count: usize = 0;
    loop {
        txt.reset();

        count += 1;
        dprintln!(txt, "Count: {}", count);

        fb.draw_sync();
        fb.wait_vblank();
        fb.swap();
    }
}
