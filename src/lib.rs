use std::io::{BufReader, Read, Write};
use std::process::{abort};

use enum_primitive_derive::Primitive;
use num_traits::FromPrimitive;

pub mod memory;
pub mod registers;


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

#[derive(Default)]
pub struct VM {
    memory: memory::Memory,
    registers: registers::Registers,
}

impl VM {

    pub fn load_file(&mut self, path: &str) -> std::io::Result<()> {
        match read_image(&mut self.memory, path) {
            Ok(addr) => {
                self.registers.program_count = addr;
                Ok(())
            },
            Err(e) => Err(e),
        }
    }

    pub fn run(&mut self) {
        loop {
            let (instr, op) = self.registers.next(&mut self.memory);
            let op = match op {
                Some(op) => op,
                None => {
                    println!("invalid operation");
                    break
                }
            };

            // println!("instruction ({:x}) {:?} - {:#018b}", reg.program_count - 1, op, instr);
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
                    // println!("LDI: M{} ({}) -> R{}", address, memory.read(address), dr);
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
                    // println!("ST : R{} ({}) -> {} + {:#b}", sr, reg.get(sr), reg.program_count, pc_offset);
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
                    // println!("{:?}", TRAP::from_u16(instr & 0xFF));
                    match TRAP::from_u16(instr & 0xFF) {
                        Some(TRAP::GETC) => {
                            self.registers.set(0, getchar() as u16);
                        }
                        Some(TRAP::OUT) => {
                            let mut stdout = std::io::stdout().lock();
                            stdout.write(&[self.registers.r0 as u8]).ok();
                            stdout.flush().ok();
                        }
                        Some(TRAP::PUTS) => {
                            let mut stdout = std::io::stdout().lock();
                            let mut c = self.registers.r0;
                            while self.memory.read(c) != 0 {
                                stdout.write(&[self.memory.read(c) as u8]).ok();
                                c += 1;
                            }
                            stdout.flush().ok();
                        }
                        Some(TRAP::IN) => {
                            println!("Enter a character: ");
                            let c = getchar();
                            let mut stdout = std::io::stdout().lock();
                            stdout.write(&[c]).ok();
                            stdout.flush().ok();
                            self.registers.set(0, c as u16);
                        }
                        Some(TRAP::PUTSP) => {
                            /* one char per byte (two bytes per word)
                               here we need to swap back to
                               big endian format */
                            let mut c = self.registers.r0;
                            let mut stdout = std::io::stdout().lock();
                            while self.memory.read(c) != 0 {
                                let c1 = self.memory.read(c) & 0xFF;
                                stdout.write(&[c1 as u8]).ok();
                                let c2 = self.memory.read(c) >> 8;
                                if c2 != 0 { stdout.write(&[c2 as u8]).ok(); };
                                c += 1;
                            }
                            stdout.flush().ok();
                        }
                        Some(TRAP::HALT) => {
                            println!("HALT");
                            break
                        }
                        None => {
                            println!("Unknown TRAP");
                        }
                    }
                }
                OP::RES => { abort() }
                OP::RTI => { abort() }
            }
        }
    }
}


fn read_image(memory: &mut memory::Memory, image_path: &str) -> std::io::Result<u16> {
    let mut file = BufReader::new(std::fs::File::open(image_path)?);
    let mut buf = [0u8; 2];

    /* the origin tells us where in memory to place the image */
    let addr: u16 = {
        file.read_exact(&mut buf)?;
        u16::from_be_bytes(buf)
    };
    // println!("Program Origin: {:#x}", addr);

    let max_offset = (memory::MEMORY_SIZE - (addr as usize)) as u16;
    for offset in 0..max_offset {
        if let Err(err) = file.read_exact(&mut buf) {
            match err.kind() {
                std::io::ErrorKind::UnexpectedEof => break,
                _ => return Err(err)
            }
        };
        // println!("data: {:#018b}", u16::from_be_bytes(buf));
        memory.write(addr + offset, u16::from_be_bytes(buf));
    }

    Ok(addr)
}

fn getchar() -> u8 {
    std::io::stdin()
        .bytes()
        .next()
        .and_then(|result| result.ok())
        .unwrap_or(0)
}

fn sign_extend(orig: u16, bit_count: u8) -> u16 {
    let mut x = orig;
    if ((x >> (bit_count - 1)) & 1) == 1 {
        x |= 0xFFFF << bit_count;
    }
    x
}