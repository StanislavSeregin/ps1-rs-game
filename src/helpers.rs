use core::{arch::asm, fmt::Write};
use psx::{Framebuffer, TextBox};
use arrayvec::ArrayString;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

/// Allow to write messages to display
pub struct DisplayLogger {
    fb: Framebuffer,
    txt: TextBox,
    storage: ConstGenericRingBuffer::<ArrayString::<64>, 28>
}

impl DisplayLogger {
    /// Init display logger
    pub fn new() -> Self {
        let mut fb = Framebuffer::default();
        let mut txt = fb.load_default_font().new_text_box((0, 8), (320, 240));
        fb.swap();
        txt.reset();
        let storage = ConstGenericRingBuffer::<ArrayString<64>, 28>::new();
        DisplayLogger { fb, txt, storage }
    }

    /// Write sized string to display
    /// # Examples
    /// ```
    /// let mut logger = DisplayLogger::new();
    /// let mut arr_str = arrayvec::ArrayString::<64>::new();
    /// let i = 123;
    /// write!(arr_str, "Some text: {i}");
    /// logger.log(arr_str);
    /// ```
    pub fn log(&mut self, message: ArrayString<64>) {
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

/// Write formatted string to display
/// # Examples
/// ```
/// let mut logger = DisplayLogger::new();
/// let i = 123;
/// write_to_display!(logger, "Some text: {i}");
/// ```
macro_rules! write_to_display {
    ($logger:ident, $fmt:literal) => {
        let mut buffer = arrayvec::ArrayString::<64>::new();
        write!(buffer, $fmt).unwrap();
        $logger.log(buffer);
    };
}