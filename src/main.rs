#![no_std]
#![no_main]

use core::ptr;
use psx::gpu::VideoMode;
use psx::{dprintln, Framebuffer};

const SPU_BASE: u32 = 0x1F801C00;

const SPU_VOICE_LEFT_VOL: u32 = SPU_BASE + 0x00;
const SPU_VOICE_RIGHT_VOL: u32 = SPU_BASE + 0x02;
const SPU_VOICE_SAMPLE_RATE: u32 = SPU_BASE + 0x04;
const SPU_VOICE_START_ADDR: u32 = SPU_BASE + 0x06;
const SPU_VOICE_ADSR: u32 = SPU_BASE + 0x08;
const SPU_VOICE_REPEAT_ADDR: u32 = SPU_BASE + 0x0E;

const SPU_KEY_ON: u32 = 0x1F801D88;
const SPU_CONTROL: u32 = 0x1F801DAA;
const SPU_STATUS: u32 = 0x1F801DAE;
const SPU_MAIN_VOL_LEFT: u32 = 0x1F801D80;
const SPU_MAIN_VOL_RIGHT: u32 = 0x1F801D82;
const SPU_RAM_DATA_TRANSFER_ADDR: u32 = 0x1F801DA6;
const SPU_RAM_DATA_TRANSFER_FIFO: u32 = 0x1F801DA8;
const SPU_RAM_DATA_TRANSFER_CONTROL: u32 = 0x1F801DAC;

const SAMPLE_DATA: &[u8] = include_bytes!("3dfx.vag");

unsafe fn write_spu_reg16(addr: u32, value: u16) {
    ptr::write_volatile(addr as *mut u16, value);
}

unsafe fn read_spu_reg16(addr: u32) -> u16 {
    ptr::read_volatile(addr as *const u16)
}

unsafe fn write_spu_reg32(addr: u32, value: u32) {
    ptr::write_volatile(addr as *mut u32, value);
}

unsafe fn init_spu() {
    // Сброс SPU
    write_spu_reg16(SPU_CONTROL, 0xC000);
    
    // Задержка для сброса
    for _ in 0..100000 {
    }
    
    // Включить SPU и отключить CD Audio
    write_spu_reg16(SPU_CONTROL, 0xC001);
    
    // Установить основную громкость (максимальная)
    write_spu_reg16(SPU_MAIN_VOL_LEFT, 0x3FFF);
    write_spu_reg16(SPU_MAIN_VOL_RIGHT, 0x3FFF);
    
    // Дополнительная задержка
    for _ in 0..50000 {
    }
}

unsafe fn load_vag_to_spu_ram(vag_data: &[u8], spu_addr: u16) {
    let audio_data = &vag_data[48..];
    
    // Адрес в SPU RAM (в единицах по 8 байт)
    write_spu_reg16(SPU_RAM_DATA_TRANSFER_ADDR, spu_addr);
    
    // Установить режим записи в SPU RAM
    write_spu_reg16(SPU_RAM_DATA_TRANSFER_CONTROL, 0x0004);
    
    // Загрузить данные
    for chunk in audio_data.chunks(2) {
        if chunk.len() == 2 {
            let word = (chunk[1] as u16) << 8 | chunk[0] as u16;
            write_spu_reg16(SPU_RAM_DATA_TRANSFER_FIFO, word);
        } else if chunk.len() == 1 {
            let word = chunk[0] as u16;
            write_spu_reg16(SPU_RAM_DATA_TRANSFER_FIFO, word);
        }
        
        // Небольшая задержка между записями
        for _ in 0..10 {
        }
    }
    
    // Завершить передачу
    write_spu_reg16(SPU_RAM_DATA_TRANSFER_CONTROL, 0x0000);
    
    // Дождаться завершения
    for _ in 0..10000 {
    }
}

unsafe fn play_sample_on_voice(voice: u8, spu_addr: u16, sample_rate: u16, volume: u16) {
    let voice_offset = voice as u32 * 0x10;
    
    // Установить громкость канала
    write_spu_reg16(SPU_VOICE_LEFT_VOL + voice_offset, volume);
    write_spu_reg16(SPU_VOICE_RIGHT_VOL + voice_offset, volume);
    
    // Установить адрес начала семпла (в единицах по 8 байт)
    write_spu_reg16(SPU_VOICE_START_ADDR + voice_offset, spu_addr);
    
    // Установить адрес повтора (тот же что и начальный для зацикливания)
    write_spu_reg16(SPU_VOICE_REPEAT_ADDR + voice_offset, spu_addr);
    
    // Установить частоту дискретизации
    write_spu_reg16(SPU_VOICE_SAMPLE_RATE + voice_offset, sample_rate);
    
    // Настроить ADSR (быстрая атака, медленное затухание, высокий sustain, медленный release)
    write_spu_reg32(SPU_VOICE_ADSR + voice_offset, 0x80FF8000);
    
    // Небольшая задержка перед запуском
    for _ in 0..1000 {
    }
    
    // Запустить канал
    let key_on_mask = 1u32 << voice;
    write_spu_reg32(SPU_KEY_ON, key_on_mask);
}

#[unsafe(no_mangle)]
fn main() {
    let buf0 = (0, 0);
    let buf1 = (0, 240);
    let res = (320, 240);
    let txt_offset = (0, 8);
    let mut fb = Framebuffer::new(buf0, buf1, res, VideoMode::NTSC, None).unwrap();
    let font = fb.load_default_font();
    let mut txt = font.new_text_box(txt_offset, res);
    
    unsafe {
        init_spu();
        
        let sample_addr: u16 = 0x1000;
        load_vag_to_spu_ram(SAMPLE_DATA, sample_addr);
        
        let sample_rate: u16 = 0x1000;
        let volume: u16 = 0x3FFF;
        play_sample_on_voice(0, sample_addr, sample_rate, volume);
    }
    
    loop {
        txt.reset();
        dprintln!(txt, "Audio Playing: 3dfx.vag");
        dprintln!(txt, "Channel: 0");
        dprintln!(txt, "Voice Volume: {:#X}", 0x3FFF);
        dprintln!(txt, "Sample Rate: {:#X}", 0x1000);
        dprintln!(txt, "Sample Addr: {:#X}", 0x1000);
        
        unsafe {
            let main_vol_l = read_spu_reg16(SPU_MAIN_VOL_LEFT);
            let main_vol_r = read_spu_reg16(SPU_MAIN_VOL_RIGHT);
            let spu_ctrl = read_spu_reg16(SPU_CONTROL);
            let spu_status = read_spu_reg16(SPU_STATUS);
            
            dprintln!(txt, "Main Vol L: {:#X}", main_vol_l);
            dprintln!(txt, "Main Vol R: {:#X}", main_vol_r);
            dprintln!(txt, "SPU Ctrl: {:#X}", spu_ctrl);
            dprintln!(txt, "SPU Status: {:#X}", spu_status);
        }
        
        fb.draw_sync();
        fb.wait_vblank();
        fb.swap();
    }
}
