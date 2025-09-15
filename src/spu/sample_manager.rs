use crate::{common::MemoryCell};

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
    const SPU_RAM_DATA_TRANSFER_ADDR: MemoryCell<u16> = MemoryCell::new(0x1F80_1DA6);
    const SPU_RAM_DATA_TRANSFER_FIFO: MemoryCell<u16> = MemoryCell::new(0x1F80_1DA8);
    const SPU_RAM_DATA_TRANSFER_CONTROL: MemoryCell<u16> = MemoryCell::new(0x1F80_1DAC);

    pub fn new() -> Self {
        SampleManager {
            next_id: 1,
            memory_blocks: [None; MAX_SAMPLES],
            loaded_samples: [None; MAX_SAMPLES],
            next_addr: SPU_RAM_START,
            block_count: 0,
        }
    }

    pub fn load(&mut self, audio_data: &[u8]) -> Result<Sample, &'static str> {
        if audio_data.is_empty() {
            return Err("Empty audio data");
        }

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
        Self::SPU_RAM_DATA_TRANSFER_ADDR.set(addr);
        Self::SPU_RAM_DATA_TRANSFER_CONTROL.set(0x0004);

        for chunk in audio_data.chunks(2) {
            if chunk.len() == 2 {
                let word = (chunk[1] as u16) << 8 | chunk[0] as u16;
                Self::SPU_RAM_DATA_TRANSFER_FIFO.set(word);
            } else if chunk.len() == 1 {
                let word = chunk[0] as u16;
                Self::SPU_RAM_DATA_TRANSFER_FIFO.set(word);
            }
        }

        Ok(())
    }

    pub fn deactivate(&mut self, sample: &Sample) {
        if let Some(loaded_sample) = self.loaded_samples
            .iter_mut()
            .filter_map(|s| s.as_mut())
            .find(|s| s.id == sample.id) {
            loaded_sample.in_use = false;
        }
    }
}
