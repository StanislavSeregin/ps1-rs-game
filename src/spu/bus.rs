use core::cell::UnsafeCell;

use super::music::Pitch;
use super::sample::SampleId;

/// Command sent from game code to the audio engine.
#[derive(Clone, Copy)]
pub enum Command {
    /// Trigger a one-shot sound effect on the SFX voice pool.
    PlaySfx(SampleId, Pitch),
    /// Signal the engine to abort the current `play_pattern` / `play_patterns`.
    Interrupt,
    /// Change the sequencer BPM (1 beat = 4 rows).
    SetBpm(u16),
    /// Key-off all voices and halt playback.
    StopAll,
}

/// Fixed-capacity ring buffer for inter-coroutine commands.
///
/// Safe on single-core PS1 with cooperative scheduling: only one
/// coroutine accesses the bus at any given moment (between yields).
pub struct CommandBus<const CAP: usize = 16> {
    buffer: [Option<Command>; CAP],
    head: usize,
    tail: usize,
}

impl<const CAP: usize> CommandBus<CAP> {
    pub const fn new() -> Self {
        Self {
            buffer: [None; CAP],
            head: 0,
            tail: 0,
        }
    }

    /// Enqueue a command. Returns `false` if the buffer is full.
    pub fn send(&mut self, cmd: Command) -> bool {
        let next = (self.head + 1) % CAP;
        if next == self.tail {
            return false;
        }
        self.buffer[self.head] = Some(cmd);
        self.head = next;
        true
    }

    /// Dequeue the oldest command, or `None` if the buffer is empty.
    pub fn poll(&mut self) -> Option<Command> {
        if self.tail == self.head {
            return None;
        }
        let cmd = self.buffer[self.tail].take();
        self.tail = (self.tail + 1) % CAP;
        cmd
    }
}

/// Read-only snapshot of the engine's playback state.
#[derive(Clone, Copy)]
pub struct AudioStatus {
    pub playing: bool,
    pub current_row: u16,
    pub current_pattern: u16,
}

impl AudioStatus {
    pub const fn idle() -> Self {
        Self {
            playing: false,
            current_row: 0,
            current_pattern: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Global instances — single-core cooperative scheduling makes this safe.
// ---------------------------------------------------------------------------

struct SyncCell<T>(UnsafeCell<T>);
unsafe impl<T> Sync for SyncCell<T> {}
impl<T> SyncCell<T> {
    const fn new(val: T) -> Self {
        Self(UnsafeCell::new(val))
    }
    fn as_ptr(&self) -> *mut T {
        self.0.get()
    }
}

static CMD_BUS: SyncCell<CommandBus> = SyncCell::new(CommandBus::new());
static AUDIO_STATUS: SyncCell<AudioStatus> = SyncCell::new(AudioStatus::idle());

/// Send a command to the audio engine (call from game coroutine).
pub fn send_command(cmd: Command) -> bool {
    unsafe { (*CMD_BUS.as_ptr()).send(cmd) }
}

/// Read the current audio engine status (call from game coroutine).
pub fn audio_status() -> AudioStatus {
    unsafe { *AUDIO_STATUS.as_ptr() }
}

/// Poll one command from the bus (call from engine coroutine).
pub(super) fn poll_command() -> Option<Command> {
    unsafe { (*CMD_BUS.as_ptr()).poll() }
}

/// Update the global status snapshot (call from engine coroutine).
pub(super) fn set_status(status: AudioStatus) {
    unsafe { *AUDIO_STATUS.as_ptr() = status; }
}
