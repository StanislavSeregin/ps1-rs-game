use crate::{common::MemoryCell};

const SPU_BASE: usize = 0x1F801C00;
const SPU_VOICE_LEFT_VOL: usize = SPU_BASE + 0x00;
const SPU_VOICE_RIGHT_VOL: usize = SPU_BASE + 0x02;
const SPU_VOICE_SAMPLE_RATE: usize = SPU_BASE + 0x04;
const SPU_VOICE_START_ADDR: usize = SPU_BASE + 0x06;
const SPU_VOICE_ADSR: usize = SPU_BASE + 0x08;
const SPU_VOICE_REPEAT_ADDR: usize = SPU_BASE + 0x0E;

const MAX_SAMPLES: usize = 64;
const SPU_RAM_START: u16 = 0x1000;
const SPU_RAM_SIZE: u32 = 508 * 1024;

#[derive(Clone, Copy)]
pub struct Sample {
    pub id: u16,
    pub spu_addr: u16,
    pub size: u16,
    pub in_use: bool,
}

impl Sample {
    pub fn deactivate(&mut self) {
        self.in_use = false;
    }

    pub fn is_active(&self) -> bool {
        self.in_use
    }
}

#[derive(Clone, Copy)]
struct MemoryBlock {
    start_addr: u16,
    size: u16,
    sample_id: Option<u16>,
    is_free: bool,
}

pub struct SampleManager {
    next_id: u16,
    memory_blocks: [Option<MemoryBlock>; MAX_SAMPLES],
    loaded_samples: [Option<Sample>; MAX_SAMPLES],
    next_addr: u16,
    block_count: usize,
}

impl SampleManager {
    pub fn new() -> Self {
        SampleManager {
            next_id: 1,
            memory_blocks: [None; MAX_SAMPLES],
            loaded_samples: [None; MAX_SAMPLES],
            next_addr: SPU_RAM_START,
            block_count: 0,
        }
    }

    pub fn load_sample(&mut self, vag_data: &[u8]) -> Result<Sample, &'static str> {
        if vag_data.len() < 48 {
            return Err("Invalid VAG data");
        }

        let audio_data = &vag_data[48..];
        let sample_size = audio_data.len() as u16;

        if let Some(addr) = self.find_free_space(sample_size) {
            let id = self.next_id;
            self.next_id += 1;

            self.write_sample_to_spu_ram(addr, audio_data)?;

            let sample = Sample {
                id,
                spu_addr: addr,
                size: sample_size,
                in_use: true,
            };

            let block = MemoryBlock {
                start_addr: addr,
                size: sample_size,
                sample_id: Some(id),
                is_free: false,
            };

            if let Some(free_slot) = self.loaded_samples.iter_mut().find(|s| s.is_none()) {
                *free_slot = Some(sample);
            } else {
                return Err("No free sample slots");
            }

            if let Some(free_block_slot) = self.memory_blocks.iter_mut().find(|b| b.is_none()) {
                *free_block_slot = Some(block);
                self.block_count += 1;
            } else {
                return Err("No free block slots");
            }

            Ok(sample)
        } else {
            Err("Not enough SPU RAM")
        }
    }

    fn find_free_space(&mut self, required_size: u16) -> Option<u16> {
        let end_addr = (SPU_RAM_START as u32).saturating_add(SPU_RAM_SIZE);
        if (self.next_addr as u32).saturating_add(required_size as u32) <= end_addr {
            let addr = self.next_addr;
            self.next_addr = self.next_addr.saturating_add(required_size);
            Some(addr)
        } else {
            None
        }
    }

    fn write_sample_to_spu_ram(&self, addr: u16, audio_data: &[u8]) -> Result<(), &'static str> {
        SPU::SPU_RAM_DATA_TRANSFER_ADDR.set(addr);
        SPU::SPU_RAM_DATA_TRANSFER_CONTROL.set(0x0004);

        for chunk in audio_data.chunks(2) {
            if chunk.len() == 2 {
                let word = (chunk[1] as u16) << 8 | chunk[0] as u16;
                SPU::SPU_RAM_DATA_TRANSFER_FIFO.set(word);
            } else if chunk.len() == 1 {
                let word = chunk[0] as u16;
                SPU::SPU_RAM_DATA_TRANSFER_FIFO.set(word);
            }

            for _ in 0..10 {/* Nothing */}
        }

        Ok(())
    }

    pub fn get_sample(&self, id: u16) -> Option<&Sample> {
        self.loaded_samples
            .iter()
            .filter_map(|s| s.as_ref())
            .find(|s| s.id == id)
    }

    pub fn get_sample_mut(&mut self, id: u16) -> Option<&mut Sample> {
        self.loaded_samples
            .iter_mut()
            .filter_map(|s| s.as_mut())
            .find(|s| s.id == id)
    }
}

pub struct SPU {
    pub sample_manager: SampleManager,
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
            sample_manager: SampleManager::new(),
        }
    }

    pub fn load_sample(&mut self, vag_data: &[u8]) -> Result<Sample, &'static str> {
        self.sample_manager.load_sample(vag_data)
    }

    pub fn get_sample(&self, id: u16) -> Option<&Sample> {
        self.sample_manager.get_sample(id)
    }

    pub fn get_sample_mut(&mut self, id: u16) -> Option<&mut Sample> {
        self.sample_manager.get_sample_mut(id)
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
        let current = Self::KEY_ON.get();
        let key_on_mask = current | (1u32 << NUM);
        Self::KEY_ON.set(key_on_mask);

        self
    }
}
