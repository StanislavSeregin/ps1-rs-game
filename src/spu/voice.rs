use super::hw::VoiceHw;

/// Defines how the 24 SPU voices are partitioned between music and SFX.
///
/// Constructed at compile time, but can be swapped at runtime via
/// [`VoiceAlloc::set_layout`].
#[derive(Clone, Copy)]
pub struct VoiceLayout {
    pub music_start: u8,
    pub music_count: u8,
    pub sfx_start: u8,
    pub sfx_count: u8,
}

impl VoiceLayout {
    /// Create a layout.  `music` and `sfx` are `(start_voice, count)` pairs.
    ///
    /// ```
    /// // Voices 0..15 for music, 16..23 for SFX
    /// const LAYOUT: VoiceLayout = VoiceLayout::new((0, 16), (16, 8));
    /// ```
    pub const fn new(music: (u8, u8), sfx: (u8, u8)) -> Self {
        assert!(
            music.0 as u16 + music.1 as u16 <= 24
                && sfx.0 as u16 + sfx.1 as u16 <= 24,
            "voice ranges must fit within 24 hardware voices",
        );
        Self {
            music_start: music.0,
            music_count: music.1,
            sfx_start: sfx.0,
            sfx_count: sfx.1,
        }
    }
}

/// Allocates hardware voices from a [`VoiceLayout`].
///
/// Uses bitmasks for O(1)-ish claim/release (CTZ scan on a 24-bit mask).
pub struct VoiceAlloc {
    layout: VoiceLayout,
    music_mask: u32,
    sfx_mask: u32,
}

impl VoiceAlloc {
    pub const fn with_layout(layout: VoiceLayout) -> Self {
        Self {
            layout,
            music_mask: 0,
            sfx_mask: 0,
        }
    }

    /// Replace the current layout.  Releases all voices first.
    pub fn set_layout(&mut self, layout: VoiceLayout) {
        self.release_all();
        self.layout = layout;
    }

    /// Claim the first free music voice, or `None` if all are in use.
    pub fn claim_music(&mut self) -> Option<VoiceHw> {
        Self::claim_from_group(self.layout.music_start, self.layout.music_count, &mut self.music_mask)
    }

    /// Return a music voice to the pool.
    pub fn release_music(&mut self, voice: &VoiceHw) {
        voice.key_off();
        self.music_mask &= !(1u32 << voice.id());
    }

    /// Return a music voice to the pool without writing KEY_OFF.
    ///
    /// Returns the voice's bit mask so the caller can batch all
    /// KEY_OFF writes into a single register store.
    pub fn release_music_deferred(&mut self, voice: &VoiceHw) -> u32 {
        let bit = 1u32 << voice.id();
        self.music_mask &= !bit;
        bit
    }

    /// Claim the first free SFX voice, or `None` if all are in use.
    pub fn claim_sfx(&mut self) -> Option<VoiceHw> {
        Self::claim_from_group(self.layout.sfx_start, self.layout.sfx_count, &mut self.sfx_mask)
    }

    /// Return a SFX voice to the pool.
    pub fn release_sfx(&mut self, voice: &VoiceHw) {
        voice.key_off();
        self.sfx_mask &= !(1u32 << voice.id());
    }

    /// Stop and release every voice in both groups.
    pub fn release_all(&mut self) {
        let all = self.music_mask | self.sfx_mask;
        if all != 0 {
            super::hw::key_off_mask(all);
        }
        self.music_mask = 0;
        self.sfx_mask = 0;
    }

    fn claim_from_group(start: u8, count: u8, mask: &mut u32) -> Option<VoiceHw> {
        for i in 0..count {
            let id = start + i;
            let bit = 1u32 << id;
            if *mask & bit == 0 {
                *mask |= bit;
                return Some(VoiceHw::new(id));
            }
        }
        None
    }
}
