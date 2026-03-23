#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

mod common;
pub mod runtime;
pub mod spu2;
mod song;

use psx::gpu::VideoMode;
use psx::{dprintln, Framebuffer};
use spu2::audio_status;

#[unsafe(no_mangle)]
fn main() {
    runtime::init();
    runtime::spawn(song::music_task, &song::MUSIC_STACK);

    let buf0 = (0, 0);
    let buf1 = (0, 240);
    let res = (320, 240);
    let txt_offset = (0, 8);
    let mut fb = Framebuffer::new(buf0, buf1, res, VideoMode::NTSC, None).unwrap();
    let font = fb.load_default_font();
    let mut txt = font.new_text_box(txt_offset, res);

    let mut frame: u32 = 0;

    loop {
        txt.reset();
        frame += 1;

        let status = audio_status();
        dprintln!(txt, "SPU2 Engine Demo");
        dprintln!(txt, "----------------");
        dprintln!(txt, "");
        dprintln!(txt, "Music coroutine");
        dprintln!(txt, "  status:  {}", if status.playing { "Playing" } else { "Stopped" });
        dprintln!(txt, "  pattern: {}", status.current_pattern);
        dprintln!(txt, "  row:     {}", status.current_row);
        dprintln!(txt, "");
        dprintln!(txt, "Render loop");
        dprintln!(txt, "  frame: {}", frame);

        fb.draw_sync();
        runtime::yield_now();
        fb.wait_vblank();
        fb.swap();
    }
}
