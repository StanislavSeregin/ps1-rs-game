use crate::{common::MemoryCell};

const SPU_BASE: usize = 0x1F801C00;
const SPU_VOICE_LEFT_VOL: usize = SPU_BASE + 0x00;
const SPU_VOICE_RIGHT_VOL: usize = SPU_BASE + 0x02;
const SPU_VOICE_SAMPLE_RATE: usize = SPU_BASE + 0x04;
const SPU_VOICE_START_ADDR: usize = SPU_BASE + 0x06;
const SPU_VOICE_ADSR: usize = SPU_BASE + 0x08;
const SPU_VOICE_REPEAT_ADDR: usize = SPU_BASE + 0x0E;

pub struct SPU {
    pub sample_addr: u16
}

impl SPU {
    const SPU_CONTROL: MemoryCell<u16> = MemoryCell::new(0x1F80_1DAA);
    const SPU_MAIN_VOL_LEFT: MemoryCell<u32> = MemoryCell::new(0x1F80_1D80);
    const SPU_MAIN_VOL_RIGHT: MemoryCell<u32> = MemoryCell::new(0x1F80_1D82);

    const SPU_RAM_DATA_TRANSFER_ADDR: MemoryCell<u16> = MemoryCell::new(0x1F80_1DA6);
    const SPU_RAM_DATA_TRANSFER_FIFO: MemoryCell<u16> = MemoryCell::new(0x1F80_1DA8);
    const SPU_RAM_DATA_TRANSFER_CONTROL: MemoryCell<u16> = MemoryCell::new(0x1F80_1DAC);

    pub fn new() -> Self {
        // Сброс SPU
        Self::SPU_CONTROL.set(0xC000);

        // Задержка для сброса
        for _ in 0..100000 {/* Nothing */}

        // Включить SPU и отключить CD Audio
        Self::SPU_CONTROL.set(0xC001);

        // Установить основную громкость (максимальная)
        Self::SPU_MAIN_VOL_LEFT.set(0x3FFF);
        Self::SPU_MAIN_VOL_RIGHT.set(0x3FFF);

        // Дополнительная задержка
        for _ in 0..50000 {/* Nothing */}

        SPU {
            sample_addr: 0x1000
        }
    }

    pub fn load_vag_to_spu_ram(&mut self, vag_data: &[u8]) -> &Self {
        // Исключаем заголовочные 48 байт
        let audio_data = &vag_data[48..];

        // Адрес в SPU RAM (в единицах по 8 байт)
        Self::SPU_RAM_DATA_TRANSFER_ADDR.set(self.sample_addr);

        // Установить режим записи в SPU RAM
        Self::SPU_RAM_DATA_TRANSFER_CONTROL.set(0x0004);

        // Загрузить данные
        for chunk in audio_data.chunks(2) {
            if chunk.len() == 2 {
                let word = (chunk[1] as u16) << 8 | chunk[0] as u16;
                Self::SPU_RAM_DATA_TRANSFER_FIFO.set(word);
            } else if chunk.len() == 1 {
                let word = chunk[0] as u16;
                Self::SPU_RAM_DATA_TRANSFER_FIFO.set(word);
            }
            
            // Небольшая задержка между записями
            for _ in 0..10 {/* Nothing */}
        }

        self
    }
}

pub struct Voice<const NUM: u8>;

impl<const NUM: u8> Voice<NUM> {
    const OFFSET: usize = NUM as usize * 0x10;
    const LEFT_VOL: MemoryCell<u16> = MemoryCell::new(SPU_VOICE_LEFT_VOL + Self::OFFSET);
    const RIGHT_VOL: MemoryCell<u16> = MemoryCell::new(SPU_VOICE_RIGHT_VOL + Self::OFFSET);
    const START_ADDR: MemoryCell<u16> = MemoryCell::new(SPU_VOICE_START_ADDR + Self::OFFSET);
    const REPEAT_ADDR: MemoryCell<u16> = MemoryCell::new(SPU_VOICE_REPEAT_ADDR + Self::OFFSET);
    const SAMPLE_RATE: MemoryCell<u16> = MemoryCell::new(SPU_VOICE_SAMPLE_RATE + Self::OFFSET);
    const ADSR: MemoryCell<u32> = MemoryCell::new(SPU_VOICE_ADSR + Self::OFFSET);
    const KEY_ON: MemoryCell<u32> = MemoryCell::new(0x1F80_1D88);

    pub fn new(spu_addr: u16, sample_rate: u16, volume: u16) -> Self {
        // Установить громкость каналов
        Self::LEFT_VOL.set(volume);
        Self::RIGHT_VOL.set(volume);

        // Установить адрес начала семпла (в единицах по 8 байт)
        Self::START_ADDR.set(spu_addr);

        // Установить адрес повтора (тот же что и начальный для зацикливания)
        Self::REPEAT_ADDR.set(spu_addr);

        // Установить частоту дискретизации
        Self::SAMPLE_RATE.set(sample_rate);

        // Настроить ADSR (быстрая атака, медленное затухание, высокий sustain, медленный release)
        Self::ADSR.set(0x80FF_8000);

        Voice
    }

    pub fn play(&mut self) -> &Self {
        let key_on_mask = 1u32 << NUM;
        Self::KEY_ON.set(key_on_mask);

        self
    }
}
