/// PS1 SPU hardware reverb configuration.
///
/// The SPU reverb unit operates on a circular buffer at the end of sound RAM,
/// applying reflection, comb, and all-pass filters to simulate acoustic
/// environments. All address fields use N×8 byte addressing, consistent with
/// other SPU address registers.
///
/// Preset values sourced from psx-spx (no$psx) verified register dumps.
/// Reference: <https://psx-spx.consoledev.net/soundprocessingunitspu/#reverb-examples>

/// Complete reverb register set for the SPU.
///
/// The 30 filter registers at `$1F801DC0`–`$1F801DFA` configure the reflection,
/// comb, and all-pass filter stages. [`buffer_start`](Self::buffer_start) sets
/// the reverb work area origin via `$1F801DA2`; the work area extends from
/// there to the end of the 512 KB sound RAM.
#[derive(Clone, Copy)]
pub struct ReverbConfig {
    /// Reverb work area start (N×8 byte addressing).
    /// The buffer spans from here to `0x10000` (end of SPU RAM address space).
    pub buffer_start: u16,

    // --- All-pass filter offset / buffer sizes (N×8) ---
    pub d_apf1: u16,
    pub d_apf2: u16,

    // --- Volume coefficients (signed 16-bit, scale N / 0x8000) ---
    pub v_iir: u16,
    pub v_comb1: u16,
    pub v_comb2: u16,
    pub v_comb3: u16,
    pub v_comb4: u16,
    pub v_wall: u16,
    pub v_apf1: u16,
    pub v_apf2: u16,

    // --- Same-side reflection head addresses (N×8, relative) ---
    pub m_l_same: u16,
    pub m_r_same: u16,

    // --- Comb filter input addresses, group 1–2 ---
    pub m_l_comb1: u16,
    pub m_r_comb1: u16,
    pub m_l_comb2: u16,
    pub m_r_comb2: u16,

    // --- Same-side reflection tail addresses ---
    pub d_l_same: u16,
    pub d_r_same: u16,

    // --- Different-side reflection head addresses ---
    pub m_l_diff: u16,
    pub m_r_diff: u16,

    // --- Comb filter input addresses, group 3–4 ---
    pub m_l_comb3: u16,
    pub m_r_comb3: u16,
    pub m_l_comb4: u16,
    pub m_r_comb4: u16,

    // --- Different-side reflection tail addresses ---
    pub d_l_diff: u16,
    pub d_r_diff: u16,

    // --- All-pass filter head addresses ---
    pub m_l_apf1: u16,
    pub m_r_apf1: u16,
    pub m_l_apf2: u16,
    pub m_r_apf2: u16,
}

impl ReverbConfig {
    /// Pack the 30 filter registers in hardware write order
    /// (`$1F801DC0`–`$1F801DFA`).
    pub(super) const fn as_registers(&self) -> [u16; 30] {
        [
            self.d_apf1,    self.d_apf2,
            self.v_iir,     self.v_comb1,   self.v_comb2,
            self.v_comb3,   self.v_comb4,
            self.v_wall,    self.v_apf1,    self.v_apf2,
            self.m_l_same,  self.m_r_same,
            self.m_l_comb1, self.m_r_comb1,
            self.m_l_comb2, self.m_r_comb2,
            self.d_l_same,  self.d_r_same,
            self.m_l_diff,  self.m_r_diff,
            self.m_l_comb3, self.m_r_comb3,
            self.m_l_comb4, self.m_r_comb4,
            self.d_l_diff,  self.d_r_diff,
            self.m_l_apf1,  self.m_r_apf1,
            self.m_l_apf2,  self.m_r_apf2,
        ]
    }

    // -----------------------------------------------------------------------
    // Presets — verified against psx-spx (no$psx) register dumps
    // -----------------------------------------------------------------------

    /// Small room. Short, subtle reverb with fast decay.
    /// No different-side reflection — mono-ish reverb.
    pub const ROOM: Self = Self {
        buffer_start: 0xFB28,
        d_apf1: 0x007D, d_apf2: 0x005B,
        v_iir:  0x6D80, v_comb1: 0x54B8, v_comb2: 0xBED0,
        v_comb3: 0x0000, v_comb4: 0x0000,
        v_wall: 0xBA80, v_apf1: 0x5800, v_apf2: 0x5300,
        m_l_same: 0x04D6, m_r_same: 0x0333,
        m_l_comb1: 0x03F0, m_r_comb1: 0x0227,
        m_l_comb2: 0x0374, m_r_comb2: 0x01EF,
        d_l_same: 0x0334, d_r_same: 0x01B5,
        m_l_diff: 0x0000, m_r_diff: 0x0000,
        m_l_comb3: 0x0000, m_r_comb3: 0x0000,
        m_l_comb4: 0x0000, m_r_comb4: 0x0000,
        d_l_diff: 0x0000, d_r_diff: 0x0000,
        m_l_apf1: 0x01B4, m_r_apf1: 0x0136,
        m_l_apf2: 0x00B8, m_r_apf2: 0x005C,
    };

    /// Small studio. Tight, controlled reverb.
    pub const STUDIO_SMALL: Self = Self {
        buffer_start: 0xFC1A,
        d_apf1: 0x0033, d_apf2: 0x0025,
        v_iir:  0x70F0, v_comb1: 0x4FA8, v_comb2: 0xBCE0,
        v_comb3: 0x4410, v_comb4: 0xC0F0,
        v_wall: 0x9C00, v_apf1: 0x5280, v_apf2: 0x4EC0,
        m_l_same: 0x03E4, m_r_same: 0x031B,
        m_l_comb1: 0x03A4, m_r_comb1: 0x02AF,
        m_l_comb2: 0x0372, m_r_comb2: 0x0266,
        d_l_same: 0x031C, d_r_same: 0x025D,
        m_l_diff: 0x025C, m_r_diff: 0x018E,
        m_l_comb3: 0x022F, m_r_comb3: 0x0135,
        m_l_comb4: 0x01D2, m_r_comb4: 0x00B7,
        d_l_diff: 0x018F, d_r_diff: 0x00B5,
        m_l_apf1: 0x00B4, m_r_apf1: 0x0080,
        m_l_apf2: 0x004C, m_r_apf2: 0x0026,
    };

    /// Medium studio. Good general-purpose reverb.
    pub const STUDIO_MEDIUM: Self = Self {
        buffer_start: 0xF6FA,
        d_apf1: 0x00B1, d_apf2: 0x007F,
        v_iir:  0x70F0, v_comb1: 0x4FA8, v_comb2: 0xBCE0,
        v_comb3: 0x4510, v_comb4: 0xBEF0,
        v_wall: 0xB4C0, v_apf1: 0x5280, v_apf2: 0x4EC0,
        m_l_same: 0x0904, m_r_same: 0x076B,
        m_l_comb1: 0x0824, m_r_comb1: 0x065F,
        m_l_comb2: 0x07A2, m_r_comb2: 0x0616,
        d_l_same: 0x076C, d_r_same: 0x05ED,
        m_l_diff: 0x05EC, m_r_diff: 0x042E,
        m_l_comb3: 0x050F, m_r_comb3: 0x0305,
        m_l_comb4: 0x0462, m_r_comb4: 0x02B7,
        d_l_diff: 0x042F, d_r_diff: 0x0265,
        m_l_apf1: 0x0264, m_r_apf1: 0x01B2,
        m_l_apf2: 0x0100, m_r_apf2: 0x0080,
    };

    /// Large studio. Warm, spacious reverb.
    pub const STUDIO_LARGE: Self = Self {
        buffer_start: 0xF203,
        d_apf1: 0x00E3, d_apf2: 0x00A9,
        v_iir:  0x6F60, v_comb1: 0x4FA8, v_comb2: 0xBCE0,
        v_comb3: 0x4510, v_comb4: 0xBEF0,
        v_wall: 0xA680, v_apf1: 0x5680, v_apf2: 0x52C0,
        m_l_same: 0x0DFB, m_r_same: 0x0B58,
        m_l_comb1: 0x0D09, m_r_comb1: 0x0A3C,
        m_l_comb2: 0x0BD9, m_r_comb2: 0x0973,
        d_l_same: 0x0B59, d_r_same: 0x08DA,
        m_l_diff: 0x08D9, m_r_diff: 0x05E9,
        m_l_comb3: 0x07EC, m_r_comb3: 0x04B0,
        m_l_comb4: 0x06EF, m_r_comb4: 0x03D2,
        d_l_diff: 0x05EA, d_r_diff: 0x031D,
        m_l_apf1: 0x031C, m_r_apf1: 0x0238,
        m_l_apf2: 0x0154, m_r_apf2: 0x00AA,
    };

    /// Concert hall. Long tail, dramatic reverb.
    pub const HALL: Self = Self {
        buffer_start: 0xEA44,
        d_apf1: 0x01A5, d_apf2: 0x0139,
        v_iir:  0x6000, v_comb1: 0x5000, v_comb2: 0x4C00,
        v_comb3: 0xB800, v_comb4: 0xBC00,
        v_wall: 0xC000, v_apf1: 0x6000, v_apf2: 0x5C00,
        m_l_same: 0x15BA, m_r_same: 0x11BB,
        m_l_comb1: 0x14C2, m_r_comb1: 0x10BD,
        m_l_comb2: 0x11BC, m_r_comb2: 0x0DC1,
        d_l_same: 0x11C0, d_r_same: 0x0DC3,
        m_l_diff: 0x0DC0, m_r_diff: 0x09C1,
        m_l_comb3: 0x0BC4, m_r_comb3: 0x07C1,
        m_l_comb4: 0x0A00, m_r_comb4: 0x06CD,
        d_l_diff: 0x09C2, d_r_diff: 0x05C1,
        m_l_apf1: 0x05C0, m_r_apf1: 0x041A,
        m_l_apf2: 0x0274, m_r_apf2: 0x013A,
    };

    /// Space echo. Very long delay — the classic PS1 BIOS reverb.
    pub const SPACE: Self = Self {
        buffer_start: 0xE128,
        d_apf1: 0x033D, d_apf2: 0x0231,
        v_iir:  0x7E00, v_comb1: 0x5000, v_comb2: 0xB400,
        v_comb3: 0xB000, v_comb4: 0x4C00,
        v_wall: 0xB000, v_apf1: 0x6000, v_apf2: 0x5400,
        m_l_same: 0x1ED6, m_r_same: 0x1A31,
        m_l_comb1: 0x1D14, m_r_comb1: 0x183B,
        m_l_comb2: 0x1BC2, m_r_comb2: 0x16B2,
        d_l_same: 0x1A32, d_r_same: 0x15EF,
        m_l_diff: 0x15EE, m_r_diff: 0x1055,
        m_l_comb3: 0x1334, m_r_comb3: 0x0F2D,
        m_l_comb4: 0x11F6, m_r_comb4: 0x0C5D,
        d_l_diff: 0x1056, d_r_diff: 0x0AE1,
        m_l_apf1: 0x0AE0, m_r_apf1: 0x07A2,
        m_l_apf2: 0x0464, m_r_apf2: 0x0232,
    };
}
