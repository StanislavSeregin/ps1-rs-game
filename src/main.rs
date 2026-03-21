#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

mod common;
pub mod runtime;
pub mod spu2;

use psx::gpu::VideoMode;
use psx::{dprintln, Framebuffer};
use runtime::TaskStack;
use spu2::*;

// ---------------------------------------------------------------------------
// Samples — carved from file_all.spu (same offsets as audio-sample branch)
// ---------------------------------------------------------------------------

const SAMPLE_A: SampleId = SampleId(0);
const SAMPLE_B: SampleId = SampleId(1);
const SAMPLE_C: SampleId = SampleId(2);

const PROJECT: SoundProject<3> = SoundProject {
    samples: [
        include_bytes_skip!("../samples/file_all.spu", 0, 13500),
        include_bytes_skip!("../samples/file_all.spu", 13600, 8400),
        include_bytes_skip!("../samples/file_all.spu", 22000),
    ],
    layout: VoiceLayout::new((0, 16), (16, 8)),
};

// ---------------------------------------------------------------------------
// Patterns — two 16-row patterns across 2 channels, 120 BPM
// ---------------------------------------------------------------------------

const PATTERN_A: Pattern<2, 16> = Pattern::new()
    .set(0,  0, Cell::note(SAMPLE_A, Pitch(0x1000)))
    .set(4,  0, Cell::note(SAMPLE_B, Pitch(0x0800)))
    .set(8,  0, Cell::note(SAMPLE_C, Pitch(0x1200)))
    .set(12, 0, Cell::note(SAMPLE_A, Pitch(0x0600)))
    .set(2,  1, Cell::note(SAMPLE_B, Pitch(0x0400)))
    .set(6,  1, Cell::note(SAMPLE_C, Pitch(0x0900)))
    .set(10, 1, Cell::note(SAMPLE_A, Pitch(0x1100)))
    .set(14, 1, Cell::note(SAMPLE_B, Pitch(0x0700)));

const PATTERN_B: Pattern<2, 16> = Pattern::new()
    .set(0,  0, Cell::note(SAMPLE_C, Pitch(0x0900)))
    .set(4,  0, Cell::note(SAMPLE_A, Pitch(0x1100)))
    .set(8,  0, Cell::note(SAMPLE_B, Pitch(0x0600)))
    .set(12, 0, Cell::note(SAMPLE_C, Pitch(0x1000)))
    .set(2,  1, Cell::note(SAMPLE_A, Pitch(0x0800)))
    .set(6,  1, Cell::note(SAMPLE_B, Pitch(0x1200)))
    .set(10, 1, Cell::note(SAMPLE_C, Pitch(0x0400)))
    .set(14, 1, Cell::note(SAMPLE_A, Pitch(0x0700)));

// ---------------------------------------------------------------------------
// Music coroutine — song structure expressed as control flow
// ---------------------------------------------------------------------------

static MUSIC_STACK: TaskStack<2048> = TaskStack::new();

extern "C" fn music_task() {
    let mut e = Engine::take().unwrap();
    e.load_project(&PROJECT);
    e.set_bpm(30);

    loop {
        e.reset_pattern_counter();
        // Pattern A twice, then Pattern B twice — forever
        e.play_pattern(&PATTERN_A);
        e.play_pattern(&PATTERN_A);
        e.play_pattern(&PATTERN_B);
        e.play_pattern(&PATTERN_B);
    }
}

// ---------------------------------------------------------------------------
// Entry point — task 0 (rendering + game logic)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
fn main() {
    runtime::init();
    runtime::spawn(music_task, &MUSIC_STACK);

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
