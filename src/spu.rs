use core::fmt::Write;

use arrayvec::ArrayString;

use crate::common::MemoryCell;

pub struct SpuUpload {
    pub current_address: u16,
}

impl SpuUpload {
    const SPU_TRANSFER_ADDR: MemoryCell<u16> = MemoryCell::new(0x1F80_1DA6);
    const DMA_BASE_ADDR: MemoryCell<u32> = MemoryCell::new(0x1F80_10F0);
    const DMA_BLOCK_SIZE: MemoryCell<u32> = MemoryCell::new(0x1F80_10F4);
    const DMA_CONTROL: MemoryCell<u32> = MemoryCell::new(0x1F80_10F8 );

    pub fn new() -> Self {
        SpuUpload {
            current_address: 0x1000
        }
    }

    pub fn load(&mut self, data: &[u32]) -> SpuSample {
        let sample = SpuSample::new(self.current_address);
        for (i, chunk) in data.chunks(4096).enumerate() {
            let mut txt = ArrayString::<64>::new();
            write!(txt, "Start: {i}");
            let (chunk_ptr, chunk_length) = (chunk.as_ptr(), chunk.len());
            Self::SPU_TRANSFER_ADDR.set(self.current_address);
            Self::DMA_BASE_ADDR.set(chunk_ptr as u32);
            Self::DMA_BLOCK_SIZE.set(chunk_length as u32);
            Self::DMA_CONTROL.set(0x0100_0201);
            let kek = Self::DMA_CONTROL.get();
            while kek > 0 {
                write!(txt, "; {kek}");
            }
            self.current_address += chunk_length as u16;
        }

        sample
    }
}

pub struct SpuSample {
    start_address: u16,
}

impl SpuSample {
    const VOICE_START_ADDR: MemoryCell<u16> = MemoryCell::new(0x1F80_1C06);
    const VOICE_REPEAT_ADDR: MemoryCell<u16> = MemoryCell::new(0x1F80_1C0E);
    const VOICE_PITCH: MemoryCell<u16> = MemoryCell::new(0x1F80_1C04);
    const VOICE_VOLUME_LEFT: MemoryCell<u16> = MemoryCell::new(0x1F80_1C00);
    const VOICE_VOLUME_RIGHT: MemoryCell<u16> = MemoryCell::new(0x1F80_1C02);
    const KEY_ON: MemoryCell<u32> = MemoryCell::new(0x1F80_1D88);

    pub fn new(start_address: u16) -> Self {
        SpuSample {
            start_address
        }
    }

    pub fn play(&mut self) -> &Self {
        Self::VOICE_START_ADDR.set(self.start_address);
        Self::VOICE_REPEAT_ADDR.set(self.start_address);
        Self::VOICE_PITCH.set(0x1000);
        Self::VOICE_VOLUME_LEFT.set(0x3FFF);
        Self::VOICE_VOLUME_RIGHT.set(0x3FFF);
        Self::KEY_ON.set(0x00000001);
        self
    }
}