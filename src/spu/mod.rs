mod macros;
mod sample_manager;
mod voice;

use crate::{common::MemoryCell};

pub use self::sample_manager::*;
pub use self::voice::*;

pub struct SPU {
    pub sample_manager: SampleManager,
}

impl SPU {
    const SPU_CONTROL: MemoryCell<u16> = MemoryCell::new(0x1F80_1DAA);
    const SPU_MAIN_VOL_LEFT: MemoryCell<u32> = MemoryCell::new(0x1F80_1D80);
    const SPU_MAIN_VOL_RIGHT: MemoryCell<u32> = MemoryCell::new(0x1F80_1D82);

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
            sample_manager: SampleManager::new(),
        }
    }
}
