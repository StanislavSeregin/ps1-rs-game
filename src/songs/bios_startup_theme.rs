use crate::runtime::TaskStack;
use crate::spu::*;
use crate::spu::reverb::ReverbConfig;
use AdsrMode::*;
use AdsrDir::*;

crate::sound_project! {
    pub PROJECT {
        samples: [
            BELL   => crate::include_bytes_skip!("samples/file_all.adpcm", 0, 13500),
            SWOOSH => crate::include_bytes_skip!("samples/file_all.adpcm", 13600, 8400),
            SWEEP  => crate::include_bytes_skip!("samples/file_all.adpcm", 22000),
        ],
        layout: VoiceLayout::new((0, 24), (0, 0)),
    }
}

const ADSR_SWEEP: Adsr = Adsr::new()
    .attack(Exp, 0x0A)
    .decay(0x04)
    .sustain_level(0x0A)
    .sustain(Exp, Decrease, 0x12)
    .release(Exp, 0x0E);

const ADSR_BELL: Adsr = Adsr::new()
    .decay(0x0A)
    .sustain_level(0x0A)
    .sustain(Exp, Decrease, 0x0F)
    .release(Exp, 0x08);

const ADSR_SWOOSH: Adsr = Adsr::new()
    .sustain_level(0x0F)
    .sustain(Exp, Decrease, 0x10)
    .release(Exp, 0x0C);

const ADSR_LOW_BELL: Adsr = Adsr::new()
    .attack(Exp, 0x04)
    .decay(0x0A)
    .sustain_level(0x0A)
    .sustain(Exp, Decrease, 0x13)
    .release(Exp, 0x10);

const BPM: u16 = 300;

const fn sweep(pitch: Pitch, pan: Pan) -> Cell {
    Cell::note_vol(PROJECT::SWEEP, pitch, Volume::HALF)
        .with_pan(pan)
        .with_adsr(ADSR_SWEEP)
}

const fn sweep_bass(pitch: Pitch, pan: Pan) -> Cell {
    Cell::note_vol(PROJECT::SWEEP, pitch, Volume(0x1400))
        .with_pan(pan)
        .with_adsr(ADSR_SWEEP)
}

const fn sweep_quiet(pitch: Pitch, pan: Pan) -> Cell {
    Cell::note_vol(PROJECT::SWEEP, pitch, Volume::QUARTER)
        .with_pan(pan)
        .with_adsr(ADSR_SWEEP)
}

const fn bell(pitch: Pitch, pan: Pan) -> Cell {
    Cell::note_vol(PROJECT::BELL, pitch, Volume(0x2800))
        .with_pan(pan)
        .with_adsr(ADSR_BELL)
}

const fn swoosh_cell(pitch: Pitch, pan: Pan) -> Cell {
    Cell::note(PROJECT::SWOOSH, pitch).with_pan(pan).with_adsr(ADSR_SWOOSH)
}

const fn low_bell(pitch: Pitch, pan: Pan) -> Cell {
    Cell::note(PROJECT::BELL, pitch).with_pan(pan).with_adsr(ADSR_LOW_BELL)
}

const SWEEP_PAT: Pattern<128> = Pattern::new()
    .set(0, 20, sweep_bass(Pitch(0x0200), Pan(-16)))
    .set(0, 21, sweep_bass(Pitch(0x0203), Pan(16)))
    .set(0,  0, sweep(Pitch(0x0400), Pan::LEFT))
    .set(0,  1, sweep(Pitch(0x0403), Pan::RIGHT))
    .set(0,  2, sweep_quiet(Pitch(0x0800), Pan(-32)))
    .set(0,  3, sweep_quiet(Pitch(0x0806), Pan(32)))
    .set(48, 20, sweep_bass(Pitch(0x0155), Pan(-16)))
    .set(48, 21, sweep_bass(Pitch(0x0158), Pan(16)))
    .set(48,  0, sweep(Pitch(0x02AB), Pan::LEFT))
    .set(48,  1, sweep(Pitch(0x02AE), Pan::RIGHT))
    .set(48,  2, sweep_quiet(Pitch(0x0555), Pan(-32)))
    .set(48,  3, sweep_quiet(Pitch(0x0558), Pan(32)));

const BELL_PAT: Pattern<128> = Pattern::new()
    .set(30, 6,  bell(Pitch(0x0C00), Pan(-48)))
    .set(33, 7,  bell(Pitch(0x1000), Pan(40)))
    .set(36, 8,  bell(Pitch(0x1200), Pan(-32)))
    .set(39, 9,  bell(Pitch(0x1800), Pan(56)))
    .set(43, 10, bell(Pitch(0x1000), Pan(-24)))
    .set(45, 11, bell(Pitch(0x0800), Pan(16)))
    .set(46, 12, bell(Pitch(0x0A00), Pan(-40)))
    .set(47, 13, bell(Pitch(0x0C00), Pan(48)))
    .set(50, 6,  bell(Pitch(0x1000), Pan(-36)))
    .set(51, 7,  bell(Pitch(0x1400), Pan(44)))
    .set(52, 8,  bell(Pitch(0x1800), Pan(-20)))
    .set(64,  6,  bell(Pitch(0x1000), Pan(-56)))
    .set(65,  7,  bell(Pitch(0x1400), Pan(52)))
    .set(65,  8,  bell(Pitch(0x1800), Pan(24)))
    .set(66,  9,  bell(Pitch(0x2000), Pan(-44)))
    .set(66,  10, bell(Pitch(0x0C00), Pan(36)))
    .set(67,  11, bell(Pitch(0x1400), Pan(-28)))
    .set(67,  12, bell(Pitch(0x1200), Pan(48)))
    .set(68,  13, bell(Pitch(0x1000), Pan(-52)))
    .set(69,  14, bell(Pitch(0x1800), Pan(32)))
    .set(69,  15, bell(Pitch(0x1400), Pan(-16)))
    .set(71,  6,  bell(Pitch(0x1000), Pan(44)))
    .set(72,  7,  bell(Pitch(0x0C00), Pan(-8)))
    .set(74,  8,  bell(Pitch(0x1200), Pan(-48)))
    .set(84,  18, low_bell(Pitch(0x0180), Pan(-40)))
    .set(84,  19, low_bell(Pitch(0x0186), Pan(40)));

const SWOOSH_PAT: Pattern<128> = Pattern::new()
    .set(64, 16, swoosh_cell(Pitch(0x0600), Pan(-32)))
    .set(70, 17, swoosh_cell(Pitch(0x0C00), Pan(32)));

pub static MUSIC_STACK: TaskStack<2048> = TaskStack::new();

pub extern "C" fn music_task() {
    let mut e = Engine::take().unwrap();
    e.load_project(&PROJECT::DATA);
    e.set_bpm(BPM);

    e.enable_reverb(&ReverbConfig::SPACE, 0x5000, 0x3000);
    e.set_channel_reverb(0..22, true);

    e.play_patterns(&[&SWEEP_PAT, &BELL_PAT, &SWOOSH_PAT]);
}
