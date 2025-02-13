use core::{arch::asm, cell::RefCell, fmt::Write};

use arrayvec::ArrayString;
use psx::{Framebuffer, TextBox};

use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

pub struct DebugPrinter {
    fb: Framebuffer,
    txt: TextBox,
    storage: ConstGenericRingBuffer::<ArrayString::<64>, 28>
}

impl DebugPrinter {
    pub fn new() -> Self {
        let mut fb = Framebuffer::default();
        let mut txt = fb.load_default_font().new_text_box((0, 8), (320, 240));
        fb.swap();
        txt.reset();
        DebugPrinter {
            fb,
            txt,
            storage: ConstGenericRingBuffer::<ArrayString<64>, 28>::new()
        }
    }

    pub fn print(&mut self, message: ArrayString<64>) {
        self.txt.reset();
        self.storage.push(message);
        for s in self.storage.iter() {
            self.txt.write_str(s);
            self.txt.newline();
        }

        self.fb.draw_sync();
        self.fb.wait_vblank();
        self.fb.swap();
    }
}

pub fn delay() {
    for _ in 1..10000000 {
        unsafe {
            asm!("nop");
        }
    }
}

#[macro_export]
macro_rules! print_debug {
    ($debug_printer:ident, $fmt:literal) => {
        let mut buffer = arrayvec::ArrayString::<64>::new();
        write!(buffer, $fmt).unwrap();
        $debug_printer.print(buffer);
    };
}