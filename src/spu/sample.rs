use super::hw;

const SPU_RAM_START: u16 = 0x1000;

const ADPCM_BLOCK_ALIGN: u16 = 16;

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
    /// Upper address bound (exclusive). Set to the reverb work area start
    /// when reverb is enabled, preventing sample data from colliding with it.
    addr_limit: u16,
}

impl<const MAX: usize> SampleBank<MAX> {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX],
            next_addr: SPU_RAM_START,
            addr_limit: u16::MAX,
        }
    }

    /// Set the upper address bound to the reverb work area start.
    /// Pass `u16::MAX` to remove the limit (e.g. when reverb is disabled).
    pub fn set_addr_limit(&mut self, limit: u16) {
        self.addr_limit = limit;
    }

    /// Load raw audio bytes into SPU RAM and register them under `id`.
    ///
    /// Returns a reference to the loaded sample, or an error if the slot
    /// is out of range or there is no space left.
    pub fn load(&mut self, id: SampleId, data: &[u8]) -> Result<&SampleRef, &'static str> {
        let slot = id.0 as usize;
        if slot >= MAX {
            return Err("bad sample id");
        }
        if data.is_empty() {
            return Err("empty sample");
        }

        let size = data.len() as u16;
        let addr = align_up_to(self.next_addr, ADPCM_BLOCK_ALIGN).ok_or("spu ram full")?;
        let end = addr
            .checked_add(size)
            .ok_or("spu ram full")?;
        if end > self.addr_limit {
            return Err("spu ram full");
        }
        self.next_addr = end;

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
        // addr_limit is intentionally preserved across resets
    }
}

fn align_up_to(addr: u16, align: u16) -> Option<u16> {
    let mask = (align - 1) as u32;
    let aligned = ((addr as u32) + mask) & !mask;
    if aligned > u16::MAX as u32 {
        None
    } else {
        Some(aligned as u16)
    }
}
