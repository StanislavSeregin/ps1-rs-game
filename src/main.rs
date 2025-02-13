#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

mod common;
mod spu;
mod helpers;

use psx::dprintln;
use psx::include_words;
use spu::SpuUpload;
use helpers::{DebugPrinter, delay};

#[unsafe(no_mangle)]
fn main() {
    let mut debug_printer = DebugPrinter::new();

    debug_printer.print("wewertqw34");

    delay();

    debug_printer.print("wewertqw4545");

    delay();

    let a = 234 as u32;
    debug_printer.write(|mut kek| {
        dprintln!(kek, "Kek: {}", a);
    });

    let audio_sample = include_words!("./../assets/audio/test.adpcm");
    let mut spu_upload = SpuUpload::new();
    let mut spu_sample = spu_upload.load(audio_sample);
    spu_sample.play();

    loop {
        // TODO
    }
}
