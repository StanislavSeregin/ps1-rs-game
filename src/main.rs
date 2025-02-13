#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

mod common;
mod spu;
mod helpers;

use core::fmt::Write;
use psx::include_words;
use spu::SpuUpload;
use helpers::DebugPrinter;

#[unsafe(no_mangle)]
fn main() {
    let mut debug_printer = DebugPrinter::new();
    for i in 0..1000 {
        print_debug!(debug_printer, "Some text: {i}");
    }

    let audio_sample = include_words!("./../assets/audio/test.adpcm");
    let mut spu_upload = SpuUpload::new();
    let mut spu_sample = spu_upload.load(audio_sample);
    spu_sample.play();

    loop {
        // TODO
    }
}
