pub struct MemoryCell<T>(*mut T);

impl<T> MemoryCell<T> {
    pub const fn new(address: usize) -> MemoryCell<T> {
        MemoryCell(address as *mut T)
    }

    pub fn get(&self) -> T {
        unsafe {
            self.0.read_volatile()
        }
    }

    pub fn set(&self, x: T) {
        unsafe {
            self.0.write_volatile(x)
        }
    }
}