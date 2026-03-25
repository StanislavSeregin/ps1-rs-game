use crate::common::MemoryCell;

const VOICE_BASE: usize = 0x1F80_1C00;
const VOICE_STRIDE: usize = 0x10;

const SPU_KEY_ON: MemoryCell<u32> = MemoryCell::new(0x1F80_1D88);
const SPU_KEY_OFF: MemoryCell<u32> = MemoryCell::new(0x1F80_1D8C);
const SPU_CONTROL: MemoryCell<u16> = MemoryCell::new(0x1F80_1DAA);
const SPU_MAIN_VOL_LEFT: MemoryCell<u16> = MemoryCell::new(0x1F80_1D80);
const SPU_MAIN_VOL_RIGHT: MemoryCell<u16> = MemoryCell::new(0x1F80_1D82);
const SPU_RAM_TRANSFER_ADDR: MemoryCell<u16> = MemoryCell::new(0x1F80_1DA6);
const SPU_RAM_TRANSFER_FIFO: MemoryCell<u16> = MemoryCell::new(0x1F80_1DA8);
const SPU_RAM_TRANSFER_CTRL: MemoryCell<u16> = MemoryCell::new(0x1F80_1DAC);

// ---------------------------------------------------------------------------
// Reverb registers
// ---------------------------------------------------------------------------

const REVERB_OUT_VOL_LEFT: MemoryCell<u16> = MemoryCell::new(0x1F80_1D84);
const REVERB_OUT_VOL_RIGHT: MemoryCell<u16> = MemoryCell::new(0x1F80_1D86);
const SPU_REVERB_ON: MemoryCell<u32> = MemoryCell::new(0x1F80_1D98);
const SPU_REVERB_BASE: MemoryCell<u16> = MemoryCell::new(0x1F80_1DA2);
const REVERB_CONFIG_BASE: usize = 0x1F80_1DC0;
const REVERB_IN_VOL_LEFT: MemoryCell<u16> = MemoryCell::new(0x1F80_1DFC);
const REVERB_IN_VOL_RIGHT: MemoryCell<u16> = MemoryCell::new(0x1F80_1DFE);

/// Hardware voice handle with runtime index (0..23).
///
/// Unlike the const-generic `Voice<const NUM: u8>` in `spu`,
/// this computes register addresses at runtime, enabling dynamic
/// voice allocation without match-arm dispatch.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VoiceHw(u8);

impl VoiceHw {
    pub const fn new(id: u8) -> Self {
        assert!(id < 24, "PS1 SPU has 24 voices (0..23)");
        Self(id)
    }

    pub const fn id(&self) -> u8 {
        self.0
    }

    const fn base(&self) -> usize {
        VOICE_BASE + self.0 as usize * VOICE_STRIDE
    }

    pub fn set_volume(&self, left: u16, right: u16) {
        MemoryCell::<u16>::new(self.base()).set(left);
        MemoryCell::<u16>::new(self.base() + 0x02).set(right);
    }

    pub fn set_pitch(&self, rate: u16) {
        MemoryCell::<u16>::new(self.base() + 0x04).set(rate);
    }

    pub fn set_sample_addr(&self, addr: u16) {
        MemoryCell::<u16>::new(self.base() + 0x06).set(addr);
    }

    pub fn set_adsr(&self, adsr: u32) {
        MemoryCell::<u32>::new(self.base() + 0x08).set(adsr);
    }

    pub fn set_repeat_addr(&self, addr: u16) {
        MemoryCell::<u16>::new(self.base() + 0x0E).set(addr);
    }

    pub fn key_on(&self) {
        SPU_KEY_ON.set(1u32 << self.0);
    }

    pub fn key_off(&self) {
        SPU_KEY_OFF.set(1u32 << self.0);
    }

    /// Configure voice registers without triggering key-on.
    ///
    /// Use this when batching multiple voice triggers into a single
    /// `key_on_mask` write to avoid the SPU latch race.
    pub fn prepare(&self, spu_addr: u16, pitch: u16, volume: u16, adsr: u32) {
        self.set_volume(volume, volume);
        self.set_sample_addr(spu_addr);
        self.set_repeat_addr(spu_addr);
        self.set_pitch(pitch);
        self.set_adsr(adsr);
    }

    /// Configure and trigger a sample in one call.
    pub fn trigger(&self, spu_addr: u16, pitch: u16, volume: u16, adsr: u32) {
        self.prepare(spu_addr, pitch, volume, adsr);
        self.key_on();
    }
}

pub fn key_on_mask(mask: u32) {
    SPU_KEY_ON.set(mask);
}

pub fn key_off_mask(mask: u32) {
    SPU_KEY_OFF.set(mask);
}

pub fn key_off_all() {
    SPU_KEY_OFF.set(0x00FF_FFFF);
}

pub fn set_master_volume(left: u16, right: u16) {
    SPU_MAIN_VOL_LEFT.set(left);
    SPU_MAIN_VOL_RIGHT.set(right);
}

/// Write raw audio data into SPU RAM at the given address.
pub fn transfer_to_spu_ram(addr: u16, data: &[u8]) {
    SPU_RAM_TRANSFER_ADDR.set(addr);
    SPU_RAM_TRANSFER_CTRL.set(0x0004);
    for chunk in data.chunks(2) {
        let word = if chunk.len() == 2 {
            (chunk[1] as u16) << 8 | chunk[0] as u16
        } else {
            chunk[0] as u16
        };
        SPU_RAM_TRANSFER_FIFO.set(word);
    }
}

/// Power-on SPU and set master volume to max.
pub fn init_spu_hardware() {
    SPU_CONTROL.set(0xC000);
    SPU_CONTROL.set(0xC001);
    set_master_volume(0x3FFF, 0x3FFF);
    REVERB_OUT_VOL_LEFT.set(0);
    REVERB_OUT_VOL_RIGHT.set(0);
}

// ---------------------------------------------------------------------------
// Reverb hardware access
// ---------------------------------------------------------------------------

/// Set the reverb work area start address (N×8 byte addressing).
/// The reverb buffer extends from this address to the end of SPU RAM.
pub fn set_reverb_base(addr: u16) {
    SPU_REVERB_BASE.set(addr);
}

/// Write all 30 reverb configuration registers (`$1F801DC0`–`$1F801DFA`).
pub fn write_reverb_config(regs: &[u16; 30]) {
    for (i, &val) in regs.iter().enumerate() {
        MemoryCell::<u16>::new(REVERB_CONFIG_BASE + i * 2).set(val);
    }
}

/// Set reverb input volume (signed 16-bit, scale N / 0x8000).
pub fn set_reverb_volume_in(left: u16, right: u16) {
    REVERB_IN_VOL_LEFT.set(left);
    REVERB_IN_VOL_RIGHT.set(right);
}

/// Set reverb output volume (signed 16-bit, scale N / 0x8000).
pub fn set_reverb_volume_out(left: u16, right: u16) {
    REVERB_OUT_VOL_LEFT.set(left);
    REVERB_OUT_VOL_RIGHT.set(right);
}

/// Set which of the 24 voices feed into the reverb unit (bits 0–23).
pub fn set_reverb_on_mask(mask: u32) {
    SPU_REVERB_ON.set(mask & 0x00FF_FFFF);
}

/// Enable reverb RAM writes (SPUCNT bit 7).
pub fn enable_reverb_master() {
    let val = SPU_CONTROL.get();
    SPU_CONTROL.set(val | 0x0080);
}

/// Disable reverb RAM writes (SPUCNT bit 7).
/// Note: output volume must also be set to 0 to fully mute reverb.
pub fn disable_reverb_master() {
    let val = SPU_CONTROL.get();
    SPU_CONTROL.set(val & !0x0080);
}

/// Zero-fill a region of SPU RAM via the manual-write FIFO.
/// `start` is in N×8 byte addressing; `halfwords` is the number of
/// 16-bit words to clear.
pub fn clear_spu_ram(start: u16, halfwords: u32) {
    SPU_RAM_TRANSFER_ADDR.set(start);
    SPU_RAM_TRANSFER_CTRL.set(0x0004);
    for _ in 0..halfwords {
        SPU_RAM_TRANSFER_FIFO.set(0);
    }
}

// ---------------------------------------------------------------------------
// Hardware timer — Root Counter 1 (HBlank)
//
// Counts horizontal blanking pulses (~15734 Hz NTSC, ~15625 Hz PAL).
// Used as a VBlank-independent time source for sequencer timing.
// The 16-bit counter wraps every ~4.2 seconds, which is safe as long as
// we read it at least once per yield (~16 ms).
// ---------------------------------------------------------------------------

const RCNT1_VALUE: MemoryCell<u16> = MemoryCell::new(0x1F80_1110);
const RCNT1_MODE: MemoryCell<u16> = MemoryCell::new(0x1F80_1114);

/// Approximate HBlank rate in Hz (NTSC ≈ 15734, PAL ≈ 15625).
/// Using a rounded average that gives < 1% error on either standard.
pub const HBLANK_RATE: u32 = 15700;

/// Start Root Counter 1 in free-running HBlank mode.
pub fn init_hblank_timer() {
    // Bit 8 = 1: clock source = HBlank
    // All other bits 0: free run, no IRQ, wrap at 0xFFFF
    RCNT1_MODE.set(0x0100);
}

/// Read the current 16-bit HBlank counter value.
pub fn read_hblank_counter() -> u16 {
    RCNT1_VALUE.get()
}
