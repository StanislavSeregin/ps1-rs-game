#![no_std]
#![no_main]

mod common;
mod spu;

use psx::gpu::VideoMode;
use psx::{dprintln, Framebuffer};

use crate::spu::{Spu, Sequencer, Song, Track, Pattern, Note};

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

    let mut sequencer: Sequencer<4> = Sequencer::new(&mut spu.sampler);
    sequencer.load_sample(0, SAMPLE_DATA_1).expect("Failed to load sample 1");
    sequencer.load_sample(1, SAMPLE_DATA_2).expect("Failed to load sample 2");
    sequencer.load_sample(2, SAMPLE_DATA_3).expect("Failed to load sample 3");

    const SONG: Song<2, 2, 16> = Song::new(120)
        .with_track(0, Track::new()
            .with_pattern(0, Pattern::new()
                .with_note(0, Note::new(1, 0x1000))
                .with_note(4, Note::new(2, 0x800))
                .with_note(8, Note::new(3, 0x1200))
                .with_note(12, Note::new(1, 0x600))
            )
            .with_order(&[0, 0])
        )
        .with_track(1, Track::new()
            .with_pattern(0, Pattern::new()
                .with_note(2, Note::new(2, 0x400))
                .with_note(6, Note::new(3, 0x900))
                .with_note(10, Note::new(1, 0x1100))
                .with_note(14, Note::new(2, 0x700))
            )
            .with_order(&[0, 0])
        );

    sequencer.play_song(&SONG);

    loop {
        sequencer.update(&SONG);

        txt.reset();
        let (pattern, row) = sequencer.get_position();
        dprintln!(txt, "Tracker Playing");
        dprintln!(txt, "Pattern: {}, Row: {}", pattern, row);
        dprintln!(txt, "Status: {}", if sequencer.is_playing() { "Playing" } else { "Stopped" });

        fb.draw_sync();
        fb.wait_vblank();
        fb.swap();
    }
}
