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
        Self::SPU_CONTROL.set(0xC000);
        Self::SPU_CONTROL.set(0xC001);
        Self::SPU_MAIN_VOL_LEFT.set(0x3FFF);
        Self::SPU_MAIN_VOL_RIGHT.set(0x3FFF);

        SPU {
            sample_manager: SampleManager::new(),
        }
    }
}
