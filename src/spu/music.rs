use super::sample::SampleId;
use super::voice::VoiceLayout;

/// SPU pitch register value.
///
/// The PS1 SPU pitch register maps `0x1000` to 44100 Hz playback rate.
/// The table below is tuned to A4 = 440 Hz with equal temperament.
/// Values are pre-computed as `round(freq / 44100 * 4096)`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Pitch(pub u16);

impl Pitch {
    pub const REST: Self = Self(0);

    // Octave 2
    pub const C2:  Self = Self(0x0060); // 65.41 Hz
    pub const CS2: Self = Self(0x0066); // 69.30 Hz
    pub const D2:  Self = Self(0x006C); // 73.42 Hz
    pub const DS2: Self = Self(0x0073); // 77.78 Hz
    pub const E2:  Self = Self(0x007A); // 82.41 Hz
    pub const F2:  Self = Self(0x0081); // 87.31 Hz
    pub const FS2: Self = Self(0x0089); // 92.50 Hz
    pub const G2:  Self = Self(0x0091); // 98.00 Hz
    pub const GS2: Self = Self(0x009A); // 103.83 Hz
    pub const A2:  Self = Self(0x00A3); // 110.00 Hz
    pub const AS2: Self = Self(0x00AD); // 116.54 Hz
    pub const B2:  Self = Self(0x00B7); // 123.47 Hz

    // Octave 3
    pub const C3:  Self = Self(0x00C1); // 130.81 Hz
    pub const CS3: Self = Self(0x00CD); // 138.59 Hz
    pub const D3:  Self = Self(0x00D9); // 146.83 Hz
    pub const DS3: Self = Self(0x00E6); // 155.56 Hz
    pub const E3:  Self = Self(0x00F4); // 164.81 Hz
    pub const F3:  Self = Self(0x0103); // 174.61 Hz
    pub const FS3: Self = Self(0x0112); // 185.00 Hz
    pub const G3:  Self = Self(0x0122); // 196.00 Hz
    pub const GS3: Self = Self(0x0134); // 207.65 Hz
    pub const A3:  Self = Self(0x0146); // 220.00 Hz
    pub const AS3: Self = Self(0x015A); // 233.08 Hz
    pub const B3:  Self = Self(0x016F); // 246.94 Hz

    // Octave 4 (middle octave, A4 = 440 Hz)
    pub const C4:  Self = Self(0x0183); // 261.63 Hz
    pub const CS4: Self = Self(0x019A); // 277.18 Hz
    pub const D4:  Self = Self(0x01B2); // 293.66 Hz
    pub const DS4: Self = Self(0x01CC); // 311.13 Hz
    pub const E4:  Self = Self(0x01E8); // 329.63 Hz
    pub const F4:  Self = Self(0x0206); // 349.23 Hz
    pub const FS4: Self = Self(0x0225); // 369.99 Hz
    pub const G4:  Self = Self(0x0247); // 392.00 Hz
    pub const GS4: Self = Self(0x026B); // 415.30 Hz
    pub const A4:  Self = Self(0x028D); // 440.00 Hz
    pub const AS4: Self = Self(0x02B4); // 466.16 Hz
    pub const B4:  Self = Self(0x02DE); // 493.88 Hz

    // Octave 5
    pub const C5:  Self = Self(0x0306); // 523.25 Hz
    pub const CS5: Self = Self(0x0334); // 554.37 Hz
    pub const D5:  Self = Self(0x0364); // 587.33 Hz
    pub const DS5: Self = Self(0x0399); // 622.25 Hz
    pub const E5:  Self = Self(0x03D1); // 659.26 Hz
    pub const F5:  Self = Self(0x040C); // 698.46 Hz
    pub const FS5: Self = Self(0x044A); // 739.99 Hz
    pub const G5:  Self = Self(0x048D); // 783.99 Hz
    pub const GS5: Self = Self(0x04D5); // 830.61 Hz
    pub const A5:  Self = Self(0x051B); // 880.00 Hz
    pub const AS5: Self = Self(0x0568); // 932.33 Hz
    pub const B5:  Self = Self(0x05BB); // 987.77 Hz

    // Octave 6
    pub const C6:  Self = Self(0x060C); // 1046.50 Hz
    pub const CS6: Self = Self(0x0668); // 1108.73 Hz
    pub const D6:  Self = Self(0x06C9); // 1174.66 Hz
    pub const DS6: Self = Self(0x0732); // 1244.51 Hz
    pub const E6:  Self = Self(0x07A2); // 1318.51 Hz
    pub const F6:  Self = Self(0x0818); // 1396.91 Hz
    pub const FS6: Self = Self(0x0894); // 1479.98 Hz
    pub const G6:  Self = Self(0x091B); // 1567.98 Hz
    pub const GS6: Self = Self(0x09AB); // 1661.22 Hz
    pub const A6:  Self = Self(0x0A36); // 1760.00 Hz
    pub const AS6: Self = Self(0x0AD0); // 1864.66 Hz
    pub const B6:  Self = Self(0x0B76); // 1975.53 Hz

    // Octave 7
    pub const C7:  Self = Self(0x0C19); // 2093.00 Hz
    pub const CS7: Self = Self(0x0CD0); // 2217.46 Hz
    pub const D7:  Self = Self(0x0D92); // 2349.32 Hz
    pub const DS7: Self = Self(0x0E64); // 2489.02 Hz
    pub const E7:  Self = Self(0x0F44); // 2637.02 Hz
    pub const F7:  Self = Self(0x1030); // 2793.83 Hz
    pub const FS7: Self = Self(0x1128); // 2959.96 Hz
    pub const G7:  Self = Self(0x1236); // 3135.96 Hz
    pub const GS7: Self = Self(0x1356); // 3322.44 Hz
    pub const A7:  Self = Self(0x146C); // 3520.00 Hz
    pub const AS7: Self = Self(0x15A0); // 3729.31 Hz
    pub const B7:  Self = Self(0x16EC); // 3951.07 Hz
}

/// Volume level for the SPU (0..0x3FFF).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Volume(pub u16);

impl Volume {
    pub const MAX: Self = Self(0x3FFF);
    pub const HALF: Self = Self(0x1FFF);
    pub const QUARTER: Self = Self(0x0FFF);
    pub const OFF: Self = Self(0);
}

/// Stereo panning position.
///
/// `0` = centre, negative = left, positive = right.
/// The full range is `−64` (hard left) to `+64` (hard right).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Pan(pub i8);

impl Pan {
    pub const CENTER: Self = Self(0);
    pub const LEFT: Self = Self(-64);
    pub const RIGHT: Self = Self(64);

    /// Derive (left, right) volume pair from a base volume.
    pub const fn apply(self, vol: u16) -> (u16, u16) {
        let p = self.0 as i16;
        if p <= 0 {
            // left-biased: right side attenuated
            let r = vol as u32 * (64 + p) as u32 / 64;
            (vol, r as u16)
        } else {
            // right-biased: left side attenuated
            let l = vol as u32 * (64 - p) as u32 / 64;
            (l as u16, vol)
        }
    }
}

/// Extensible effect slot.
///
/// Starts with only `None`; future variants (portamento, vibrato, etc.)
/// can be added without breaking existing patterns.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    None,
}

// ---------------------------------------------------------------------------
// ADSR envelope
//
// SPU register format (32-bit, two halfwords at voice base + 0x08):
//
//   Lower halfword: [Am:1][Ash:5][As:2][Dsh:4][Sl:4]
//   Upper halfword: [_:1][Sm:1][Sd:1][Ssh:5][Ss:2][Rm:1][Rsh:5]
//
// Packed u32 bit positions:
//    0- 3  Sustain Level          4- 7  Decay Shift
//    8- 9  Attack Step           10-14  Attack Shift
//   15     Attack Mode           16-20  Release Shift
//   21     Release Mode          22-23  Sustain Step
//   24-28  Sustain Shift         29     Sustain Direction
//   30     Sustain Mode
// ---------------------------------------------------------------------------

/// Envelope curve shape.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AdsrMode {
    Lin = 0,
    Exp = 1,
}

/// Sustain slope direction.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AdsrDir {
    Increase = 0,
    Decrease = 1,
}

/// ADSR envelope packed into the PS1 SPU register format.
///
/// Construct with a const builder chain:
///
/// ```ignore
/// use AdsrMode::*;
/// use AdsrDir::*;
///
/// const BELL: Adsr = Adsr::new()
///     .decay(0x0A)
///     .sustain_level(0x0A)
///     .sustain(Exp, Decrease, 0x0F)
///     .release(Exp, 0x08);
/// ```
///
/// Omitted phases default to zero (instant / off).
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Adsr(u32);

impl Adsr {
    pub const DEFAULT: Self = Self(0x80FF_8000);

    /// All zeros: instant attack, no decay, sustain level 0, instant release.
    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Set attack curve and speed.
    ///
    /// `shift`: 0-31 — higher is slower.
    pub const fn attack(self, mode: AdsrMode, shift: u8) -> Self {
        self.attack_step(mode, shift, 0)
    }

    /// Set attack with fine-grained step (0-3, rarely needed).
    pub const fn attack_step(self, mode: AdsrMode, shift: u8, step: u8) -> Self {
        let cleared = self.0 & !0xFF00;
        let am  = (mode as u32) << 15;
        let ash = ((shift as u32) & 0x1F) << 10;
        let a_s = ((step as u32) & 0x03) << 8;
        Self(cleared | am | ash | a_s)
    }

    /// Set decay speed (always exponential decrease toward sustain level).
    ///
    /// `shift`: 0-15 — higher is slower.
    pub const fn decay(self, shift: u8) -> Self {
        let cleared = self.0 & !0x00F0;
        Self(cleared | (((shift as u32) & 0x0F) << 4))
    }

    /// Set the sustain level the decay phase targets.
    ///
    /// `level`: 0-15 (0 ≈ 6%, 7 ≈ 50%, 10 ≈ 69%, 15 = 100%).
    pub const fn sustain_level(self, level: u8) -> Self {
        let cleared = self.0 & !0x000F;
        Self(cleared | ((level as u32) & 0x0F))
    }

    /// Set the sustain phase curve, direction, and speed.
    ///
    /// `shift`: 0-31 — higher is slower.
    pub const fn sustain(self, mode: AdsrMode, dir: AdsrDir, shift: u8) -> Self {
        self.sustain_step(mode, dir, shift, 0)
    }

    /// Set sustain with fine-grained step (0-3, rarely needed).
    pub const fn sustain_step(self, mode: AdsrMode, dir: AdsrDir, shift: u8, step: u8) -> Self {
        let cleared = self.0 & !0x7FC0_0000;
        let sm  = ((mode as u32) & 1) << 30;
        let sd  = ((dir as u32) & 1) << 29;
        let ssh = ((shift as u32) & 0x1F) << 24;
        let ss  = ((step as u32) & 0x03) << 22;
        Self(cleared | sm | sd | ssh | ss)
    }

    /// Set release curve and speed.
    ///
    /// `shift`: 0-31 — higher is slower.
    pub const fn release(self, mode: AdsrMode, shift: u8) -> Self {
        let cleared = self.0 & !0x003F_0000;
        let rm  = ((mode as u32) & 1) << 21;
        let rsh = ((shift as u32) & 0x1F) << 16;
        Self(cleared | rm | rsh)
    }
}

/// One cell in a pattern grid.
///
/// Each field is optional so that a cell can express partial updates
/// (e.g. change volume without re-triggering the note).
#[derive(Clone, Copy)]
pub struct Cell {
    pub pitch: Option<Pitch>,
    pub sample: Option<SampleId>,
    pub volume: Option<Volume>,
    pub effect: Effect,
    pub pan: Option<Pan>,
    pub adsr: Option<Adsr>,
}

impl Cell {
    pub const EMPTY: Self = Self {
        pitch: None,
        sample: None,
        volume: None,
        effect: Effect::None,
        pan: None,
        adsr: None,
    };

    /// Note-on with default max volume.
    pub const fn note(sample: SampleId, pitch: Pitch) -> Self {
        Self {
            pitch: Some(pitch),
            sample: Some(sample),
            volume: Some(Volume::MAX),
            effect: Effect::None,
            pan: None,
            adsr: None,
        }
    }

    /// Note-on with explicit volume.
    pub const fn note_vol(sample: SampleId, pitch: Pitch, vol: Volume) -> Self {
        Self {
            pitch: Some(pitch),
            sample: Some(sample),
            volume: Some(vol),
            effect: Effect::None,
            pan: None,
            adsr: None,
        }
    }

    /// Override the stereo panning for this cell.
    pub const fn with_pan(mut self, pan: Pan) -> Self {
        self.pan = Some(pan);
        self
    }

    /// Override the ADSR envelope for this cell.
    pub const fn with_adsr(mut self, adsr: Adsr) -> Self {
        self.adsr = Some(adsr);
        self
    }
}

// ---------------------------------------------------------------------------
// Event-based Pattern
// ---------------------------------------------------------------------------

/// Maximum events (non-empty cells) per pattern.
pub const MAX_EVENTS: usize = 48;

/// One event in a pattern: a [`Cell`] placed at a specific row and
/// **global** tracker channel.
#[derive(Clone, Copy)]
pub struct Event {
    pub row: u8,
    pub ch: u8,
    pub cell: Cell,
}

impl Event {
    const EMPTY: Self = Self {
        row: 0,
        ch: 0,
        cell: Cell::EMPTY,
    };
}

/// A pattern: a sparse list of [`Event`]s on a timeline of `ROWS` rows.
///
/// Channel indices are **global** tracker channels (0–23).  Multiple
/// patterns passed to [`play_patterns`] share the same channel namespace —
/// no automatic offsetting occurs.
///
/// ```ignore
/// const PAT: Pattern<16> = Pattern::new()
///     .set(0, 0, Cell::note(HAT,   Pitch(0x1000)))   // row 0, channel 0
///     .set(0, 1, Cell::note(KICK,  Pitch(0x1000)))   // row 0, channel 1
///     .set(4, 2, Cell::note(SNARE, Pitch(0x1000)));   // row 4, channel 2
/// ```
#[derive(Clone, Copy)]
pub struct Pattern<const ROWS: usize> {
    events: [Event; MAX_EVENTS],
    len: u8,
}

impl<const ROWS: usize> Pattern<ROWS> {
    pub const fn new() -> Self {
        Self {
            events: [Event::EMPTY; MAX_EVENTS],
            len: 0,
        }
    }

    /// Place a cell at (`row`, `ch`).  `ch` is a global tracker channel.
    pub const fn set(mut self, row: usize, ch: usize, cell: Cell) -> Self {
        if row < ROWS && (self.len as usize) < MAX_EVENTS {
            self.events[self.len as usize] = Event {
                row: row as u8,
                ch: ch as u8,
                cell,
            };
            self.len += 1;
        }
        self
    }
}

/// Type-erased access to pattern data.
///
/// Allows [`Engine::play_patterns`](super::engine::Engine::play_patterns)
/// to layer patterns with different row counts in one call.
pub trait PatternSource {
    fn rows(&self) -> usize;
    fn event_count(&self) -> usize;
    fn event(&self, idx: usize) -> &Event;
}

impl<const ROWS: usize> PatternSource for Pattern<ROWS> {
    fn rows(&self) -> usize { ROWS }
    fn event_count(&self) -> usize { self.len as usize }
    fn event(&self, idx: usize) -> &Event { &self.events[idx] }
}

/// A complete, swappable audio configuration.
///
/// Bundles sample data references and a voice layout so that the engine
/// can atomically switch to a new set of sounds (e.g. on level change).
pub struct SoundProject<const SAMPLES: usize> {
    pub samples: [&'static [u8]; SAMPLES],
    pub layout: VoiceLayout,
}
