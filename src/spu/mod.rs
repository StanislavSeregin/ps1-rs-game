mod macros;
mod sampler;
mod voice;
pub mod tracker;

use crate::{common::MemoryCell};

pub use self::sampler::*;
pub use self::voice::*;
pub use self::tracker::*;

pub struct Spu {
    pub sampler: Sampler,
}

impl Spu {
    const SPU_CONTROL: MemoryCell<u16> = MemoryCell::new(0x1F80_1DAA);
    const SPU_MAIN_VOL_LEFT: MemoryCell<u32> = MemoryCell::new(0x1F80_1D80);
    const SPU_MAIN_VOL_RIGHT: MemoryCell<u32> = MemoryCell::new(0x1F80_1D82);

    pub fn take() -> Option<Self> {
        static mut ALREARY_INITIALIZED: bool = false;
        if unsafe { ALREARY_INITIALIZED } {
            None
        } else {
            unsafe { ALREARY_INITIALIZED = true };
            Some(Spu::new())
        }
    }

    fn new() -> Self {
        Self::SPU_CONTROL.set(0xC000);
        Self::SPU_CONTROL.set(0xC001);
        Self::SPU_MAIN_VOL_LEFT.set(0x3FFF);
        Self::SPU_MAIN_VOL_RIGHT.set(0x3FFF);

        Spu {
            sampler: Sampler::new(),
        }
    }
}
