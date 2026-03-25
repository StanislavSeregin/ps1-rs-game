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

/// Extensible effect slot.
///
/// Starts with only `None`; future variants (portamento, vibrato, etc.)
/// can be added without breaking existing patterns.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    None,
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
}

impl Cell {
    pub const EMPTY: Self = Self {
        pitch: None,
        sample: None,
        volume: None,
        effect: Effect::None,
    };

    /// Note-on with default max volume.
    pub const fn note(sample: SampleId, pitch: Pitch) -> Self {
        Self {
            pitch: Some(pitch),
            sample: Some(sample),
            volume: Some(Volume::MAX),
            effect: Effect::None,
        }
    }

    /// Note-on with explicit volume.
    pub const fn note_vol(sample: SampleId, pitch: Pitch, vol: Volume) -> Self {
        Self {
            pitch: Some(pitch),
            sample: Some(sample),
            volume: Some(vol),
            effect: Effect::None,
        }
    }
}

/// A pattern: a `ROWS x CHANNELS` grid of [`Cell`]s.
///
/// Built at compile time via the builder pattern:
/// ```ignore
/// const PAT: Pattern<2, 8> = Pattern::new()
///     .set(0, 0, Cell::note(KICK, Pitch::C4))
///     .set(4, 1, Cell::note(SNARE, Pitch::D4));
/// ```
#[derive(Clone, Copy)]
pub struct Pattern<const CHANNELS: usize, const ROWS: usize> {
    pub cells: [[Cell; CHANNELS]; ROWS],
}

impl<const CH: usize, const ROWS: usize> Pattern<CH, ROWS> {
    pub const fn new() -> Self {
        Self {
            cells: [[Cell::EMPTY; CH]; ROWS],
        }
    }

    /// Set a single cell. Returns `self` for chaining.
    pub const fn set(mut self, row: usize, ch: usize, cell: Cell) -> Self {
        if row < ROWS && ch < CH {
            self.cells[row][ch] = cell;
        }
        self
    }
}

impl<const ROWS: usize> Pattern<1, ROWS> {
    /// Set a cell in a single-channel pattern (no channel index needed).
    pub const fn set_cell(self, row: usize, cell: Cell) -> Self {
        self.set(row, 0, cell)
    }
}

/// Type-erased access to pattern data.
///
/// Allows [`Engine::play_patterns`](super::engine::Engine::play_patterns)
/// to layer patterns with different channel counts in one call.
/// Implemented automatically for every `Pattern<CH, ROWS>`.
pub trait PatternSource {
    fn rows(&self) -> usize;
    fn channels(&self) -> usize;
    fn cell(&self, row: usize, ch: usize) -> &Cell;
}

impl<const CH: usize, const ROWS: usize> PatternSource for Pattern<CH, ROWS> {
    fn rows(&self) -> usize { ROWS }
    fn channels(&self) -> usize { CH }
    fn cell(&self, row: usize, ch: usize) -> &Cell { &self.cells[row][ch] }
}

/// Maximum length of a track's pattern order list.
pub const MAX_ORDER: usize = 64;

/// One layer of a song: a bank of patterns plus an order list.
///
/// `CH` channels per pattern, `PAT` unique patterns, `ROWS` rows per pattern.
/// Each track occupies `CH` consecutive voice channels during playback.
#[derive(Clone, Copy)]
pub struct Track<const CH: usize, const PAT: usize, const ROWS: usize> {
    pub patterns: [Pattern<CH, ROWS>; PAT],
    pub order: [u8; MAX_ORDER],
    pub order_len: usize,
}

impl<const CH: usize, const PAT: usize, const ROWS: usize> Track<CH, PAT, ROWS> {
    pub const fn new() -> Self {
        Self {
            patterns: [Pattern::new(); PAT],
            order: [0; MAX_ORDER],
            order_len: 0,
        }
    }

    pub const fn with_pattern(mut self, idx: usize, pattern: Pattern<CH, ROWS>) -> Self {
        if idx < PAT {
            self.patterns[idx] = pattern;
        }
        self
    }

    pub const fn with_order(mut self, order: &[u8]) -> Self {
        let mut i = 0;
        let len = if order.len() < MAX_ORDER {
            order.len()
        } else {
            MAX_ORDER
        };
        while i < len {
            self.order[i] = order[i];
            i += 1;
        }
        self.order_len = len;
        self
    }
}

/// A song: multiple tracks playing simultaneously.
///
/// `TRACKS` layers, each with `CH` channels, `PAT` patterns, `ROWS` rows.
/// Track *i* uses voice channels `i*CH .. (i+1)*CH`.
/// Timing is controlled by `bpm` (1 beat = 4 rows).
#[derive(Clone, Copy)]
pub struct Song<const TRACKS: usize, const CH: usize, const PAT: usize, const ROWS: usize> {
    pub tracks: [Track<CH, PAT, ROWS>; TRACKS],
    pub bpm: u16,
}

impl<const TRACKS: usize, const CH: usize, const PAT: usize, const ROWS: usize>
    Song<TRACKS, CH, PAT, ROWS>
{
    pub const fn new(bpm: u16) -> Self {
        Self {
            tracks: [Track::new(); TRACKS],
            bpm,
        }
    }

    pub const fn with_track(mut self, idx: usize, track: Track<CH, PAT, ROWS>) -> Self {
        if idx < TRACKS {
            self.tracks[idx] = track;
        }
        self
    }
}

/// A complete, swappable audio configuration.
///
/// Bundles sample data references and a voice layout so that the engine
/// can atomically switch to a new set of sounds (e.g. on level change).
pub struct SoundProject<const SAMPLES: usize> {
    pub samples: [&'static [u8]; SAMPLES],
    pub layout: VoiceLayout,
}
