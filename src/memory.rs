use super::getchar;

pub const MEMORY_SIZE: usize = 1 << 16;
const KEYBOARD_STATUS_ADDR: usize = 0xFE00;
const KEYBOARD_DATA_ADDR: usize   = 0xFE02;

pub struct Memory([u16; MEMORY_SIZE]);

impl Default for Memory {
    fn default() -> Self {
        Memory([0; MEMORY_SIZE])
    }
}

impl Memory {
    pub fn read(&mut self, addr: u16) -> u16 {
        if addr == KEYBOARD_STATUS_ADDR as u16 {
            let c = getchar();
            if c != 0 {
                self.0[KEYBOARD_STATUS_ADDR] = 1 << 15;
                self.0[KEYBOARD_DATA_ADDR] = c as u16;
            } else {
                self.0[KEYBOARD_STATUS_ADDR] = 0;
            }
        }
        self.0[addr as usize]
    }

    pub fn write(&mut self, addr: u16, val: u16) {
        self.0[addr as usize] = val;
    }
}