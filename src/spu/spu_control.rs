#[derive(Clone, PartialEq, Debug)]
pub enum NoiseFrequencyShift {
    Low0 = 0,
    Low1 = 1,
    Low2 = 2,
    Low3 = 3,
    Low4 = 4,
    Low5 = 5,
    Low6 = 6,
    Low7 = 7,
    Medium8 = 8,
    Medium9 = 9,
    Medium10 = 10,
    Medium11 = 11,
    Medium12 = 12,
    Medium13 = 13,
    Medium14 = 14,
    High15 = 15,
}

#[derive(Clone, PartialEq, Debug)]
pub enum NoiseFrequencyStep {
    Step4 = 0,
    Step5 = 1,
    Step6 = 2,
    Step7 = 3,
}

#[derive(Clone, PartialEq, Debug)]
pub enum SpuRamTransferMode {
    Stop = 0,
    ManualWrite = 1,
    DMAWrite = 2,
    DMARead = 3,
}

pub struct SpuControl {
    pub spu_enable: bool,
    pub mute_spu: bool,
    pub noise_freq_shift: NoiseFrequencyShift,
    pub noise_freq_step: NoiseFrequencyStep,
    pub reverb_master_enable: bool,
    pub irq9_enable: bool,
    pub sound_ram_transfer_mode: SpuRamTransferMode,
    pub external_audio_reverb: bool,
    pub cd_audio_reverb: bool,
    pub external_audio_enable: bool,
    pub cd_audio_enable: bool,
}

impl SpuControl {
    pub fn new() -> Self {
        Self {
            spu_enable: false,
            mute_spu: false,
            noise_freq_shift: NoiseFrequencyShift::Low0,
            noise_freq_step: NoiseFrequencyStep::Step4,
            reverb_master_enable: false,
            irq9_enable: false,
            sound_ram_transfer_mode: SpuRamTransferMode::Stop,
            external_audio_reverb: false,
            cd_audio_reverb: false,
            external_audio_enable: false,
            cd_audio_enable: false,
        }
    }

    pub fn to_u16(&self) -> u16 {
        let mut value = 0;
        value |= (self.spu_enable as u16) << 15;
        value |= (self.mute_spu as u16) << 14;
        value |= (self.noise_freq_shift.clone() as u16) << 10;
        value |= (self.noise_freq_step.clone() as u16) << 8;
        value |= (self.reverb_master_enable as u16) << 7;
        value |= (self.irq9_enable as u16) << 6;
        value |= (self.sound_ram_transfer_mode.clone() as u16) << 4;
        value |= (self.external_audio_reverb as u16) << 3;
        value |= (self.cd_audio_reverb as u16) << 2;
        value |= (self.external_audio_enable as u16) << 1;
        value |= (self.cd_audio_enable as u16) << 0;
        value
    }
}
