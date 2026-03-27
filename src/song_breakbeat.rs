use crate::runtime::TaskStack;
use crate::spu::*;
use crate::spu::reverb::ReverbConfig;

const KICK: SampleId = SampleId(0);
const SNARE: SampleId = SampleId(1);
const HAT: SampleId = SampleId(2);

pub const PROJECT: SoundProject<3> = SoundProject {
    samples: [
        crate::include_vag!("../samples/kick.vag"),
        crate::include_vag!("../samples/snare.vag"),
        crate::include_vag!("../samples/hat.vag"),
    ],
    layout: VoiceLayout::new((0, 16), (16, 8)),
};

const BPM: u16 = 100;

const CELL_KICK: Cell = Cell::note(KICK, Pitch(0x1000));
const CELL_KICK_HALF: Cell = Cell::note_vol(KICK, Pitch(0x1000), Volume::HALF);
const CELL_SNARE: Cell = Cell::note(SNARE, Pitch(0x1000));
const CELL_HAT_QUART: Cell = Cell::note_vol(HAT, Pitch(0x1000), Volume::QUARTER);
const CELL_HAT_HALF: Cell = Cell::note_vol(HAT, Pitch(0x1000), Volume::HALF);
const CELL_HAT: Cell = Cell::note(HAT, Pitch(0x1000));

const HAT_PAT: Pattern<16> = Pattern::new()
    .set(0,  0, CELL_HAT_HALF)
    .set(2,  0, CELL_HAT_QUART)
    .set(4,  0, CELL_HAT)
    .set(6,  0, CELL_HAT_QUART)
    .set(8,  0, CELL_HAT_HALF)
    .set(10, 0, CELL_HAT_QUART)
    .set(12, 0, CELL_HAT)
    .set(14, 0, CELL_HAT_QUART);

const KICK_SNARE_1: Pattern<16> = Pattern::new()
    .set(0,  1, CELL_KICK)
    .set(2,  1, CELL_KICK_HALF)
    .set(4,  2, CELL_SNARE)
    .set(10, 1, CELL_KICK)
    .set(12, 2, CELL_SNARE);

const KICK_SNARE_2: Pattern<16> = Pattern::new()
    .set(0,  1, CELL_KICK)
    .set(2,  1, CELL_KICK_HALF)
    .set(4,  2, CELL_SNARE)
    .set(10, 1, CELL_KICK)
    .set(11, 1, CELL_KICK_HALF)
    .set(12, 2, CELL_SNARE);

pub static MUSIC_STACK: TaskStack<2048> = TaskStack::new();

pub extern "C" fn music_task() {
    let mut e = Engine::take().unwrap();
    e.load_project(&PROJECT);

    e.enable_reverb(&ReverbConfig::HALL, 0x7FFF, 0x5000);
    e.set_channel_reverb(0, true);
    e.set_channel_reverb(1, true);
    e.set_channel_reverb(2, true);

    e.set_bpm(BPM);

    loop {
        e.play_patterns(&[&HAT_PAT, &KICK_SNARE_1]);
        e.play_patterns(&[&HAT_PAT, &KICK_SNARE_2]);
    }
}
