use crate::{common::MemoryCell, spu::Voice};

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
    pub fn bind_to_voice<const NUM: u8>(self, sample_rate: u16, volume: u16) -> Voice<NUM> {
        Voice::<NUM>::new(self.spu_addr, sample_rate, volume)
    }
}

#[derive(Clone, Copy)]
struct MemoryBlock {
    start_addr: u16,
    size: u16,
    sample_id: Option<u16>,
    is_free: bool,
}

pub struct Sampler {
    next_id: u16,
    memory_blocks: [Option<MemoryBlock>; MAX_SAMPLES],
    loaded_samples: [Option<Sample>; MAX_SAMPLES],
    next_addr: u16,
    block_count: usize,
}

impl Sampler {
    const SPU_RAM_DATA_TRANSFER_ADDR: MemoryCell<u16> = MemoryCell::new(0x1F80_1DA6);
    const SPU_RAM_DATA_TRANSFER_FIFO: MemoryCell<u16> = MemoryCell::new(0x1F80_1DA8);
    const SPU_RAM_DATA_TRANSFER_CONTROL: MemoryCell<u16> = MemoryCell::new(0x1F80_1DAC);

    pub(super) const fn new() -> Self {
        Sampler {
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
            self.write_sample_to_spu_ram(addr, audio_data);
            let sample = Sample {
                id,
                spu_addr: addr,
                size: sample_size,
                in_use: true,
            };

            if let Some(_) = self.find_reusable_blocks(sample_size) {
                if addr == self.find_reusable_blocks(sample_size).unwrap() {
                    self.merge_blocks_for_reuse(addr, id, sample_size);
                } else {
                    let reusing_block = self.memory_blocks
                        .iter_mut()
                        .filter_map(|b| b.as_mut())
                        .find(|b| b.start_addr == addr);

                    if let Some(existing_block) = reusing_block {
                        existing_block.sample_id = Some(id);
                        existing_block.size = sample_size;
                        existing_block.is_free = false;
                    }
                }
            } else {
                let block = MemoryBlock {
                    start_addr: addr,
                    size: sample_size,
                    sample_id: Some(id),
                    is_free: false,
                };

                if let Some(free_block_slot) = self.memory_blocks.iter_mut().find(|b| b.is_none()) {
                    *free_block_slot = Some(block);
                    self.block_count += 1;
                } else {
                    return Err("No free block slots");
                }
            }

            if let Some(free_slot) = self.loaded_samples.iter_mut().find(|s| s.is_none()) {
                *free_slot = Some(sample);
            } else {
                return Err("No free sample slots");
            }

            Ok(sample)
        } else {
            Err("Not enough SPU RAM")
        }
    }

    fn find_reusable_blocks(&self, required_size: u16) -> Option<u16> {
        let mut inactive_blocks: [(usize, &MemoryBlock); MAX_SAMPLES] = [
            (0, &MemoryBlock {
                start_addr: 0,
                size: 0,
                sample_id: None,
                is_free: true,
            });
            MAX_SAMPLES
        ];

        let mut inactive_count = 0;
        for (i, block_opt) in self.memory_blocks.iter().enumerate() {
            if let Some(block) = block_opt {
                if let Some(sample_id) = block.sample_id {
                    if let Some(sample) = self.loaded_samples
                        .iter()
                        .filter_map(|s| s.as_ref())
                        .find(|s| s.id == sample_id) {
                        if !sample.in_use && inactive_count < MAX_SAMPLES {
                            inactive_blocks[inactive_count] = (i, block);
                            inactive_count += 1;
                        }
                    }
                }
            }
        }

        for i in 0..inactive_count {
            for j in 0..(inactive_count - 1 - i) {
                if inactive_blocks[j].1.start_addr > inactive_blocks[j + 1].1.start_addr {
                    inactive_blocks.swap(j, j + 1);
                }
            }
        }

        for start_idx in 0..inactive_count {
            let mut total_size = inactive_blocks[start_idx].1.size;
            let start_addr = inactive_blocks[start_idx].1.start_addr;
            let mut current_end = start_addr + total_size;

            for next_idx in (start_idx + 1)..inactive_count {
                let next_block = inactive_blocks[next_idx].1;
                if next_block.start_addr == current_end {
                    total_size += next_block.size;
                    current_end = next_block.start_addr + next_block.size;
                    if total_size >= required_size {
                        return Some(start_addr);
                    }
                } else {
                    break;
                }
            }

            if total_size >= required_size {
                return Some(start_addr);
            }
        }

        None
    }

    fn merge_blocks_for_reuse(&mut self, start_addr: u16, new_sample_id: u16, new_sample_size: u16) {
        let mut blocks_to_remove = [0usize; MAX_SAMPLES];
        let mut remove_count = 0;
        let mut first_block_index = None;
        for (i, block_opt) in self.memory_blocks.iter().enumerate() {
            if let Some(block) = block_opt {
                if block.start_addr >= start_addr {
                    if let Some(sample_id) = block.sample_id {
                        if let Some(sample) = self.loaded_samples
                            .iter()
                            .filter_map(|s| s.as_ref())
                            .find(|s| s.id == sample_id) {
                            if !sample.in_use && block.start_addr < start_addr + new_sample_size {
                                if first_block_index.is_none() && block.start_addr == start_addr {
                                    first_block_index = Some(i);
                                } else if remove_count < MAX_SAMPLES {
                                    blocks_to_remove[remove_count] = i;
                                    remove_count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        for i in 0..remove_count {
            let index = blocks_to_remove[i];
            self.memory_blocks[index] = None;
            self.block_count -= 1;
        }

        if let Some(index) = first_block_index {
            if let Some(first_block) = &mut self.memory_blocks[index] {
                first_block.sample_id = Some(new_sample_id);
                first_block.size = new_sample_size;
                first_block.is_free = false;
            }
        }
    }

    fn find_free_space(&mut self, required_size: u16) -> Option<u16> {
        if let Some(addr) = self.find_reusable_blocks(required_size) {
            return Some(addr);
        }

        let end_addr = (SPU_RAM_START as u32).saturating_add(SPU_RAM_SIZE);
        if (self.next_addr as u32).saturating_add(required_size as u32) <= end_addr {
            let addr = self.next_addr;
            self.next_addr = self.next_addr.saturating_add(required_size);
            Some(addr)
        } else {
            None
        }
    }

    fn write_sample_to_spu_ram(&self, addr: u16, audio_data: &[u8]) {
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
    }

    pub fn deactivate(&mut self, sample: Sample) {
        if let Some(loaded_sample) = self.loaded_samples
            .iter_mut()
            .filter_map(|s| s.as_mut())
            .find(|s| s.id == sample.id) {
            loaded_sample.in_use = false;
        }
    }
}
