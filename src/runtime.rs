//! Cooperative multitasking runtime for PSX (MIPS R3000A).
//!
//! Provides lightweight stackful coroutines with explicit yielding.
//! Only callee-saved registers are preserved on context switch (48 bytes per task),
//! making it significantly cheaper than full preemptive threading with a complete TCB.
//!
//! # Architecture
//!
//! Each task has its own stack and a small saved-register context. When a task
//! calls `yield_now()`, the runtime saves its callee-saved registers (s0-s7,
//! gp, sp, fp, ra), picks the next runnable task round-robin, and restores
//! that task's registers. The `jr $ra` at the end of the context switch resumes
//! the new task exactly where it last yielded.
//!
//! The calling task (task 0) is registered automatically by `init()` — its
//! context is captured on the first `yield_now()` call and restored when
//! switching back.

use core::arch::naked_asm;
use core::cell::UnsafeCell;

const MAX_TASKS: usize = 4;

/// Saved callee-preserved registers for cooperative context switching.
///
/// Layout must match the assembly offsets in `switch_context` exactly.
///   0: s0,  4: s1,  8: s2, 12: s3, 16: s4, 20: s5, 24: s6, 28: s7
///  32: gp, 36: sp, 40: fp, 44: ra
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
        Self {
            s: [0; 8],
            gp: 0,
            sp: 0,
            fp: 0,
            ra: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct Task {
    context: TaskContext,
    active: bool,
}

impl Task {
    const fn empty() -> Self {
        Self {
            context: TaskContext::zeroed(),
            active: false,
        }
    }
}

struct Executor {
    tasks: [Task; MAX_TASKS],
    current: usize,
    count: usize,
}

impl Executor {
    const fn new() -> Self {
        Self {
            tasks: [Task::empty(); MAX_TASKS],
            current: 0,
            count: 0,
        }
    }
}

/// Interior-mutable cell that is `Sync`. Sound on single-core PSX
/// with cooperative (non-preemptive) scheduling — only one task
/// accesses the cell at any given time.
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

static EXECUTOR: SyncCell<Executor> = SyncCell::new(Executor::new());

/// Static stack storage for a spawned task.
///
/// `N` is measured in `u32` words — multiply by 4 for byte size.
/// For example, `TaskStack<2048>` = 8 KiB.
pub struct TaskStack<const N: usize>(UnsafeCell<[u32; N]>);
unsafe impl<const N: usize> Sync for TaskStack<N> {}

impl<const N: usize> TaskStack<N> {
    pub const fn new() -> Self {
        Self(UnsafeCell::new([0u32; N]))
    }
}

/// Initialize the cooperative runtime.
///
/// The current execution context is registered as task 0.
/// Must be called once before any `spawn` or `yield_now`.
pub fn init() {
    let exec = unsafe { &mut *EXECUTOR.as_ptr() };
    exec.tasks[0].active = true;
    exec.current = 0;
    exec.count = 1;
}

/// Spawn a new cooperative task.
///
/// `entry` — the task's entry point (should contain an infinite loop).
/// If it returns, the task halts silently.
///
/// `stack` — dedicated stack storage. Must outlive the task (use a `static`).
pub fn spawn<const N: usize>(entry: extern "C" fn(), stack: &TaskStack<N>) {
    let exec = unsafe { &mut *EXECUTOR.as_ptr() };
    let idx = exec.count;
    assert!(idx < MAX_TASKS);

    let stack_top = unsafe { (stack.0.get() as *mut u32).add(N) } as u32;
    let sp = stack_top & !7; // MIPS requires 8-byte stack alignment

    let gp: u32;
    unsafe { core::arch::asm!("move {}, $gp", out(reg) gp) };

    exec.tasks[idx] = Task {
        context: TaskContext {
            // s0 holds the entry function pointer — read by task_bootstrap
            s: [entry as u32, 0, 0, 0, 0, 0, 0, 0],
            gp,
            sp,
            fp: sp,
            ra: task_bootstrap as u32,
        },
        active: true,
    };
    exec.count += 1;
}

/// Yield execution to the next runnable task (round-robin).
///
/// If only one task is active, returns immediately (no-op).
/// Otherwise saves the current task's callee-saved registers and
/// restores the next task's registers, effectively resuming it.
pub fn yield_now() {
    unsafe {
        let exec = EXECUTOR.as_ptr();
        let old = (*exec).current;
        let count = (*exec).count;

        let mut new = old;
        for i in 1..=count {
            let idx = (old + i) % count;
            if (*exec).tasks[idx].active {
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

/// Returns the index of the currently executing task (0 = initial/main task).
pub fn current_task() -> usize {
    unsafe { (*EXECUTOR.as_ptr()).current }
}

// ---------------------------------------------------------------------------
// MIPS assembly routines
// ---------------------------------------------------------------------------

/// Trampoline for newly spawned tasks.
///
/// On first resume, `switch_context` loads s0 = entry function pointer and
/// ra = this trampoline's address. `jalr $s0` calls the entry function;
/// if it ever returns, we spin (the task is effectively dead).
#[unsafe(naked)]
unsafe extern "C" fn task_bootstrap() {
    naked_asm! {
        ".set noreorder",
        "jalr $s0",
        "nop",
        "1: j 1b",
        "nop",
        ".set reorder",
    }
}

/// Saves callee-saved registers to `old` context, loads them from `new`,
/// and resumes the new task via `jr $ra`.
///
/// Note: `$ra` is loaded before `$fp` to satisfy the MIPS I (R3000A)
/// load-delay hazard — a loaded register cannot be used in the very
/// next instruction.
#[unsafe(naked)]
unsafe extern "C" fn switch_context(
    _old: *mut TaskContext,
    _new: *const TaskContext,
) {
    naked_asm! {
        ".set noreorder",
        ".set noat",

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
        "lw $ra, 44($a1)",
        "lw $fp, 40($a1)",

        "jr $ra",
        "nop",

        ".set at",
        ".set reorder",
    }
}
