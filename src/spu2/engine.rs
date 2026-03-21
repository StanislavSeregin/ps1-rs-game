use super::bus::{self, AudioStatus, Command};
use super::hw::{self, VoiceHw};
use super::music::{Cell, Effect, Pattern, Pitch, Song, SoundProject, Volume};
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

    /// Play an entire song, pattern by pattern, according to the order list.
    ///
    /// Returns when the song ends or a [`Command::Interrupt`] is received.
    pub fn play_song<const CH: usize, const PAT: usize, const ROWS: usize>(
        &mut self,
        song: &Song<CH, PAT, ROWS>,
    ) {
        self.bpm = song.bpm;
        self.interrupted = false;

        for order_idx in 0..song.order_len {
            let pat_idx = song.order[order_idx] as usize;
            if pat_idx >= PAT {
                continue;
            }

            bus::set_status(AudioStatus {
                playing: true,
                current_pattern: order_idx as u16,
                current_row: 0,
            });

            if self.play_pattern_inner(&song.patterns[pat_idx], order_idx as u16).interrupted() {
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

    /// Play a single pattern (all rows), then release music voices.
    ///
    /// The engine maintains a running pattern counter so that
    /// [`audio_status()`](super::bus::audio_status) reports meaningful
    /// positions even when patterns are played individually via control flow.
    ///
    /// Returns when the pattern ends or a [`Command::Interrupt`] is received.
    pub fn play_pattern<const CH: usize, const ROWS: usize>(
        &mut self,
        pattern: &Pattern<CH, ROWS>,
    ) {
        self.interrupted = false;
        let idx = self.pattern_counter;
        self.pattern_counter = self.pattern_counter.wrapping_add(1);
        self.play_pattern_inner(pattern, idx);
        self.release_music_voices();
    }

    fn play_pattern_inner<const CH: usize, const ROWS: usize>(
        &mut self,
        pattern: &Pattern<CH, ROWS>,
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

    fn trigger_row<const CH: usize, const ROWS: usize>(
        &mut self,
        pattern: &Pattern<CH, ROWS>,
        row: usize,
    ) {
        for ch in 0..CH {
            let cell = &pattern.cells[row][ch];
            self.apply_cell(ch, cell);
        }
    }

    fn apply_cell(&mut self, ch: usize, cell: &Cell) {
        if matches!(
            cell,
            Cell {
                pitch: None,
                sample: None,
                volume: None,
                effect: Effect::None,
            }
        ) {
            return;
        }

        if let (Some(sample_id), Some(pitch)) = (cell.sample, cell.pitch) {
            if pitch.0 == 0 {
                if let Some(voice) = self.channel_voices[ch].take() {
                    self.voices.release_music(&voice);
                }
                return;
            }

            let sample_ref = match self.samples.get(sample_id) {
                Some(s) => s,
                None => return,
            };

            if let Some(old) = self.channel_voices[ch].take() {
                self.voices.release_music(&old);
            }

            let voice = match self.voices.claim_music() {
                Some(v) => v,
                None => return,
            };

            let vol = cell.volume.unwrap_or(Volume::MAX).0;
            voice.trigger(sample_ref.spu_addr, pitch.0, vol, DEFAULT_ADSR);
            self.channel_voices[ch] = Some(voice);
        } else if let Some(vol) = cell.volume {
            if let Some(voice) = &self.channel_voices[ch] {
                voice.set_volume(vol.0, vol.0);
            }
        }
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

    /// Reset the pattern counter to 0.
    /// Useful at the start of a song loop when using `play_pattern` directly.
    pub fn reset_pattern_counter(&mut self) {
        self.pattern_counter = 0;
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
                self.voices.release_music(&voice);
            }
        }
    }
}
