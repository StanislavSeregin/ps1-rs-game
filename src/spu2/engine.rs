use super::bus::{self, AudioStatus, Command};
use super::hw::{self, VoiceHw};
use super::music::{Cell, Effect, Pattern, PatternSource, Pitch, Song, SoundProject, Volume};
use super::reverb::ReverbConfig;
use super::sample::{SampleBank, SampleId};
use super::voice::{VoiceAlloc, VoiceLayout};
use crate::runtime;

const DEFAULT_ADSR: u32 = 0x80FF_8000;

/// Convention: one beat = 4 rows (standard 4/4 tracker convention).
const ROWS_PER_BEAT: u32 = 4;

/// Result of waiting for a row -- tells the caller whether the wait
/// completed normally or was interrupted by a [`Command::Interrupt`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WaitResult {
    Complete,
    Interrupted,
}

impl WaitResult {
    pub fn interrupted(self) -> bool {
        self == Self::Interrupted
    }
}

/// Coroutine-based audio engine.
///
/// Owns all audio resources (samples, voices) and exposes blocking
/// playback methods that internally yield via [`runtime::yield_now()`].
///
/// Timing is driven by a hardware HBlank counter, not by VBlank or
/// yield counting.  The single timing parameter is **BPM** (beats per
/// minute), where 1 beat = 4 rows by convention.
///
/// # Usage
///
/// Meant to live inside a static-spawned coroutine:
///
/// ```ignore
/// extern "C" fn music_task() {
///     let mut e = Engine::take().unwrap();
///     e.load_project(&MY_PROJECT);
///     e.set_bpm(120);
///     loop {
///         e.play_pattern(&VERSE);
///         e.play_pattern(&CHORUS);
///     }
/// }
/// ```
pub struct Engine {
    samples: SampleBank,
    voices: VoiceAlloc,
    bpm: u16,
    interrupted: bool,
    pattern_counter: u16,
    last_hblank: u16,
    channel_voices: [Option<VoiceHw>; 24],
    /// Per-tracker-channel reverb enable (bit N = channel N feeds reverb).
    channel_reverb: u32,
    /// Current hardware EON mask — tracks which active voices have reverb.
    reverb_voice_mask: u32,
}

impl Engine {
    /// Acquire the singleton engine, initialising SPU hardware and timer.
    ///
    /// Returns `None` if the engine was already taken.
    pub fn take() -> Option<Self> {
        static mut TAKEN: bool = false;
        if unsafe { TAKEN } {
            return None;
        }
        unsafe { TAKEN = true };

        hw::init_spu_hardware();
        hw::init_hblank_timer();

        Some(Self {
            samples: SampleBank::new(),
            voices: VoiceAlloc::with_layout(VoiceLayout::new((0, 16), (16, 8))),
            bpm: 120,
            interrupted: false,
            pattern_counter: 0,
            last_hblank: hw::read_hblank_counter(),
            channel_voices: [None; 24],
            channel_reverb: 0,
            reverb_voice_mask: 0,
        })
    }

    // -----------------------------------------------------------------------
    // Project management
    // -----------------------------------------------------------------------

    /// Stop all playback, reload samples and voice layout from a project.
    pub fn load_project<const N: usize>(&mut self, project: &SoundProject<N>) {
        self.voices.release_all();
        self.channel_voices = [None; 24];
        self.samples.reset();
        self.voices.set_layout(project.layout);
        self.interrupted = false;
        self.pattern_counter = 0;

        for (i, data) in project.samples.iter().enumerate() {
            let _ = self.samples.load(SampleId(i as u8), data);
        }
    }

    // -----------------------------------------------------------------------
    // Sequenced playback (blocking -- yields internally)
    // -----------------------------------------------------------------------

    /// Play a multi-track song. All tracks advance in lockstep; each
    /// track's current pattern is layered simultaneously.
    ///
    /// All tracks share the global channel namespace — use distinct
    /// channel indices in each track's patterns to avoid collisions.
    /// Playback continues until the longest track's order list is
    /// exhausted or a [`Command::Interrupt`] is received.
    pub fn play_song<const TRACKS: usize, const PAT: usize, const ROWS: usize>(
        &mut self,
        song: &Song<TRACKS, PAT, ROWS>,
    ) {
        self.bpm = song.bpm;
        self.interrupted = false;

        let mut max_len: usize = 0;
        let mut i = 0;
        while i < TRACKS {
            if song.tracks[i].order_len > max_len {
                max_len = song.tracks[i].order_len;
            }
            i += 1;
        }

        for pos in 0..max_len {
            if self.play_song_position(song, pos).interrupted() {
                return;
            }
        }

        self.release_music_voices();
        bus::set_status(AudioStatus {
            playing: false,
            current_pattern: 0,
            current_row: 0,
        });
    }

    fn play_song_position<const TRACKS: usize, const PAT: usize, const ROWS: usize>(
        &mut self,
        song: &Song<TRACKS, PAT, ROWS>,
        pos: usize,
    ) -> WaitResult {
        for row in 0..ROWS {
            bus::set_status(AudioStatus {
                playing: true,
                current_pattern: pos as u16,
                current_row: row as u16,
            });

            let mut key_on: u32 = 0;
            let mut key_off: u32 = 0;

            for track in song.tracks.iter() {
                if pos >= track.order_len {
                    continue;
                }
                let pat_idx = track.order[pos] as usize;
                if pat_idx >= PAT {
                    continue;
                }
                let pat = &track.patterns[pat_idx];
                for i in 0..pat.event_count() {
                    let ev = pat.event(i);
                    if ev.row as usize == row {
                        let (on, off) = self.apply_cell(ev.ch as usize, &ev.cell);
                        key_on |= on;
                        key_off |= off;
                    }
                }
            }

            Self::flush_keys(key_on, key_off);
            self.flush_reverb();

            if self.wait_row().interrupted() {
                return WaitResult::Interrupted;
            }
        }
        WaitResult::Complete
    }

    /// Play a single pattern (all rows), then release music voices.
    ///
    /// The engine maintains a running pattern counter so that
    /// [`audio_status()`](super::bus::audio_status) reports meaningful
    /// positions even when patterns are played individually via control flow.
    pub fn play_pattern<const ROWS: usize>(
        &mut self,
        pattern: &Pattern<ROWS>,
    ) {
        self.interrupted = false;
        let idx = self.pattern_counter;
        self.pattern_counter = self.pattern_counter.wrapping_add(1);
        self.play_pattern_inner(pattern, idx);
        self.release_music_voices();
    }

    /// Layer multiple patterns simultaneously, then release music voices.
    ///
    /// All patterns share the global channel namespace — channel indices
    /// in [`set()`](Pattern::set) are used as-is, with no automatic
    /// offsetting. Patterns **must** share the same row count.
    pub fn play_patterns(&mut self, patterns: &[&dyn PatternSource]) {
        if patterns.is_empty() {
            return;
        }
        self.interrupted = false;
        let idx = self.pattern_counter;
        self.pattern_counter = self.pattern_counter.wrapping_add(1);
        self.play_patterns_inner(patterns, idx);
        self.release_music_voices();
    }

    fn play_pattern_inner<const ROWS: usize>(
        &mut self,
        pattern: &Pattern<ROWS>,
        pattern_idx: u16,
    ) -> WaitResult {
        for row in 0..ROWS {
            bus::set_status(AudioStatus {
                playing: true,
                current_pattern: pattern_idx,
                current_row: row as u16,
            });

            self.trigger_row(pattern, row);

            if self.wait_row().interrupted() {
                return WaitResult::Interrupted;
            }
        }
        WaitResult::Complete
    }

    fn play_patterns_inner(
        &mut self,
        patterns: &[&dyn PatternSource],
        pattern_idx: u16,
    ) -> WaitResult {
        let rows = patterns[0].rows();

        for row in 0..rows {
            bus::set_status(AudioStatus {
                playing: true,
                current_pattern: pattern_idx,
                current_row: row as u16,
            });

            let mut key_on: u32 = 0;
            let mut key_off: u32 = 0;

            for pat in patterns {
                for i in 0..pat.event_count() {
                    let ev = pat.event(i);
                    if ev.row as usize == row {
                        let (on, off) = self.apply_cell(ev.ch as usize, &ev.cell);
                        key_on |= on;
                        key_off |= off;
                    }
                }
            }

            Self::flush_keys(key_on, key_off);
            self.flush_reverb();

            if self.wait_row().interrupted() {
                return WaitResult::Interrupted;
            }
        }
        WaitResult::Complete
    }

    fn trigger_row<const ROWS: usize>(
        &mut self,
        pattern: &Pattern<ROWS>,
        row: usize,
    ) {
        let mut key_on: u32 = 0;
        let mut key_off: u32 = 0;

        for i in 0..pattern.event_count() {
            let ev = pattern.event(i);
            if ev.row as usize == row {
                let (on, off) = self.apply_cell(ev.ch as usize, &ev.cell);
                key_on |= on;
                key_off |= off;
            }
        }

        Self::flush_keys(key_on, key_off);
        self.flush_reverb();
    }

    fn flush_keys(key_on: u32, key_off: u32) {
        if key_off != 0 {
            hw::key_off_mask(key_off);
        }
        if key_on != 0 {
            hw::key_on_mask(key_on);
        }
    }

    /// Process one cell and return `(key_on_mask, key_off_mask)` bits
    /// so the caller can batch all SPU register writes.
    ///
    /// Also updates [`reverb_voice_mask`](Self::reverb_voice_mask) according
    /// to the per-channel reverb flags in [`channel_reverb`](Self::channel_reverb).
    fn apply_cell(&mut self, ch: usize, cell: &Cell) -> (u32, u32) {
        if matches!(
            cell,
            Cell {
                pitch: None,
                sample: None,
                volume: None,
                effect: Effect::None,
            }
        ) {
            return (0, 0);
        }

        if let (Some(sample_id), Some(pitch)) = (cell.sample, cell.pitch) {
            if pitch.0 == 0 {
                let off = if let Some(voice) = self.channel_voices[ch].take() {
                    self.reverb_voice_mask &= !(1u32 << voice.id());
                    self.voices.release_music_deferred(&voice)
                } else {
                    0
                };
                return (0, off);
            }

            let sample_ref = match self.samples.get(sample_id) {
                Some(s) => s,
                None => return (0, 0),
            };

            let off = if let Some(old) = self.channel_voices[ch].take() {
                self.reverb_voice_mask &= !(1u32 << old.id());
                self.voices.release_music_deferred(&old)
            } else {
                0
            };

            let voice = match self.voices.claim_music() {
                Some(v) => v,
                None => return (0, off),
            };

            let vol = cell.volume.unwrap_or(Volume::MAX).0;
            voice.prepare(sample_ref.spu_addr, pitch.0, vol, DEFAULT_ADSR);
            let voice_bit = 1u32 << voice.id();
            if self.channel_reverb & (1u32 << ch) != 0 {
                self.reverb_voice_mask |= voice_bit;
            }
            self.channel_voices[ch] = Some(voice);
            (voice_bit, off)
        } else if let Some(vol) = cell.volume {
            if let Some(voice) = &self.channel_voices[ch] {
                voice.set_volume(vol.0, vol.0);
            }
            (0, 0)
        } else {
            (0, 0)
        }
    }

    fn flush_reverb(&self) {
        hw::set_reverb_on_mask(self.reverb_voice_mask);
    }

    // -----------------------------------------------------------------------
    // Reverb
    // -----------------------------------------------------------------------

    /// Configure and enable the SPU reverb unit.
    ///
    /// Writes the full reverb register set, clears the work area in SPU RAM,
    /// and enables reverb master writes. Use [`set_channel_reverb`] to select
    /// which tracker channels feed into the reverb unit.
    ///
    /// `input_vol` scales audio entering the reverb (0..`0x7FFF`).
    /// `output_vol` scales the wet signal mixed back into the output
    /// (0..`0x7FFF`). Both are applied symmetrically to L and R.
    ///
    /// Must be called **after** [`load_project`](Self::load_project) so that
    /// the sample address limit is set before any further sample loads.
    pub fn enable_reverb(&mut self, config: &ReverbConfig, input_vol: u16, output_vol: u16) {
        hw::disable_reverb_master();

        hw::set_reverb_base(config.buffer_start);

        let buffer_halfwords = (0x1_0000u32 - config.buffer_start as u32) * 4;
        hw::clear_spu_ram(config.buffer_start, buffer_halfwords);

        hw::write_reverb_config(&config.as_registers());
        hw::set_reverb_volume_in(input_vol, input_vol);
        hw::set_reverb_volume_out(output_vol, output_vol);

        self.samples.set_addr_limit(config.buffer_start);

        hw::enable_reverb_master();
    }

    /// Enable or disable reverb for a single tracker channel.
    ///
    /// When enabled, any voice assigned to this channel will have its
    /// EON (Echo On) bit set automatically. The hardware mask is flushed
    /// on every row together with key-on/key-off.
    pub fn set_channel_reverb(&mut self, channel: usize, enabled: bool) {
        if channel < 24 {
            if enabled {
                self.channel_reverb |= 1u32 << channel;
            } else {
                self.channel_reverb &= !(1u32 << channel);
                if let Some(voice) = &self.channel_voices[channel] {
                    self.reverb_voice_mask &= !(1u32 << voice.id());
                }
            }
        }
    }

    /// Adjust the reverb input volume (L, R).
    pub fn set_reverb_input_volume(&self, left: u16, right: u16) {
        hw::set_reverb_volume_in(left, right);
    }

    /// Adjust the reverb output (wet) volume (L, R).
    pub fn set_reverb_output_volume(&self, left: u16, right: u16) {
        hw::set_reverb_volume_out(left, right);
    }

    /// Fully disable the reverb unit: mute output, clear voice mask,
    /// and stop reverb RAM writes.
    pub fn disable_reverb(&mut self) {
        hw::set_reverb_volume_out(0, 0);
        self.channel_reverb = 0;
        self.reverb_voice_mask = 0;
        hw::set_reverb_on_mask(0);
        hw::disable_reverb_master();
        self.samples.set_addr_limit(u16::MAX);
    }

    // -----------------------------------------------------------------------
    // SFX (fire-and-forget on the SFX voice pool)
    // -----------------------------------------------------------------------

    /// Play a one-shot sample on the SFX pool.
    ///
    /// If no SFX voice is free the call is silently dropped.
    pub fn play_sfx(&mut self, sample: SampleId, pitch: Pitch) {
        let sample_ref = match self.samples.get(sample) {
            Some(s) => s,
            None => return,
        };
        let voice = match self.voices.claim_sfx() {
            Some(v) => v,
            None => return,
        };
        voice.trigger(sample_ref.spu_addr, pitch.0, Volume::MAX.0, DEFAULT_ADSR);
        self.voices.release_sfx(&voice);
    }

    // -----------------------------------------------------------------------
    // Timing
    // -----------------------------------------------------------------------

    /// Compute how many HBlank ticks one row should last at the current BPM.
    ///
    /// Formula: `HBLANK_RATE * 60 / (bpm * ROWS_PER_BEAT)`
    ///
    /// At 120 BPM: 15700 * 60 / 480 = 1962 hblanks ≈ 125 ms per row.
    fn hblanks_per_row(&self) -> u32 {
        hw::HBLANK_RATE * 60 / (self.bpm as u32 * ROWS_PER_BEAT)
    }

    /// Wait for one row duration using the HBlank hardware timer.
    ///
    /// Yields cooperatively while measuring real elapsed time.
    /// Processes commands at each yield point; returns early on interrupt.
    fn wait_row(&mut self) -> WaitResult {
        let target = self.hblanks_per_row();
        let mut remaining = target;

        loop {
            runtime::yield_now();

            let now = hw::read_hblank_counter();
            let delta = now.wrapping_sub(self.last_hblank) as u32;
            self.last_hblank = now;
            remaining = remaining.saturating_sub(delta);

            self.process_commands();
            if self.interrupted {
                return WaitResult::Interrupted;
            }

            if remaining == 0 {
                return WaitResult::Complete;
            }
        }
    }

    /// Yield until there is at least one command to process.
    pub fn idle(&mut self) {
        loop {
            self.process_commands();
            if self.interrupted {
                return;
            }
            runtime::yield_now();
        }
    }

    // -----------------------------------------------------------------------
    // Command processing
    // -----------------------------------------------------------------------

    fn process_commands(&mut self) {
        while let Some(cmd) = bus::poll_command() {
            match cmd {
                Command::PlaySfx(sample, pitch) => {
                    self.play_sfx(sample, pitch);
                }
                Command::Interrupt => {
                    self.interrupted = true;
                }
                Command::SetBpm(bpm) => {
                    self.bpm = bpm;
                }
                Command::StopAll => {
                    self.voices.release_all();
                    self.channel_voices = [None; 24];
                    self.reverb_voice_mask = 0;
                    hw::set_reverb_on_mask(0);
                    self.interrupted = true;
                }
            }
        }
    }

    /// Clear the interrupt flag.  Call after handling an interruption
    /// at the top-level coroutine before starting new playback.
    pub fn clear_interrupt(&mut self) {
        self.interrupted = false;
    }

    pub fn is_interrupted(&self) -> bool {
        self.interrupted
    }

    /// Set the BPM (beats per minute, where 1 beat = 4 rows).
    pub fn set_bpm(&mut self, bpm: u16) {
        self.bpm = bpm;
    }

    fn release_music_voices(&mut self) {
        for slot in self.channel_voices.iter_mut() {
            if let Some(voice) = slot.take() {
                self.reverb_voice_mask &= !(1u32 << voice.id());
                self.voices.release_music(&voice);
            }
        }
        hw::set_reverb_on_mask(self.reverb_voice_mask);
    }
}
