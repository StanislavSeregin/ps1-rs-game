use super::spu_control::SpuRamTransferMode;

pub struct SpuStatus {
    pub unknown15: bool,
    pub unknown14: bool,
    pub unknown13: bool,
    pub unknown12: bool,
    pub writing_to_first_second_half_of_capture_buffers: bool,
    pub data_transfer_busy_flag: bool,
    pub data_transfer_dma_read_request: bool,
    pub data_transfer_dma_write_request: bool,
    pub data_transfer_dma_read_write_request: bool,
    pub irq9_flag_interrupt_request: bool,
    pub sound_ram_transfer_mode: SpuRamTransferMode,
    pub external_audio_reverb: bool,
    pub cd_audio_reverb: bool,
    pub external_audio_enable: bool,
    pub cd_audio_enable: bool,
}

impl SpuStatus {
    pub fn from_u16(value: &u16) -> Self {
        Self {
            unknown15: (value >> 15) & 1 == 1,
            unknown14: (value >> 14) & 1 == 1,
            unknown13: (value >> 13) & 1 == 1,
            unknown12: (value >> 12) & 1 == 1,
            writing_to_first_second_half_of_capture_buffers: (value >> 11) & 1 == 1,
            data_transfer_busy_flag: (value >> 10) & 1 == 1,
            data_transfer_dma_read_request: (value >> 9) & 1 == 1,
            data_transfer_dma_write_request: (value >> 8) & 1 == 1,
            data_transfer_dma_read_write_request: (value >> 7) & 1 == 1,
            irq9_flag_interrupt_request: (value >> 6) & 1 == 1,
            sound_ram_transfer_mode: Self::get_sound_ram_transfer_mode(value),
            external_audio_reverb: (value >> 3) & 1 == 1,
            cd_audio_reverb: (value >> 2) & 1 == 1,
            external_audio_enable: (value >> 1) & 1 == 1,
            cd_audio_enable: (value >> 0) & 1 == 1,
        }
    }

    pub fn get_sound_ram_transfer_mode(value: &u16) -> SpuRamTransferMode {
        match (value >> 4) & 0b11 {
            0 => SpuRamTransferMode::Stop,
            1 => SpuRamTransferMode::ManualWrite,
            2 => SpuRamTransferMode::DMAWrite,
            3 => SpuRamTransferMode::DMARead,
            _ => SpuRamTransferMode::Stop,
        }
    }
}