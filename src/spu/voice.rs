use crate::{common::MemoryCell};

pub struct Voice<const NUM: u8>;

impl<const NUM: u8> Voice<NUM> {
    const OFFSET: usize = NUM as usize * 0x10;
    const LEFT_VOL: MemoryCell<u16> = MemoryCell::new(0x1F80_1C00 + Self::OFFSET);
    const RIGHT_VOL: MemoryCell<u16> = MemoryCell::new(0x1F80_1C02 + Self::OFFSET);
    const SAMPLE_RATE: MemoryCell<u16> = MemoryCell::new(0x1F80_1C04 + Self::OFFSET);
    const START_ADDR: MemoryCell<u16> = MemoryCell::new(0x1F80_1C06 + Self::OFFSET);
    const ADSR: MemoryCell<u32> = MemoryCell::new(0x1F80_1C08 + Self::OFFSET);
    const REPEAT_ADDR: MemoryCell<u16> = MemoryCell::new(0x1F80_1C0E + Self::OFFSET);
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
        let current = Self::KEY_ON.get();
        let key_on_mask = current | (1u32 << NUM);
        Self::KEY_ON.set(key_on_mask);

        self
    }
}