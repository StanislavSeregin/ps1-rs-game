use spu_control::{SpuRamTransferMode, SpuControl};
use spu_status::SpuStatus;
use core::fmt::Write;
mod spu_control;
mod spu_status;

use crate::{common::MemoryCell, helpers::DisplayLogger, write_to_display};

pub struct SpuUpload {
    pub current_address: u32,
}

impl SpuUpload {
    const SPU_DMA_MEMORY_ADDRESS_REG: MemoryCell<u32> = MemoryCell::new(0x1F80_10C0);
    const SPU_DMA_BLOCK_CONTROL_REG: MemoryCell<u32> = MemoryCell::new(0x1F80_10C4);
    const SPU_DMA_CHANNEL_CONTROL_REG: MemoryCell<u32> = MemoryCell::new(0x1F80_10C8);

    const SPU_CONTROL_REG: MemoryCell<u16> = MemoryCell::new(0x1F80_1DAA);
    const SPU_RAM_TRANSFER_ADDRESS_REG: MemoryCell<u16> = MemoryCell::new(0x1F80_1DA6);
    const SPU_RAM_TRANSFER_CONTROL_REG: MemoryCell<u32> = MemoryCell::new(0x1F80_1DAC);
    const SPU_STATUS_REG: MemoryCell<u16> = MemoryCell::new(0x1F80_1DAE);

    pub fn new() -> Self {
        SpuUpload {
            current_address: 0x1000
        }
    }

    pub fn load(&mut self, data: &[u32]) -> SpuSample {
        let mut logger = DisplayLogger::new();

        Self::SPU_RAM_TRANSFER_CONTROL_REG.set(0x0004);

        let mut spu_control = SpuControl::new();
        spu_control.sound_ram_transfer_mode = SpuRamTransferMode::Stop;
        let spu_control_raw = spu_control.to_u16();
        Self::SPU_CONTROL_REG.set(spu_control_raw);

        let transfer_adress = (self.current_address >> 3) as u16;
        Self::SPU_RAM_TRANSFER_ADDRESS_REG.set(transfer_adress);

        spu_control.sound_ram_transfer_mode = SpuRamTransferMode::DMAWrite;
        let spu_control_raw = spu_control.to_u16();
        Self::SPU_CONTROL_REG.set(spu_control_raw);

        while {
            let spu_status_raw = Self::SPU_STATUS_REG.get();
            let ram_transfer_mode = SpuStatus::get_sound_ram_transfer_mode(&spu_status_raw);
            ram_transfer_mode != SpuRamTransferMode::DMAWrite
        } {}

        // TODO TEST AREA
        let main_ptr = data.as_ptr();
        write_to_display!(logger, "Base: {main_ptr:?}");

        for n in data.iter() {
            let ptr = n as *const u32;
            let ptr_u32 = ptr as u32;
            write_to_display!(logger, "Iter: {ptr_u32:?}");
        }

        let ptr = data.as_ptr();

        write_to_display!(logger, "{ptr:?}");

        Self::SPU_DMA_MEMORY_ADDRESS_REG.set(ptr as u32);
        Self::SPU_DMA_BLOCK_CONTROL_REG.set(0x1000);
        Self::SPU_DMA_CHANNEL_CONTROL_REG.set(0x01000201);
        // TODO END

        // TODO: Start DMA4 at CPU Side (blocksize=10h, control=01000201h)

        // TODO: Wait until DMA4 finishes (at CPU side)

        let sample = SpuSample::new(self.current_address);

        sample
    }
}

pub struct SpuSample {
    start_address: u32,
}

impl SpuSample {
    const VOICE_START_ADDR: MemoryCell<u32> = MemoryCell::new(0x1F80_1C06);
    const VOICE_REPEAT_ADDR: MemoryCell<u32> = MemoryCell::new(0x1F80_1C0E);
    const VOICE_PITCH: MemoryCell<u16> = MemoryCell::new(0x1F80_1C04);
    const VOICE_VOLUME_LEFT: MemoryCell<u16> = MemoryCell::new(0x1F80_1C00);
    const VOICE_VOLUME_RIGHT: MemoryCell<u16> = MemoryCell::new(0x1F80_1C02);
    const KEY_ON: MemoryCell<u32> = MemoryCell::new(0x1F80_1D88);

    pub fn new(start_address: u32) -> Self {
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