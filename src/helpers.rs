use core::{arch::asm, cell::{RefCell, RefMut}, fmt::Write};

use psx::{Framebuffer, TextBox};

pub struct DebugPrinter {
    fb: Framebuffer,
    ref_cell_txt: RefCell<TextBox>
}

impl DebugPrinter {
    pub fn new() -> Self {
        let mut fb = Framebuffer::default();
        let mut txt = fb.load_default_font().new_text_box((0, 8), (320, 240));
        fb.swap();
        txt.reset();
        DebugPrinter {
            fb,
            ref_cell_txt: RefCell::new(txt)
        }
    }

    pub fn print(&mut self, message: &str) {
        let mut txt = self.ref_cell_txt.borrow_mut();
        txt.reset();
        txt.write_str(message);
        self.fb.draw_sync();
        self.fb.wait_vblank();
        self.fb.swap();
    }

    pub fn write(&mut self, kek: impl Fn(RefMut<TextBox>)) {
        let mut txt = self.ref_cell_txt.borrow_mut();
        txt.reset();
        kek(txt);
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