pub const MEMORY_SIZE: usize = 1 << 16;
const KEYBOARD_STATUS_ADDR: usize = 0xFE00;
const KEYBOARD_DATA_ADDR: usize   = 0xFE02;

/// KEYBOARD_CHECK_ADDR is an address I am custom defining.
/// It records if the program has checked the KEYBOARD_STATUS
/// address. This is useful for the WASM code to determine
/// when to suspend execution to await user input.
const KEYBOARD_CHECK_ADDR: usize = 0xFE04;

pub struct Memory([u16; MEMORY_SIZE]);

impl Default for Memory {
    fn default() -> Self {
        Memory([0; MEMORY_SIZE])
    }
}

impl Memory {
    pub fn read(&mut self, addr: u16) -> u16 {
        if addr == KEYBOARD_STATUS_ADDR as u16 {
            self.0[KEYBOARD_CHECK_ADDR] = 1;
            let c = super::io::get_char();
            if c != 0 {
                self.0[KEYBOARD_STATUS_ADDR] = 1 << 15;
                self.0[KEYBOARD_DATA_ADDR] = c as u16;
            } else {
                self.0[KEYBOARD_STATUS_ADDR] = 0;
            }
        } else {
            self.0[KEYBOARD_CHECK_ADDR] = 0;
        }
        self.0[addr as usize]
    }

    pub fn write(&mut self, addr: u16, val: u16) {
        self.0[addr as usize] = val;
    }

    #[allow(dead_code)]
    pub fn kbstatus(&self) -> u16 {
        self.0[KEYBOARD_CHECK_ADDR]
    }
}