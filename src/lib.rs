use std::fs::File;
use std::io::{BufReader, Read};

use enum_primitive_derive::Primitive;
use num_traits::FromPrimitive;

mod io;
mod memory;
mod registers;

#[derive(Eq, PartialEq, Primitive)]
enum OP {
    BR   = 0b0000,  /* branch */
    ADD  = 0b0001,  /* add  */
    LD   = 0b0010,  /* load */
    ST   = 0b0011,  /* store */
    JSR  = 0b0100,  /* jump register */
    AND  = 0b0101,  /* bitwise and */
    LDR  = 0b0110,  /* load register */
    STR  = 0b0111,  /* store register */
    RTI  = 0b1000,  /* unused */
    NOT  = 0b1001,  /* bitwise not */
    LDI  = 0b1010,  /* load indirect */
    STI  = 0b1011,  /* store indirect */
    JMP  = 0b1100,  /* jump */
    RES  = 0b1101,  /* reserved (unused) */
    LEA  = 0b1110,  /* load effective address */
    TRAP = 0b1111,  /* execute trap */
}

#[derive(Eq, PartialEq, Primitive)]
enum TRAP {
    GETC  = 0x20,  /* get character from keyboard, not echoed onto the terminal */
    OUT   = 0x21,  /* output a character */
    PUTS  = 0x22,  /* output a word string */
    IN    = 0x23,  /* get character from keyboard, echoed onto the terminal */
    PUTSP = 0x24,  /* output a byte string */
    HALT  = 0x25,  /* halt the program */
}

#[derive(Eq, PartialEq)]
pub enum STATUS {
    Halted,
    Continue,
    SoftInterrupt,
    HardInterrupt,
}

#[cfg(target_family = "wasm")]
#[derive(Default)]
#[wasm_bindgen::prelude::wasm_bindgen]
pub struct VM {
    pub halted: bool,
    memory: memory::Memory,
    registers: registers::Registers,
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
impl VM {
    #[cfg(target_family = "wasm")]
    #[wasm_bindgen::prelude::wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let mut vm = VM::default();
        let image = include_bytes!("../hello_world.obj");
        if let Ok(addr) = read_image(&mut vm.memory, BufReader::new(&image[..])) {
            vm.registers.program_count = addr;
        }
        vm
    }

    /// Run until execution is interrupted or halted.
    /// This will return true for a "SoftInterrupt" (polling for user input)
    /// and false for a "HardInterrupt" (blocking for user input or halted)
    pub fn run_wasm(&mut self) -> bool {
        while !self.halted {
            match self.step() {
                STATUS::Halted => {self.halted = true;},
                STATUS::Continue => {},
                STATUS::SoftInterrupt => return true,
                STATUS::HardInterrupt => return false,
            }
        }
        false
    }

    pub fn load_wasm(&mut self, image: &[u8]) -> bool {
        let mut new_memory = memory::Memory::default();
        let mut new_registers = registers::Registers::default();

        match read_image(&mut new_memory, BufReader::new(&image[..])) {
            Ok(addr) => new_registers.program_count = addr,
            Err(_) => return false,
        }

        self.memory = new_memory;
        self.registers = new_registers;
        self.halted = false;
        true
    }
}

#[cfg(target_family = "unix")]
#[derive(Default)]
pub struct VM {
    pub halted: bool,
    memory: memory::Memory,
    registers: registers::Registers,
}


impl VM {
    pub fn load_file(&mut self, path: &str) -> std::io::Result<()> {
        let file = BufReader::new(File::open(path)?);
        match read_image(&mut self.memory, file) {
            Ok(addr) => {
                self.registers.program_count = addr;
                Ok(())
            },
            Err(e) => Err(e),
        }
    }

    pub fn run(&mut self) {
        while !self.halted {
            match self.step() {
                STATUS::Halted => {self.halted = false;},
                _ => {}
            }
        }
    }

    pub fn step(&mut self) -> STATUS {
        let (instr, op) = self.registers.next(&mut self.memory);
        let op = match op {
            Some(op) => op,
            None => {
                #[cfg(target_family = "unix")]
                println!("invalid operation");
                return STATUS::Halted
            }
        };

        match op {
            OP::ADD => {
                /* |0001| DR|SR1|0|00|SR2|
                   |0001| DR|SR1|1| IMM5 | */
                let dr = (instr >> 9) & 0x7;
                let sr1 = (instr >> 6) & 0x7;
                let imm_flag = (instr >> 5) & 1 != 0;

                let value = if imm_flag {
                    sign_extend(instr & 0x1F, 5)
                } else {
                    let sr2 = instr & 0x7;
                    self.registers.get(sr2)
                };
                self.registers.set(dr, self.registers.get(sr1).wrapping_add(value));
            }
            OP::AND => {
                /* |0001| DR|SR1|0|00|SR2|
                   |0001| DR|SR1|1| IMM5 | */
                let dr = (instr >> 9) & 0x7;
                let sr1 = (instr >> 6) & 0x7;
                let imm_flag = (instr >> 5) & 1 != 0;

                let value = if imm_flag {
                    sign_extend(instr & 0x1F, 5)
                } else {
                    let sr2 = instr & 0x7;
                    self.registers.get(sr2)
                };
                self.registers.set(dr, self.registers.get(sr1) & value);
            }
            OP::NOT => {
                /* |1001| DR| SR|111111| */
                let dr = (instr >> 9) & 0x7;
                let sr = (instr >> 6) & 0x7;
                self.registers.set(dr, !self.registers.get(sr));
            }
            OP::BR => {
                /* |0000|N|Z|P|PCoffset9| */
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                let cond_flag = (instr >> 9) & 0x7;
                if (cond_flag & self.registers.condition) != 0 {
                    self.registers.program_count =
                        self.registers.program_count.wrapping_add(pc_offset);
                }
            }
            OP::JMP => {
                /* |1100|000| SR|000000| (RET when SR=7) */
                let sr = (instr >> 6) & 0x7;
                self.registers.program_count = self.registers.get(sr);
            }
            OP::JSR => {
                /*  JSR: |0100|1|  PCoffset11 | */
                /* JSRR: |0100|0|00| SR|000000| */
                let long_flag = (instr >> 11) & 1 != 0;
                self.registers.r7 = self.registers.program_count;
                if long_flag {
                    let long_pc_offset = sign_extend(instr & 0x7FF, 11);
                    self.registers.program_count =
                        self.registers.program_count.wrapping_add(long_pc_offset);
                } else {
                    let sr = (instr >> 6) & 0x7;
                    self.registers.program_count = self.registers.get(sr);
                }
            }
            OP::LD => {
                /* |0010| DR|PCoffset9| */
                let dr = (instr >> 9) & 0x7;
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                let address = self.registers.program_count.wrapping_add(pc_offset);
                self.registers.set(dr, self.memory.read(address));
            }
            OP::LDI => {
                /* |1010| DR|PCoffset9| */
                let dr = (instr >> 9) & 0x7;
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                /* add pc_offset to the current PC, look at that memory location to get the final address */
                let address = self.memory.read(self.registers.program_count.wrapping_add(pc_offset));
                self.registers.set(dr, self.memory.read(address));
            }
            OP::LDR => {
                /* |0110| DR| SR|offset6| */
                let dr = (instr >> 9) & 0x7;
                let sr = (instr >> 6) & 0x7;
                let offset = sign_extend(instr & 0x3F, 6);
                let address = self.registers.get(sr).wrapping_add(offset);
                self.registers.set(dr, self.memory.read(address));
            }
            OP::LEA => {
                /* |1110| DR|PCoffset9| */
                let dr = (instr >> 9) & 0x7;
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                self.registers.set(dr, self.registers.program_count.wrapping_add(pc_offset));
            }
            OP::ST => {
                /* |0011| SR|PCoffset9| */
                let sr = (instr >> 9) & 0x7;
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                self.memory.write(
                    self.registers.program_count.wrapping_add(pc_offset),
                    self.registers.get(sr)
                );
            }
            OP::STI => {
                /* |1011| SR|PCoffset9| */
                let sr = (instr >> 9) & 0x7;
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                let address = self.memory.read(self.registers.program_count.wrapping_add(pc_offset));
                self.memory.write(address, self.registers.get(sr));
            }
            OP::STR => {
                /* |0111| SR| DR|offset6| */
                let sr = (instr >> 9) & 0x7;
                let dr = (instr >> 6) & 0x7;
                let offset = sign_extend(instr & 0x3F, 6);
                self.memory.write(
                    self.registers.get(dr).wrapping_add(offset),
                    self.registers.get(sr)
                );
            }
            OP::TRAP => {
                /* |1111|0000|trapvec8| */
                self.registers.r7 = self.registers.program_count;
                match TRAP::from_u16(instr & 0xFF) {
                    Some(TRAP::GETC) => {
                        let c = io::get_char();
                        if c == 0 {
                            // If we get a null character, we roll back the
                            // instruction and suspend program execution to
                            // await user input.
                            self.registers.program_count -= 1;
                            return STATUS::HardInterrupt;
                        } else {
                            self.registers.set(0, c as u16);
                        }
                    }
                    Some(TRAP::OUT) => {
                        io::put_char(self.registers.r0 as u8);
                    }
                    Some(TRAP::PUTS) => {
                        let mut c = self.registers.r0;
                        while self.memory.read(c) != 0 {
                            io::put_char(self.memory.read(c) as u8);
                            c += 1;
                        }
                    }
                    Some(TRAP::IN) => {
                        #[cfg(target_family = "unix")]
                        println!("Enter a character: ");

                        let c = io::get_char();
                        if c == 0 {
                            // If we get a null character, we roll back the
                            // instruction and suspend program execution to
                            // await user input.
                            self.registers.program_count -= 1;
                            return STATUS::HardInterrupt;
                        } else {
                            io::put_char(c);
                            self.registers.set(0, c as u16);
                        }
                    }
                    Some(TRAP::PUTSP) => {
                        /* one char per byte (two bytes per word)
                           here we need to swap back to
                           big endian format */
                        let mut c = self.registers.r0;
                        while self.memory.read(c) != 0 {
                            let c1 = self.memory.read(c) & 0xFF;
                            io::put_char(c1 as u8);
                            let c2 = self.memory.read(c) >> 8;
                            if c2 != 0 { io::put_char(c2 as u8); };
                            c += 1;
                        }
                    }
                    Some(TRAP::HALT) => {
                        #[cfg(target_family = "unix")]
                        println!("HALT");

                        return STATUS::Halted;
                    }
                    None => {
                        #[cfg(target_family = "unix")]
                        println!("Unknown TRAP");
                        return STATUS::Halted;
                    }
                }
            }
            OP::RES => {
                #[cfg(target_family = "unix")]
                println!("Invalid operation: RESERVED");
                return STATUS::Halted;
            }
            OP::RTI => {
                #[cfg(target_family = "unix")]
                println!("Invalid operation: RTI");
                return STATUS::Halted;
            }
        };

        #[cfg(target_family = "wasm")]
        if self.memory.kbstatus() != 0 {
            return STATUS::SoftInterrupt;
        }
        STATUS::Continue
    }
}


fn read_image(memory: &mut memory::Memory, mut image: impl Read) -> std::io::Result<u16> {
    let mut buf = [0u8; 2];

    /* the origin tells us where in memory to place the image */
    let addr: u16 = {
        image.read_exact(&mut buf)?;
        u16::from_be_bytes(buf)
    };

    let max_offset = (memory::MEMORY_SIZE - (addr as usize)) as u16;
    for offset in 0..max_offset {
        if let Err(err) = image.read_exact(&mut buf) {
            match err.kind() {
                std::io::ErrorKind::UnexpectedEof => break,
                _ => return Err(err)
            }
        };
        memory.write(addr + offset, u16::from_be_bytes(buf));
    }

    Ok(addr)
}

fn sign_extend(orig: u16, bit_count: u8) -> u16 {
    let mut x = orig;
    if ((x >> (bit_count - 1)) & 1) == 1 {
        x |= 0xFFFF << bit_count;
    }
    x
}
