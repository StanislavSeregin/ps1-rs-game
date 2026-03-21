use super::hw;

const SPU_RAM_START: u16 = 0x1000;

/// Typed sample index used to reference loaded samples.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SampleId(pub u8);

/// Reference to audio data already loaded into SPU RAM.
#[derive(Clone, Copy)]
pub struct SampleRef {
    pub spu_addr: u16,
    pub size: u16,
}

/// Manages sample storage in SPU RAM.
///
/// Samples are loaded sequentially into SPU RAM. There is no compaction or
/// block reuse in this version -- call [`reset`](SampleBank::reset) to wipe
/// everything (e.g. when switching [`SoundProject`](super::music::SoundProject)s).
pub struct SampleBank<const MAX: usize = 32> {
    slots: [Option<SampleRef>; MAX],
    next_addr: u16,
}

impl<const MAX: usize> SampleBank<MAX> {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX],
            next_addr: SPU_RAM_START,
        }
    }

    /// Load raw audio bytes into SPU RAM and register them under `id`.
    ///
    /// Returns a reference to the loaded sample, or an error if the slot
    /// is out of range or there is no space left.
    pub fn load(&mut self, id: SampleId, data: &[u8]) -> Result<&SampleRef, &'static str> {
        let slot = id.0 as usize;
        if slot >= MAX {
            return Err("sample id out of range");
        }
        if data.is_empty() {
            return Err("empty audio data");
        }

        let size = data.len() as u16;
        let addr = self.next_addr;
        self.next_addr = self.next_addr.wrapping_add(size);

        hw::transfer_to_spu_ram(addr, data);

        self.slots[slot] = Some(SampleRef {
            spu_addr: addr,
            size,
        });

        Ok(self.slots[slot].as_ref().unwrap())
    }

    pub fn get(&self, id: SampleId) -> Option<&SampleRef> {
        self.slots.get(id.0 as usize).and_then(|s| s.as_ref())
    }

    /// Wipe all loaded samples and reset the RAM cursor.
    /// Use this before loading a new [`SoundProject`](super::music::SoundProject).
    pub fn reset(&mut self) {
        self.slots = [None; MAX];
        self.next_addr = SPU_RAM_START;
    }
}
