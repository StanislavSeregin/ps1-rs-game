#[derive(Clone, Copy)]
pub struct Note {
    pub sample_id: u16,
    pub pitch: u16,
    pub volume: u16,
}

impl Note {
    pub const fn new(sample_id: u16, pitch: u16) -> Self {
        Self {
            sample_id,
            pitch,
            volume: 0x3FFF,
        }
    }

    pub const fn with_volume(sample_id: u16, pitch: u16, volume: u16) -> Self {
        Self {
            sample_id,
            pitch,
            volume,
        }
    }
}

#[derive(Clone, Copy)]
pub struct PatternEntry {
    pub note: Option<Note>,
}

impl PatternEntry {
    pub const fn empty() -> Self {
        Self { note: None }
    }

    pub const fn note(note: Note) -> Self {
        Self { note: Some(note) }
    }
}

#[derive(Clone, Copy)]
pub struct Pattern<const ROWS: usize> {
    pub entries: [PatternEntry; ROWS],
}

impl<const ROWS: usize> Pattern<ROWS> {
    pub const fn new() -> Self {
        Self {
            entries: [PatternEntry::empty(); ROWS],
        }
    }

    pub const fn with_note(mut self, row: usize, note: Note) -> Self {
        if row < ROWS {
            self.entries[row] = PatternEntry::note(note);
        }
        self
    }
}

#[derive(Clone, Copy)]
pub struct Track<const PATTERNS: usize, const ROWS: usize> {
    pub patterns: [Pattern<ROWS>; PATTERNS],
    pub pattern_order: [u8; PATTERNS],
    pub length: usize,
}

impl<const PATTERNS: usize, const ROWS: usize> Track<PATTERNS, ROWS> {
    pub const fn new() -> Self {
        Self {
            patterns: [Pattern::new(); PATTERNS],
            pattern_order: [0; PATTERNS],
            length: 0,
        }
    }

    pub const fn with_pattern(mut self, pattern_idx: u8, pattern: Pattern<ROWS>) -> Self {
        if (pattern_idx as usize) < PATTERNS {
            self.patterns[pattern_idx as usize] = pattern;
        }
        self
    }

    pub const fn with_order(mut self, order: &[u8]) -> Self {
        let mut i = 0;
        while i < order.len() && i < PATTERNS {
            self.pattern_order[i] = order[i];
            i += 1;
        }
        Self {
            length: i,
            ..self
        }
    }
}

#[derive(Clone, Copy)]
pub struct Song<const TRACKS: usize, const PATTERNS: usize, const ROWS: usize> {
    pub tracks: [Track<PATTERNS, ROWS>; TRACKS],
    pub tempo: u16,
    pub ticks_per_row: u8,
}

impl<const TRACKS: usize, const PATTERNS: usize, const ROWS: usize> Song<TRACKS, PATTERNS, ROWS> {
    pub const fn new(tempo: u16) -> Self {
        Self {
            tracks: [Track::new(); TRACKS],
            tempo,
            ticks_per_row: 6,
        }
    }

    pub const fn with_track(mut self, track_idx: usize, track: Track<PATTERNS, ROWS>) -> Self {
        if track_idx < TRACKS {
            self.tracks[track_idx] = track;
        }
        self
    }

    pub const fn with_ticks_per_row(mut self, ticks: u8) -> Self {
        Self {
            ticks_per_row: ticks,
            ..self
        }
    }
}

use crate::spu::{Sampler, Sample, Voice};

pub struct Sequencer<'a, const MAX_VOICES: usize> {
    sampler: &'a mut Sampler,
    samples: [Option<Sample>; MAX_VOICES],
    voices: [Option<VoiceInfo>; MAX_VOICES],
    current_tick: u32,
    current_row: usize,
    current_pattern: usize,
    playing: bool,
    ticks_per_frame: u32,
    frame_counter: u32,
}

#[derive(Clone, Copy)]
struct VoiceInfo {
    voice_id: u8,
    active: bool,
}

impl<'a, const MAX_VOICES: usize> Sequencer<'a, MAX_VOICES> {
    pub fn new(sampler: &'a mut Sampler) -> Self {
        Self {
            sampler,
            samples: [None; MAX_VOICES],
            voices: [None; MAX_VOICES],
            current_tick: 0,
            current_row: 0,
            current_pattern: 0,
            playing: false,
            ticks_per_frame: 1,
            frame_counter: 0,
        }
    }

    pub fn load_sample(&mut self, slot: usize, audio_data: &[u8]) -> Result<(), &'static str> {
        if slot >= MAX_VOICES {
            return Err("Invalid slot");
        }

        let sample = self.sampler.load(audio_data)?;
        self.samples[slot] = Some(sample);
        Ok(())
    }

    pub fn play_song<const TRACKS: usize, const PATTERNS: usize, const ROWS: usize>(
        &mut self,
        song: &Song<TRACKS, PATTERNS, ROWS>,
    ) {
        self.playing = true;
        self.current_tick = 0;
        self.current_row = 0;
        self.current_pattern = 0;
        self.ticks_per_frame = (60 * song.ticks_per_row as u32) / song.tempo as u32;
    }

    pub fn stop(&mut self) {
        self.playing = false;
        for voice_info in &mut self.voices {
            if let Some(info) = voice_info {
                info.active = false;
            }
        }
    }

    pub fn update<const TRACKS: usize, const PATTERNS: usize, const ROWS: usize>(
        &mut self,
        song: &Song<TRACKS, PATTERNS, ROWS>,
    ) {
        if !self.playing {
            return;
        }

        self.frame_counter += 1;
        if self.frame_counter >= self.ticks_per_frame {
            self.frame_counter = 0;
            self.process_tick(song);
        }
    }

    fn process_tick<const TRACKS: usize, const PATTERNS: usize, const ROWS: usize>(
        &mut self,
        song: &Song<TRACKS, PATTERNS, ROWS>,
    ) {
        if self.current_tick == 0 {
            self.process_row(song);
        }

        self.current_tick += 1;
        if self.current_tick >= song.ticks_per_row as u32 {
            self.current_tick = 0;
            self.current_row += 1;

            if self.current_row >= ROWS {
                self.current_row = 0;
                self.current_pattern += 1;
            }
        }
    }

    fn process_row<const TRACKS: usize, const PATTERNS: usize, const ROWS: usize>(
        &mut self,
        song: &Song<TRACKS, PATTERNS, ROWS>,
    ) {
        for (track_idx, track) in song.tracks.iter().enumerate().take(MAX_VOICES) {
            if self.current_pattern >= track.length {
                continue;
            }

            let pattern_idx = track.pattern_order[self.current_pattern] as usize;
            if pattern_idx >= PATTERNS {
                continue;
            }

            let pattern = &track.patterns[pattern_idx];
            let entry = &pattern.entries[self.current_row];

            if let Some(note) = entry.note {
                self.play_note(track_idx, note);
            }
        }
    }

    fn play_note(&mut self, voice_idx: usize, note: Note) {
        if voice_idx >= MAX_VOICES {
            return;
        }

        let sample_slot = (note.sample_id - 1) as usize;
        if sample_slot >= MAX_VOICES {
            return;
        }

        if let Some(sample) = self.samples[sample_slot] {
            self.voices[voice_idx] = Some(VoiceInfo {
                voice_id: voice_idx as u8,
                active: true,
            });

            match voice_idx {
                0 => {
                    let mut voice = sample.bind_to_voice::<0>(note.pitch, note.volume);
                    voice.play();
                }
                1 => {
                    let mut voice = sample.bind_to_voice::<1>(note.pitch, note.volume);
                    voice.play();
                }
                2 => {
                    let mut voice = sample.bind_to_voice::<2>(note.pitch, note.volume);
                    voice.play();
                }
                3 => {
                    let mut voice = sample.bind_to_voice::<3>(note.pitch, note.volume);
                    voice.play();
                }
                _ => {}
            }
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn get_position(&self) -> (usize, usize) {
        (self.current_pattern, self.current_row)
    }
}