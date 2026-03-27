use crate::runtime::TaskStack;
use crate::spu::*;
use crate::spu::reverb::ReverbConfig;

// ---------------------------------------------------------------------------
// Samples
// ---------------------------------------------------------------------------

const BELL: SampleId = SampleId(0);
const SWOOSH: SampleId = SampleId(1);
const SWEEP: SampleId = SampleId(2);

pub const PROJECT: SoundProject<3> = SoundProject {
    samples: [
        crate::include_bytes_skip!("../samples/file_all.spu", 0, 13500),
        crate::include_bytes_skip!("../samples/file_all.spu", 13600, 8400),
        crate::include_bytes_skip!("../samples/file_all.spu", 22000),
    ],
    layout: VoiceLayout::new((0, 24), (0, 0)),
};

// ---------------------------------------------------------------------------
// ADSR envelopes
//
// Register format (32-bit, two halfwords written to 1F801C08h+N*10h):
//
//   Lower [Am:1][Ash:5][As:2][Dsh:4][Sl:4]
//   Upper [_:1][Sm:1][Sd:1][Ssh:5][Ss:2][Rm:1][Rsh:5]
// ---------------------------------------------------------------------------

/// ADSR envelope parameters matching the SPU register layout.
struct Adsr {
    am: u32, ash: u32, a_s: u32,
    dsh: u32, sl: u32,
    sm: u32, sd: u32, ssh: u32, ss: u32,
    rm: u32, rsh: u32,
}

impl Adsr {
    const fn pack(self) -> u32 {
        let lo = (self.am << 15) | (self.ash << 10) | (self.a_s << 8)
               | (self.dsh << 4) | self.sl;
        let hi = (self.sm << 14) | (self.sd << 13) | (self.ssh << 8)
               | (self.ss << 6) | (self.rm << 5) | self.rsh;
        (hi << 16) | lo
    }
}

/// Pad envelope for Spacious Sweep.
///
/// Attack:  exponential, shift=10 — fast rise (sample has built-in swell)
/// Decay:   shift=4 — quick settle from peak to sustain
/// Sustain: level=10 (~69%), exp decrease shift=18 — ~8 s half-life,
///          gentle fade to ~60% over the 6.4 s song
/// Release: exponential, shift=14 — ~0.5 s half-life, smooth ~3.5 s tail
const ADSR_SWEEP: u32 = Adsr {
    am: 1, ash: 0x0A, a_s: 0, dsh: 0x04, sl: 0x0A,
    sm: 1, sd: 1, ssh: 0x12, ss: 0, rm: 1, rsh: 0x0E,
}.pack();

/// Bell pluck: instant attack, audible decay to ~69%,
/// then gentle exponential ring-out.
///
/// Decay:   shift=10 — ~46 ms half-life, pluck with body
/// Sustain: level=10 (~69%), exp decrease shift=15 — ~1.5 s half-life
/// Release: exponential, shift=8 — fast cutoff on KEY OFF
const ADSR_BELL: u32 = Adsr {
    am: 0, ash: 0x00, a_s: 0, dsh: 0x0A, sl: 0x0A,
    sm: 1, sd: 1, ssh: 0x0F, ss: 0, rm: 1, rsh: 0x08,
}.pack();

/// Swoosh: instant attack, held at max, gradual fade.
///
/// Sustain: max level, exp decrease shift=16 — fades during playback
/// Release: exponential, shift=12 — ~0.13 s half-life, clean cutoff
const ADSR_SWOOSH: u32 = Adsr {
    am: 0, ash: 0x00, a_s: 0, dsh: 0x00, sl: 0x0F,
    sm: 1, sd: 1, ssh: 0x10, ss: 0, rm: 1, rsh: 0x0C,
}.pack();

/// Deep low bell: moderate attack, high sustain, prolonged ring-out.
///
/// Decay:   shift=10 — ~46 ms half-life, gentle settling
/// Sustain: level=10 (~69%), exp decrease shift=19 — ~24 s half-life
/// Release: exponential, shift=16 — ~2 s half-life, long natural tail
const ADSR_LOW_BELL: u32 = Adsr {
    am: 1, ash: 0x04, a_s: 0, dsh: 0x0A, sl: 0x0A,
    sm: 1, sd: 1, ssh: 0x13, ss: 0, rm: 1, rsh: 0x10,
}.pack();

// ---------------------------------------------------------------------------
// Timing — BPM 300 → 50 ms per row, 20 rows per second
// ---------------------------------------------------------------------------

const BPM: u16 = 300;

// ---------------------------------------------------------------------------
// Cell constructors
// ---------------------------------------------------------------------------

const fn sweep(pitch: Pitch, pan: Pan) -> Cell {
    Cell::note_vol(SWEEP, pitch, Volume::HALF)
        .with_pan(pan)
        .with_adsr(ADSR_SWEEP)
}

const fn sweep_bass(pitch: Pitch, pan: Pan) -> Cell {
    Cell::note_vol(SWEEP, pitch, Volume(0x1400))
        .with_pan(pan)
        .with_adsr(ADSR_SWEEP)
}

const fn sweep_quiet(pitch: Pitch, pan: Pan) -> Cell {
    Cell::note_vol(SWEEP, pitch, Volume::QUARTER)
        .with_pan(pan)
        .with_adsr(ADSR_SWEEP)
}

const fn bell(pitch: Pitch, pan: Pan) -> Cell {
    Cell::note_vol(BELL, pitch, Volume(0x2800))
        .with_pan(pan)
        .with_adsr(ADSR_BELL)
}

const fn swoosh_cell(pitch: Pitch, pan: Pan) -> Cell {
    Cell::note(SWOOSH, pitch).with_pan(pan).with_adsr(ADSR_SWOOSH)
}

const fn low_bell(pitch: Pitch, pan: Pan) -> Cell {
    Cell::note(BELL, pitch).with_pan(pan).with_adsr(ADSR_LOW_BELL)
}

// ---------------------------------------------------------------------------
// Position 0 (rows 0–63, 3.2 s)
//
//   t = 0.0 s  Spacious Sweep — bass foundation + main note (plays ALONE)
//   t = 1.5 s  Bell twinkling begins
//   t = 2.4 s  Spacious Sweep — second note enters
// ---------------------------------------------------------------------------

/// Spacious Sweep voices — three octave layers, same channels for both notes.
///
/// When the second note triggers on the same channel, SPU naturally replaces
/// the first note so both aren't fighting for headroom.
///
/// Bass @ ~5.5 kHz → ~3.7 kHz: deep foundation (ch 20-21)
/// Mid  @ ~11  kHz → ~7.3 kHz: main lead (ch 0-1)
/// High @ ~22  kHz → ~14.6 kHz: quiet shimmer (ch 2-3)
const SWEEP_PAT_0: Pattern<64> = Pattern::new()
    // ---- first note (G) — deep bass foundation ----
    .set(0, 20, sweep_bass(Pitch(0x0200), Pan(-16)))
    .set(0, 21, sweep_bass(Pitch(0x0203), Pan(16)))
    .set(0,  0, sweep(Pitch(0x0400), Pan::LEFT))
    .set(0,  1, sweep(Pitch(0x0403), Pan::RIGHT))
    .set(0,  2, sweep_quiet(Pitch(0x0800), Pan(-32)))
    .set(0,  3, sweep_quiet(Pitch(0x0806), Pan(32)))
    // ---- second note (C) — re-triggers the SAME channels at row 48 ----
    .set(48, 20, sweep_bass(Pitch(0x0155), Pan(-16)))
    .set(48, 21, sweep_bass(Pitch(0x0158), Pan(16)))
    .set(48,  0, sweep(Pitch(0x02AB), Pan::LEFT))
    .set(48,  1, sweep(Pitch(0x02AE), Pan::RIGHT))
    .set(48,  2, sweep_quiet(Pitch(0x0555), Pan(-32)))
    .set(48,  3, sweep_quiet(Pitch(0x0558), Pan(32)));

/// Bell twinkling — starts 1.5 s after the sweep.
///
/// Sequence: G → C → D → G(8va) → C → C5 → E5 → G5 → C-E-G arpeggio.
/// Each bell is panned to a different position for stereo shimmer.
const BELL_PAT_0: Pattern<64> = Pattern::new()
    .set(30, 6,  bell(Pitch(0x0C00), Pan(-48)))  // G5
    .set(33, 7,  bell(Pitch(0x1000), Pan(40)))   // C6
    .set(36, 8,  bell(Pitch(0x1200), Pan(-32)))  // D6
    .set(39, 9,  bell(Pitch(0x1800), Pan(56)))   // G6
    .set(43, 10, bell(Pitch(0x1000), Pan(-24)))  // C6
    .set(45, 11, bell(Pitch(0x0800), Pan(16)))   // C5
    .set(46, 12, bell(Pitch(0x0A00), Pan(-40)))  // E5
    .set(47, 13, bell(Pitch(0x0C00), Pan(48)))   // G5
    // C-major arpeggio
    .set(50, 6,  bell(Pitch(0x1000), Pan(-36)))  // C6
    .set(51, 7,  bell(Pitch(0x1400), Pan(44)))   // E6
    .set(52, 8,  bell(Pitch(0x1800), Pan(-20))); // G6

/// No swoosh in position 0.
const SWOOSH_PAT_0: Pattern<64> = Pattern::new();

// ---------------------------------------------------------------------------
// Position 1 (rows 0–63, 3.2 s — begins at t = 3.2 s)
//
//   t = 3.2 s  Swoosh low enters + bell cacophony
//   t = 3.5 s  Swoosh high layers in
//   t = 4.2 s  Deep low bell
// ---------------------------------------------------------------------------

/// Sweep voices continue through ADSR sustain — no new triggers.
const SWEEP_PAT_1: Pattern<64> = Pattern::new();

/// Bell cacophony — rapid-fire notes, then a deep low bell.
const BELL_PAT_1: Pattern<64> = Pattern::new()
    // ---- cacophony: one or two bells per row ----
    .set(0,  6,  bell(Pitch(0x1000), Pan(-56)))  // C6
    .set(1,  7,  bell(Pitch(0x1400), Pan(52)))   // E6
    .set(1,  8,  bell(Pitch(0x1800), Pan(24)))   // G6
    .set(2,  9,  bell(Pitch(0x2000), Pan(-44)))  // C7
    .set(2,  10, bell(Pitch(0x0C00), Pan(36)))   // G5
    .set(3,  11, bell(Pitch(0x1400), Pan(-28)))  // E6
    .set(3,  12, bell(Pitch(0x1200), Pan(48)))   // D6
    .set(4,  13, bell(Pitch(0x1000), Pan(-52)))  // C6
    .set(5,  14, bell(Pitch(0x1800), Pan(32)))   // G6
    .set(5,  15, bell(Pitch(0x1400), Pan(-16)))  // E6
    .set(7,  6,  bell(Pitch(0x1000), Pan(44)))   // C6
    .set(8,  7,  bell(Pitch(0x0C00), Pan(-8)))   // G5
    .set(10, 8,  bell(Pitch(0x1200), Pan(-48)))  // D6
    // ---- deep low bell, stereo pair with detune ----
    .set(20, 18, low_bell(Pitch(0x0180), Pan(-40)))
    .set(20, 19, low_bell(Pitch(0x0186), Pan(40)));

/// Swoosh — reversed glass.  The lower voice enters first, creating the
/// initial dramatic sweep; the higher voice layers on top ~300 ms later
/// adding metallic brightness.
const SWOOSH_PAT_1: Pattern<64> = Pattern::new()
    // Lower swoosh — deeper, enters first
    .set(0, 16, swoosh_cell(Pitch(0x0600), Pan(-32)))
    // Higher swoosh — brighter, added on top
    .set(6, 17, swoosh_cell(Pitch(0x0C00), Pan(32)));

// ---------------------------------------------------------------------------
// Song assembly — 3 tracks × 2 positions × 64 rows = 6.4 seconds
// ---------------------------------------------------------------------------

const BOOT_SONG: Song<3, 2, 64> = Song::new(BPM)
    .with_track(0, Track::new()
        .with_pattern(0, SWEEP_PAT_0)
        .with_pattern(1, SWEEP_PAT_1)
        .with_order(&[0, 1]))
    .with_track(1, Track::new()
        .with_pattern(0, BELL_PAT_0)
        .with_pattern(1, BELL_PAT_1)
        .with_order(&[0, 1]))
    .with_track(2, Track::new()
        .with_pattern(0, SWOOSH_PAT_0)
        .with_pattern(1, SWOOSH_PAT_1)
        .with_order(&[0, 1]));

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub static MUSIC_STACK: TaskStack<2048> = TaskStack::new();

pub extern "C" fn music_task() {
    let mut e = Engine::take().unwrap();
    e.load_project(&PROJECT);

    e.enable_reverb(&ReverbConfig::SPACE, 0x5000, 0x3000);
    for ch in 0..22 {
        e.set_channel_reverb(ch, true);
    }

    e.play_song(&BOOT_SONG);
}
