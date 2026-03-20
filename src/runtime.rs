//! Cooperative multitasking runtime for PlayStation 1 (MIPS R3000A).
//!
//! # Overview
//!
//! Provides lightweight stackful coroutines with **explicit yielding**.
//! PSX has a single CPU core, so "multitasking" means rapidly switching
//! between tasks — each task keeps its own stack and a small set of saved
//! registers (48 bytes).  Switching only happens when a task voluntarily
//! calls [`yield_now()`].
//!
//! # Two kinds of tasks
//!
//! | Kind | Created with | Stack comes from | Lifetime |
//! |------|--------------|------------------|----------|
//! | **Static** | [`spawn()`] | Caller-provided [`TaskStack<N>`] | Infinite (audio loop, etc.) |
//! | **Dynamic** | [`spawn_dynamic()`] / [`spawn_dynamic_with_arg()`] | Built-in pool | Finite — stack auto-returned on completion |
//!
//! # Quick-start
//!
//! ```rust,ignore
//! use crate::runtime::{self, TaskStack};
//!
//! static AUDIO_STACK: TaskStack<1024> = TaskStack::new(); // 4 KiB
//!
//! fn main() {
//!     runtime::init();                            // register main as task 0
//!     runtime::spawn(audio_task, &AUDIO_STACK);   // long-lived static task
//!
//!     loop {
//!         // game logic …
//!         runtime::yield_now();   // give other tasks a turn
//!         // render …
//!     }
//! }
//!
//! extern "C" fn audio_task() {
//!     loop {
//!         // update SPU …
//!         runtime::yield_now();
//!     }
//! }
//! ```
//!
//! # Dynamic (finite) tasks
//!
//! ```rust,ignore
//! extern "C" fn compute(obj_ptr: u32) {
//!     let obj = unsafe { &mut *(obj_ptr as *mut MyObject) };
//!     // heavy work … yield_now() as needed …
//!     // when this function returns, the pool stack is freed automatically
//! }
//!
//! let handle = runtime::spawn_dynamic_with_arg(compute, ptr as u32)
//!     .expect("pool exhausted");
//! handle.join();   // cooperatively wait for completion
//! ```
//!
//! # When (not) to yield
//!
//! **Good yield points:**
//! - After `draw_sync()` and before `wait_vblank()` — CPU is idle anyway.
//! - Inside a long loop, every N iterations.
//! - While polling for an external event (`while !ready { yield_now(); }`).
//!
//! **Do NOT yield in the middle of:**
//! - A sequence of SPU / GPU / DMA register writes (another task might
//!   touch the same hardware).
//! - An update of shared state — finish writing all related fields first.
//!
//! # Memory budget
//!
//! The only knob that matters is [`POOL_CAPACITY`] × [`POOL_STACK_SIZE`].
//! Everything else is derived or negligible.
//!
//! With defaults (8 slots × 1 KiB): **≈ 8.5 KiB** out of 2 048 KiB RAM.
//!
//! # How context switching works
//!
//! Because `yield_now()` is a normal **function call**, the compiler already
//! saves caller-saved registers (t0-t9, a0-a3, v0-v1) before the call.
//! We only need to save the 12 **callee-saved** registers mandated by the
//! MIPS C ABI: s0-s7, gp, sp, fp, ra — 48 bytes per task.
//!
//! `switch_context` (26 MIPS instructions) stores those 12 registers from
//! the old task's context, loads them from the new task's context, and
//! executes `jr $ra`.  Since `$ra` now contains the *new* task's saved
//! return address, we resume exactly where that task last yielded.
//!
//! A brand-new task has `ra = task_bootstrap`, which calls the entry
//! function.  When the entry returns, `task_on_complete` marks the task
//! `Finished`, and the next `yield_now()` from another task reclaims
//! the slot (lazy cleanup — we can't free the stack while standing on it).

use core::arch::naked_asm;
use core::cell::UnsafeCell;

// ---------------------------------------------------------------------------
// Configuration
//
// POOL_CAPACITY and POOL_STACK_SIZE are the two numbers you actually tune.
// STATIC_SLOTS is a small budget for main + long-lived tasks (audio, etc.)
// that bring their own stack via TaskStack<N> and never touch the pool.
// MAX_TASKS is derived — you should never need to edit it directly.
// ---------------------------------------------------------------------------

/// Max number of dynamic tasks alive at the same time.
/// Each slot reserves `POOL_STACK_SIZE × 4` bytes of RAM.
const POOL_CAPACITY: usize = 8;

/// Per-task stack size in the pool, measured in `u32` words.
/// 256 words = 1 KiB.  Enough for shallow call chains (physics, AI).
/// Increase if a dynamic task does deep recursion or heavy formatting.
const POOL_STACK_SIZE: usize = 256;

/// Slots reserved for tasks that manage their own stacks:
///   - slot 0 is always `main`
///   - remaining slots are for `spawn()` (e.g. a permanent audio loop)
const STATIC_SLOTS: usize = 4;

/// Total task-slot array size.  Derived — do not edit.
const MAX_TASKS: usize = STATIC_SLOTS + POOL_CAPACITY;

// ---------------------------------------------------------------------------
// Core data structures
// ---------------------------------------------------------------------------

/// 48 bytes of saved register state — the only thing kept per task.
///
/// Layout is `#[repr(C)]` and the field order / offsets must match
/// the `sw` / `lw` instructions in [`switch_context`] exactly:
///
/// | offset | register(s)     |
/// |--------|-----------------|
/// |  0..28 | s0-s7 (r16-r23) |
/// |     32 | gp    (r28)     |
/// |     36 | sp    (r29)     |
/// |     40 | fp/s8 (r30)     |
/// |     44 | ra    (r31)     |
#[repr(C)]
#[derive(Clone, Copy)]
struct TaskContext {
    s: [u32; 8],
    gp: u32,
    sp: u32,
    fp: u32,
    ra: u32,
}

impl TaskContext {
    const fn zeroed() -> Self {
        Self { s: [0; 8], gp: 0, sp: 0, fp: 0, ra: 0 }
    }
}

/// Lifecycle of a task slot.
///
/// ```text
///  Empty ──spawn──► Active ──entry returns──► Finished ──collect──► Empty
///                     │                                               ▲
///                     └───────── yield / resume (stays Active) ───────┘
/// ```
#[derive(Clone, Copy, PartialEq)]
enum TaskState {
    /// Slot is free and can be claimed by `spawn*`.
    Empty,
    /// Task is running or suspended (waiting for its turn).
    Active,
    /// Entry function returned; slot awaits cleanup by [`collect_finished`].
    Finished,
}

/// One task slot.  Combines saved registers, state, and bookkeeping.
#[derive(Clone, Copy)]
struct Task {
    context: TaskContext,
    state: TaskState,
    /// Monotonically increasing tag (wrapping `u8`).  Every time a slot is
    /// reused, `generation` increments.  [`TaskHandle`] stores the generation
    /// it was born with; if they mismatch the handle knows "my task is gone".
    generation: u8,
    /// `Some(pool_index)` when the stack was allocated from the pool.
    /// On cleanup the pool slot is returned.  `None` for static tasks.
    pool_slot: Option<u8>,
}

impl Task {
    const fn empty() -> Self {
        Self {
            context: TaskContext::zeroed(),
            state: TaskState::Empty,
            generation: 0,
            pool_slot: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Pool — fixed-size allocator for task stacks
//
// Each PoolSlot holds the stack memory and a `free` flag.  Alloc scans for
// the first free slot, dealloc just flips the flag.  No fragmentation ever:
// all blocks are the same size.
// ---------------------------------------------------------------------------

/// Fixed-size stack allocator.
///
/// Stacks and free-flags are stored in **separate** arrays so that the
/// all-zero stack memory lands in `.bss` (costs nothing in the executable)
/// while only the tiny `free` array (a few bytes) lives in `.data`.
struct Pool {
    /// Stack memory — zero-initialized → `.bss`, not stored in the EXE.
    stacks: [[u32; POOL_STACK_SIZE]; POOL_CAPACITY],
    /// Per-slot availability.  `true` = free, `false` = in use.
    free: [bool; POOL_CAPACITY],
}

impl Pool {
    const fn new() -> Self {
        Self {
            stacks: [[0u32; POOL_STACK_SIZE]; POOL_CAPACITY],
            free: [true; POOL_CAPACITY],
        }
    }

    /// Claim the first free slot.  Returns `None` when pool is exhausted.
    fn alloc(&mut self) -> Option<u8> {
        for (i, is_free) in self.free.iter_mut().enumerate() {
            if *is_free {
                *is_free = false;
                return Some(i as u8);
            }
        }
        None
    }

    /// Return a slot to the pool.  The stack memory is not zeroed — it will
    /// be overwritten by the next task that claims the slot.
    fn dealloc(&mut self, idx: u8) {
        self.free[idx as usize] = true;
    }

    /// Address of the top of slot's stack (highest address, 8-byte aligned).
    /// MIPS stacks grow downward, so sp starts here and decreases.
    fn stack_top(&self, idx: u8) -> u32 {
        let stack = &self.stacks[idx as usize];
        let top = unsafe { (stack.as_ptr() as *const u32).add(POOL_STACK_SIZE) } as u32;
        top & !7 // MIPS ABI requires 8-byte stack alignment
    }
}

// ---------------------------------------------------------------------------
// Executor — round-robin scheduler over the task array
// ---------------------------------------------------------------------------

struct Executor {
    /// Slot 0 = main.  Slots 1..STATIC_SLOTS for `spawn()` tasks.
    /// Slots STATIC_SLOTS.. for `spawn_dynamic*()` tasks.
    /// (In practice any empty slot can be used by any spawn variant.)
    tasks: [Task; MAX_TASKS],
    /// Index of the task that is currently executing.
    current: usize,
}

impl Executor {
    const fn new() -> Self {
        Self { tasks: [Task::empty(); MAX_TASKS], current: 0 }
    }
}

// ---------------------------------------------------------------------------
// SyncCell — interior mutability that is Sync
//
// Sound on single-core PSX with cooperative scheduling: only one task
// executes at a time, and there is no interrupt-driven preemption of
// user code within the runtime.
// ---------------------------------------------------------------------------

struct SyncCell<T>(UnsafeCell<T>);
unsafe impl<T> Sync for SyncCell<T> {}
impl<T> SyncCell<T> {
    const fn new(val: T) -> Self { Self(UnsafeCell::new(val)) }
    fn as_ptr(&self) -> *mut T { self.0.get() }
}

// ---------------------------------------------------------------------------
// Globals (single-core ⇒ one executor, one pool)
// ---------------------------------------------------------------------------

static EXECUTOR: SyncCell<Executor> = SyncCell::new(Executor::new());
static POOL: SyncCell<Pool> = SyncCell::new(Pool::new());

// ===========================================================================
// Public types
// ===========================================================================

/// Static stack storage for a long-lived task.
///
/// Declare as a `static` and pass a reference to [`spawn()`].
/// `N` is measured in `u32` words — multiply by 4 for byte size.
///
/// ```rust,ignore
/// static MY_STACK: TaskStack<2048> = TaskStack::new(); // 8 KiB
/// runtime::spawn(my_task, &MY_STACK);
/// ```
pub struct TaskStack<const N: usize>(UnsafeCell<[u32; N]>);
unsafe impl<const N: usize> Sync for TaskStack<N> {}

impl<const N: usize> TaskStack<N> {
    pub const fn new() -> Self {
        Self(UnsafeCell::new([0u32; N]))
    }
}

/// Opaque handle to a spawned task.
///
/// Use [`is_finished()`](TaskHandle::is_finished) to poll or
/// [`join()`](TaskHandle::join) to cooperatively block until the task ends.
///
/// Internally carries a **generation counter** — if the underlying slot is
/// recycled for a completely different task, the stale handle will report
/// `is_finished() == true` (your task is long gone).
#[derive(Clone, Copy)]
pub struct TaskHandle {
    index: u8,
    generation: u8,
}

impl TaskHandle {
    /// Returns `true` when the entry function has returned
    /// (or the slot was already recycled for another task).
    pub fn is_finished(&self) -> bool {
        let task = unsafe { &(*EXECUTOR.as_ptr()).tasks[self.index as usize] };
        task.generation != self.generation || task.state != TaskState::Active
    }

    /// Cooperatively block: keep yielding until the task completes.
    ///
    /// ```rust,ignore
    /// let h = runtime::spawn_dynamic(heavy_work).unwrap();
    /// // … do other things …
    /// h.join(); // wait for heavy_work to finish
    /// ```
    pub fn join(self) {
        while !self.is_finished() {
            yield_now();
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Scan the task array (skipping slot 0 = main) for the first empty slot.
fn find_free_slot() -> Option<usize> {
    let tasks = unsafe { &(*EXECUTOR.as_ptr()).tasks };
    (1..MAX_TASKS).find(|&i| tasks[i].state == TaskState::Empty)
}

/// Read the current value of the `$gp` register.
/// Needed to initialise a new task's context so that global/static accesses
/// work correctly after a context switch.
fn read_gp() -> u32 {
    let gp: u32;
    unsafe { core::arch::asm!("move {}, $gp", out(reg) gp) };
    gp
}

/// Populate a task slot and return a handle.
///
/// Initial context setup:
///   - `s0` = entry function address (read by [`task_bootstrap`])
///   - `s1` = argument passed to entry via `$a0` (read by [`task_bootstrap`])
///   - `ra` = [`task_bootstrap`] — the trampoline that calls entry and
///     handles cleanup if entry returns
///   - `sp`, `fp` = top of the task's stack (grows downward)
///   - `gp` = copied from the spawning task
fn setup_task(idx: usize, entry: u32, arg: u32, sp: u32, pool_slot: Option<u8>) -> TaskHandle {
    let exec = unsafe { &mut *EXECUTOR.as_ptr() };
    let next_gen = exec.tasks[idx].generation.wrapping_add(1);
    exec.tasks[idx] = Task {
        context: TaskContext {
            s: [entry, arg, 0, 0, 0, 0, 0, 0],
            gp: read_gp(),
            sp,
            fp: sp,
            ra: task_bootstrap as u32,
        },
        state: TaskState::Active,
        generation: next_gen,
        pool_slot,
    };
    TaskHandle { index: idx as u8, generation: next_gen }
}

/// Lazy garbage collection: reclaim resources of finished tasks.
///
/// Called at the top of every [`yield_now()`].  Iterates the task array and
/// for each `Finished` slot (except the *current* one — we might still be
/// standing on its stack!) returns the pool stack and resets the slot.
///
/// Why "lazy"?  When a task's entry function returns, [`task_on_complete`]
/// marks it `Finished` but **cannot** free the stack because we are still
/// executing on that stack.  The actual free happens here, from a *different*
/// task's call to `yield_now()`.
fn collect_finished() {
    unsafe {
        let exec = EXECUTOR.as_ptr();
        let pool = POOL.as_ptr();
        let current = (*exec).current;

        for i in 0..MAX_TASKS {
            if i != current && (*exec).tasks[i].state == TaskState::Finished {
                if let Some(slot) = (*exec).tasks[i].pool_slot {
                    (*pool).dealloc(slot);
                }
                let prev_gen = (*exec).tasks[i].generation;
                (*exec).tasks[i] = Task::empty();
                // Preserve generation so that stale TaskHandles that reference
                // this slot can still detect that their task has ended.
                (*exec).tasks[i].generation = prev_gen;
            }
        }
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Initialise the cooperative runtime.
///
/// Must be called **once**, before any `spawn*` or `yield_now`.
/// The current execution context (your `main` function) is registered as
/// task 0.  It keeps its existing stack — nothing is allocated.
pub fn init() {
    let exec = unsafe { &mut *EXECUTOR.as_ptr() };
    exec.tasks[0] = Task {
        context: TaskContext::zeroed(),
        state: TaskState::Active,
        generation: 1,
        pool_slot: None,
    };
    exec.current = 0;
}

/// Spawn a **long-lived** task with a caller-provided static stack.
///
/// Use this for tasks that run for the entire lifetime of the program
/// (e.g. an audio loop).  The stack is never freed.
///
/// Entry must be `extern "C" fn()` — typically an infinite loop.
/// If it returns, the task halts silently.
///
/// Panics if there is no free task slot.
///
/// ```rust,ignore
/// static AUDIO_STACK: TaskStack<1024> = TaskStack::new(); // 4 KiB
/// runtime::spawn(audio_task, &AUDIO_STACK);
/// ```
pub fn spawn<const N: usize>(entry: extern "C" fn(), stack: &TaskStack<N>) {
    let stack_top = unsafe { (stack.0.get() as *mut u32).add(N) } as u32;
    let sp = stack_top & !7;
    let idx = find_free_slot().expect("no free task slots");
    setup_task(idx, entry as u32, 0, sp, None);
}

/// Spawn a **finite** task whose stack comes from the built-in pool.
///
/// When the entry function returns, the pool stack is automatically
/// reclaimed (on the next [`yield_now()`] from another task).
///
/// Returns `None` if the pool is exhausted (no free stacks) or there
/// are no free task slots.
///
/// ```rust,ignore
/// let handle = runtime::spawn_dynamic(my_short_task).unwrap();
/// handle.join();  // block until done
/// ```
pub fn spawn_dynamic(entry: extern "C" fn()) -> Option<TaskHandle> {
    let pool = unsafe { &mut *POOL.as_ptr() };
    let slot = pool.alloc()?;
    let sp = pool.stack_top(slot);
    Some(setup_task(find_free_slot()?, entry as u32, 0, sp, Some(slot)))
}

/// Spawn a finite task and pass it a `u32` argument.
///
/// The argument arrives as the first parameter of the entry function.
/// Typical use: cast a pointer to `u32` to hand each task its own data.
///
/// ```rust,ignore
/// extern "C" fn compute(ptr: u32) {
///     let data = unsafe { &mut *(ptr as *mut MyData) };
///     // ...
/// }
///
/// let ptr = &mut my_data as *mut MyData as u32;
/// let h = runtime::spawn_dynamic_with_arg(compute, ptr).unwrap();
/// h.join();
/// ```
pub fn spawn_dynamic_with_arg(entry: extern "C" fn(u32), arg: u32) -> Option<TaskHandle> {
    let pool = unsafe { &mut *POOL.as_ptr() };
    let slot = pool.alloc()?;
    let sp = pool.stack_top(slot);
    Some(setup_task(find_free_slot()?, entry as u32, arg, sp, Some(slot)))
}

/// Yield execution to the next runnable task.
///
/// Scheduling is **round-robin**: the next Active task after the current one
/// (wrapping around) gets the CPU.  If no other task is active, returns
/// immediately (no-op).
///
/// Also performs lazy cleanup: finished tasks are collected and their pool
/// stacks returned before the switch happens.
///
/// Cost: ~30 clock cycles for the context switch itself (12 stores +
/// 12 loads + jump).  At 33 MHz that is under 1 µs — call it freely.
pub fn yield_now() {
    unsafe {
        let exec = EXECUTOR.as_ptr();

        collect_finished();

        let old = (*exec).current;
        let mut new = old;
        for i in 1..MAX_TASKS {
            let idx = (old + i) % MAX_TASKS;
            if (*exec).tasks[idx].state == TaskState::Active {
                new = idx;
                break;
            }
        }

        if old == new {
            return;
        }

        (*exec).current = new;
        switch_context(
            core::ptr::addr_of_mut!((*exec).tasks[old].context),
            core::ptr::addr_of!((*exec).tasks[new].context),
        );
    }
}

/// Returns the index of the currently executing task.
/// Main is always 0.
pub fn current_task() -> usize {
    unsafe { (*EXECUTOR.as_ptr()).current }
}

// ===========================================================================
// MIPS I (R3000A) assembly
// ===========================================================================

/// Entry trampoline — every new task starts here.
///
/// When [`switch_context`] first resumes a freshly spawned task, the
/// restored register state is:
///
///   `$s0` = entry function address
///   `$s1` = argument (0 if none)
///   `$ra` = address of this trampoline
///   `$sp` = top of the task's stack
///
/// Execution:
///
/// 1. `jalr $s0`        — jump to the entry function.
///    `move $a0, $s1`   — (delay slot) pass the argument in `$a0`.
/// 2. If entry returns → fall through to `jal task_on_complete`.
/// 3. `task_on_complete` marks the task Finished and yields forever.
/// 4. The infinite `j` loop is a safety net (should never be reached).
#[unsafe(naked)]
unsafe extern "C" fn task_bootstrap() {
    naked_asm! {
        ".set noreorder",
        "jalr $s0",
        "move $a0, $s1",
        "jal task_on_complete",
        "nop",
        "1: j 1b",
        "nop",
        ".set reorder",
    }
}

/// Runs when a task's entry function returns.
///
/// Marks the current task as [`Finished`](TaskState::Finished) and then
/// yields in a loop.  The scheduler will never resume this task (it is no
/// longer Active), and the next [`yield_now()`] from another task will
/// reclaim the slot via [`collect_finished`].
///
/// `#[no_mangle]` is required so that the `jal task_on_complete` instruction
/// in [`task_bootstrap`] can reference this symbol at link time.
#[unsafe(no_mangle)]
extern "C" fn task_on_complete() {
    unsafe {
        let exec = EXECUTOR.as_ptr();
        let idx = (*exec).current;
        (*exec).tasks[idx].state = TaskState::Finished;
    }
    loop {
        yield_now();
    }
}

/// Low-level cooperative context switch (26 instructions).
///
/// 1. **Save** the 12 callee-saved registers from the *current* task into
///    the [`TaskContext`] pointed to by `$a0`.
/// 2. **Load** them from the *new* task's context pointed to by `$a1`.
/// 3. **`jr $ra`** — `$ra` now holds the new task's saved return address,
///    so we resume right where that task last called `yield_now()` (or, for
///    a brand-new task, at [`task_bootstrap`]).
///
/// ## R3000A load-delay hazard
///
/// On MIPS I a loaded register **cannot be used in the very next
/// instruction**.  That is why `$ra` is loaded *before* `$fp`:
///
/// ```asm
/// lw $ra, 44($a1)    # load ra
/// lw $fp, 40($a1)    # ← fills the load-delay slot for $ra
/// jr $ra             # $ra is now safe to use
/// nop                # branch-delay slot
/// ```
#[unsafe(naked)]
unsafe extern "C" fn switch_context(
    _old: *mut TaskContext,
    _new: *const TaskContext,
) {
    naked_asm! {
        ".set noreorder",
        ".set noat",

        // ---- save current task's callee-saved registers ----
        "sw $s0,  0($a0)",
        "sw $s1,  4($a0)",
        "sw $s2,  8($a0)",
        "sw $s3, 12($a0)",
        "sw $s4, 16($a0)",
        "sw $s5, 20($a0)",
        "sw $s6, 24($a0)",
        "sw $s7, 28($a0)",
        "sw $gp, 32($a0)",
        "sw $sp, 36($a0)",
        "sw $fp, 40($a0)",
        "sw $ra, 44($a0)",

        // ---- load new task's callee-saved registers ----
        "lw $s0,  0($a1)",
        "lw $s1,  4($a1)",
        "lw $s2,  8($a1)",
        "lw $s3, 12($a1)",
        "lw $s4, 16($a1)",
        "lw $s5, 20($a1)",
        "lw $s6, 24($a1)",
        "lw $s7, 28($a1)",
        "lw $gp, 32($a1)",
        "lw $sp, 36($a1)",
        "lw $ra, 44($a1)",     // load $ra first …
        "lw $fp, 40($a1)",     // … this fills $ra's load-delay slot

        // ---- resume new task ----
        "jr $ra",
        "nop",                 // branch-delay slot

        ".set at",
        ".set reorder",
    }
}
