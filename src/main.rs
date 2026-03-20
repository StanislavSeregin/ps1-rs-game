#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

mod common;
pub mod runtime;

use core::cell::UnsafeCell;
use psx::gpu::VideoMode;
use psx::{dprintln, Framebuffer};
use runtime::TaskStack;

// ---------------------------------------------------------------------------
// Shared state — safe on single-core PSX with cooperative scheduling:
// only one task is running at any given time, so no data races.
// ---------------------------------------------------------------------------

struct Shared<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Shared<T> {}

impl<T> Shared<T> {
    const fn new(val: T) -> Self {
        Self(UnsafeCell::new(val))
    }
    fn as_ptr(&self) -> *mut T {
        self.0.get()
    }
}

// Task 1 intermediate results
struct FibState {
    n: u32,
    value: u32,
}

// Task 2 intermediate results
struct PrimeState {
    checked: u32,
    count: u32,
    last_prime: u32,
}

static FIB_STATE: Shared<FibState> = Shared::new(FibState { n: 0, value: 0 });
static PRIME_STATE: Shared<PrimeState> = Shared::new(PrimeState {
    checked: 1,
    count: 0,
    last_prime: 0,
});

// 4 KiB stack per coroutine (1024 × 4 bytes)
static FIB_STACK: TaskStack<1024> = TaskStack::new();
static PRIME_STACK: TaskStack<1024> = TaskStack::new();

// ---------------------------------------------------------------------------
// Entry point — task 0 (rendering)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
fn main() {
    runtime::init();
    runtime::spawn(fib_task, &FIB_STACK);
    runtime::spawn(prime_task, &PRIME_STACK);

    let buf0 = (0, 0);
    let buf1 = (0, 240);
    let res = (320, 240);
    let txt_offset = (0, 8);
    let mut fb = Framebuffer::new(buf0, buf1, res, VideoMode::NTSC, None).unwrap();
    let font = fb.load_default_font();
    let mut txt = font.new_text_box(txt_offset, res);

    let mut frame: u32 = 0;

    loop {
        txt.reset();
        frame += 1;

        let fib = unsafe { &*FIB_STATE.as_ptr() };
        let primes = unsafe { &*PRIME_STATE.as_ptr() };

        dprintln!(txt, "Cooperative Multitasking Demo");
        dprintln!(txt, "----------------------------");
        dprintln!(txt, "");
        dprintln!(txt, "Coroutine 1  Fibonacci");
        dprintln!(txt, "  fib({}) = {}", fib.n, fib.value);
        dprintln!(txt, "");
        dprintln!(txt, "Coroutine 2  Prime counter");
        dprintln!(txt, "  checked up to: {}", primes.checked);
        dprintln!(txt, "  primes found:  {}", primes.count);
        dprintln!(txt, "  last prime:    {}", primes.last_prime);
        dprintln!(txt, "");
        dprintln!(txt, "Render loop");
        dprintln!(txt, "  frame: {}", frame);

        fb.draw_sync();
        runtime::yield_now();
        fb.wait_vblank();
        fb.swap();
    }
}

// ---------------------------------------------------------------------------
// Task 1 — Fibonacci sequence (yields after each step)
// ---------------------------------------------------------------------------

extern "C" fn fib_task() {
    let mut a: u32 = 0;
    let mut b: u32 = 1;
    let mut n: u32 = 0;

    loop {
        unsafe {
            let state = &mut *FIB_STATE.as_ptr();
            state.n = n;
            state.value = a;
        }

        let next = a.wrapping_add(b);
        a = b;
        b = next;
        n += 1;

        if n >= 48 {
            a = 0;
            b = 1;
            n = 0;
        }

        runtime::yield_now();
    }
}

// ---------------------------------------------------------------------------
// Task 2 — Prime counter (yields every batch of candidates)
// ---------------------------------------------------------------------------

extern "C" fn prime_task() {
    let mut count: u32 = 0;
    let mut last_prime: u32 = 0;
    let mut n: u32 = 2;

    loop {
        if is_prime(n) {
            count += 1;
            last_prime = n;
        }

        unsafe {
            let state = &mut *PRIME_STATE.as_ptr();
            state.checked = n;
            state.count = count;
            state.last_prime = last_prime;
        }

        n += 1;

        if n % 50 == 0 {
            runtime::yield_now();
        }
    }
}

fn is_prime(n: u32) -> bool {
    if n < 2 {
        return false;
    }
    if n < 4 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    let mut i = 3u32;
    while i <= n / i {
        if n % i == 0 {
            return false;
        }
        i += 2;
    }
    true
}
