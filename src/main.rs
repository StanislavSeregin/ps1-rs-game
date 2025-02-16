#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

mod dma;
mod common;
mod helpers;

use core::arch::asm;
use psx::include_words;
use dma::spu::SpuUpload;

#[unsafe(no_mangle)]
fn main() {
    let audio_sample = include_words!("./../assets/audio/test.adpcm");
    let mut spu_upload = SpuUpload::new();
    let mut spu_sample = spu_upload.load(audio_sample);
    spu_sample.play();

    loop {
        unsafe {
            asm!("nop");
        }
    }
}
