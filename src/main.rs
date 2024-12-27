use std::env;
use std::fmt::Display;
use std::io::{BufReader, Read, Write};
use std::process::{abort, exit};

use enum_primitive_derive::Primitive;
use num_traits::FromPrimitive;
use termios::*;

const MEMORY_SIZE: usize = 1 << 16;
const KEYBOARD_STATUS: usize = 0xFE00;
const KEYBOARD_DATA: usize   = 0xFE02;
struct Memory([u16; MEMORY_SIZE]);

impl Default for Memory {
    fn default() -> Self {
        Memory([0; MEMORY_SIZE])
    }
}

impl Memory {
    fn read(&mut self, addr: u16) -> u16 {
        if addr == KEYBOARD_STATUS as u16 {
            let c = getchar();
            if c != 0 {
                self.0[KEYBOARD_STATUS] = 1 << 15;
                self.0[KEYBOARD_DATA] = c as u16;
            } else {
                self.0[KEYBOARD_STATUS] = 0;
            }
        }
        self.0[addr as usize]
    }

    fn write(&mut self, addr: u16, val: u16) {
        self.0[addr as usize] = val;
    }
}

struct Registers {
    _0: u16,
    _1: u16,
    _2: u16,
    _3: u16,
    _4: u16,
    _5: u16,
    _6: u16,
    _7: u16,
    program_count: u16,
    condition: u16,
}

impl Default for Registers {
    fn default() -> Self {
        Self {
            _0: 0, _1: 0, _2: 0, _3: 0, _4: 0, _5: 0, _6: 0, _7: 0,
            /* set the PC to starting position - 0x3000 is the default */
            program_count: 0x3000,
            /* since exactly one condition flag should be set at any given time, set the Z flag */
            condition: 0b010,
        }
    }
}

impl Display for Registers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            &format!("|{:#04x}|{:#04x}|{:#04x}|{:#04x}|{:#04x}|{:#04x}|{:#04x}|{:#04x}|{:?}|{:#04x}|",
            self._0, self._1, self._2, self._3, self._4, self._5, self._6, self._7, self.condition, self.program_count))
    }
}

impl Registers {
    #[allow(unreachable_code)]
    fn get(&self, r: u16) -> u16 {
        match r & 0x7 {
            0 => self._0,
            1 => self._1,
            2 => self._2,
            3 => self._3,
            4 => self._4,
            5 => self._5,
            6 => self._6,
            7 => self._7,
            _ => !unreachable!(),
        }
    }

    #[allow(unreachable_code)]
    fn set(&mut self, r: u16, value: u16) {
        match r & 0x7 {
            0 => self._0 = value,
            1 => self._1 = value,
            2 => self._2 = value,
            3 => self._3 = value,
            4 => self._4 = value,
            5 => self._5 = value,
            6 => self._6 = value,
            7 => self._7 = value,
            _ => !unreachable!(),
        }

        // Set the condition flag.
        self.condition = match value {
            0        => 0b010,
            0x8000.. => 0b100,
            _        => 0b001,
        };
    }

    fn next(&mut self, memory: &mut Memory) -> (u16, Option<OP>) {
        let pc = self.program_count;
        self.program_count += 1;
        let instruction = memory.read(pc);
        let operation = OP::from_u16(instruction >> 12);
        (instruction, operation)
    }
}

#[derive(Debug, Eq, PartialEq, Primitive)]
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

#[derive(Eq, PartialEq, Primitive, Debug)]
enum TRAP {
    GETC  = 0x20,  /* get character from keyboard, not echoed onto the terminal */
    OUT   = 0x21,  /* output a character */
    PUTS  = 0x22,  /* output a word string */
    IN    = 0x23,  /* get character from keyboard, echoed onto the terminal */
    PUTSP = 0x24,  /* output a byte string */
    HALT  = 0x25,  /* halt the program */
}


fn main() {
    let mut memory = Memory::default();
    let mut reg = Registers::default();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("lc3 [image-file1] ...");
        exit(2);
    }

    args.iter().skip(1).for_each(|image| {
        match read_image(&mut memory, image) {
            Ok(addr) => reg.program_count = addr,
            Err(_) => {
                println!("failed to load image: {}", image);
                exit(1);
            }
        }
    });

    // Get the terminal working such that it reads one char at a time.
    let stdin = 0;
    let mut termios = Termios::from_fd(stdin).unwrap();
    termios.c_iflag &= IGNBRK | BRKINT | PARMRK | ISTRIP | INLCR | IGNCR | ICRNL | IXON;
    termios.c_lflag &= !(ICANON | ECHO); // no echo and canonical mode
    tcsetattr(stdin, TCSANOW, &mut termios).unwrap();

    loop {
        let (instr, op) = reg.next(&mut memory);
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
                    reg.get(sr2)
                };
                reg.set(dr, reg.get(sr1).wrapping_add(value));
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
                    reg.get(sr2)
                };
                reg.set(dr, reg.get(sr1) & value);
            }
            OP::NOT => {
                /* |1001| DR| SR|111111| */
                let dr = (instr >> 9) & 0x7;
                let sr = (instr >> 6) & 0x7;
                reg.set(dr, !reg.get(sr));
            }
            OP::BR => {
                /* |0000|N|Z|P|PCoffset9| */
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                let cond_flag = (instr >> 9) & 0x7;
                if (cond_flag & reg.condition) != 0 {
                    reg.program_count = reg.program_count.wrapping_add(pc_offset);
                }
            }
            OP::JMP => {
                /* |1100|000| SR|000000| (RET when SR=7) */
                let sr = (instr >> 6) & 0x7;
                reg.program_count = reg.get(sr);
            }
            OP::JSR => {
                /*  JSR: |0100|1|  PCoffset11 | */
                /* JSRR: |0100|0|00| SR|000000| */
                let long_flag = (instr >> 11) & 1 != 0;
                reg._7 = reg.program_count;
                if long_flag {
                    let long_pc_offset = sign_extend(instr & 0x7FF, 11);
                    reg.program_count = reg.program_count.wrapping_add(long_pc_offset);
                } else {
                    let sr = (instr >> 6) & 0x7;
                    reg.program_count = reg.get(sr);
                }
            }
            OP::LD => {
                /* |0010| DR|PCoffset9| */
                let dr = (instr >> 9) & 0x7;
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                reg.set(dr, memory.read(reg.program_count.wrapping_add(pc_offset)));
            }
            OP::LDI => {
                /* |1010| DR|PCoffset9| */
                let dr = (instr >> 9) & 0x7;
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                /* add pc_offset to the current PC, look at that memory location to get the final address */
                let address = memory.read(reg.program_count.wrapping_add(pc_offset));
                // println!("LDI: M{} ({}) -> R{}", address, memory.read(address), dr);
                reg.set(dr, memory.read(address));
            }
            OP::LDR => {
                /* |0110| DR| SR|offset6| */
                let dr = (instr >> 9) & 0x7;
                let sr = (instr >> 6) & 0x7;
                let offset = sign_extend(instr & 0x3F, 6);
                reg.set(dr, memory.read(reg.get(sr).wrapping_add(offset)));
            }
            OP::LEA => {
                /* |1110| DR|PCoffset9| */
                let dr = (instr >> 9) & 0x7;
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                reg.set(dr, reg.program_count.wrapping_add(pc_offset));
            }
            OP::ST => {
                /* |0011| SR|PCoffset9| */
                let sr = (instr >> 9) & 0x7;
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                // println!("ST : R{} ({}) -> {} + {:#b}", sr, reg.get(sr), reg.program_count, pc_offset);
                memory.write(reg.program_count.wrapping_add(pc_offset), reg.get(sr));
            }
            OP::STI => {
                /* |1011| SR|PCoffset9| */
                let sr = (instr >> 9) & 0x7;
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                let address = memory.read(reg.program_count.wrapping_add(pc_offset));
                memory.write(address, reg.get(sr));
            }
            OP::STR => {
                /* |0111| SR| DR|offset6| */
                let sr = (instr >> 9) & 0x7;
                let dr = (instr >> 6) & 0x7;
                let offset = sign_extend(instr & 0x3F, 6);
                memory.write(reg.get(dr).wrapping_add(offset), reg.get(sr));
            }
            OP::TRAP => {
                /* |1111|0000|trapvec8| */
                reg._7 = reg.program_count;
                // println!("{:?}", TRAP::from_u16(instr & 0xFF));
                match TRAP::from_u16(instr & 0xFF) {
                    Some(TRAP::GETC) => {
                        reg.set(0, getchar() as u16);
                    }
                    Some(TRAP::OUT) => {
                        let mut stdout = std::io::stdout().lock();
                        stdout.write(&[reg._0 as u8]).ok();
                        stdout.flush().ok();
                    }
                    Some(TRAP::PUTS) => {
                        let mut stdout = std::io::stdout().lock();
                        let mut c = reg._0;
                        while memory.read(c) != 0 {
                            stdout.write(&[memory.read(c) as u8]).ok();
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
                        reg.set(0, c as u16);
                    }
                    Some(TRAP::PUTSP) => {
                        /* one char per byte (two bytes per word)
                           here we need to swap back to
                           big endian format */
                        let mut c = reg._0;
                        let mut stdout = std::io::stdout().lock();
                        while memory.read(c) != 0 {
                            let c1 = memory.read(c) & 0xFF;
                            stdout.write(&[c1 as u8]).ok();
                            let c2 = memory.read(c) >> 8;
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
        // println!("{}", reg);
    }

    tcsetattr(stdin, TCSANOW, &termios).unwrap();
}

fn read_image(memory: &mut Memory, image_path: &str) -> std::io::Result<u16> {
    let mut file = BufReader::new(std::fs::File::open(image_path)?);
    let mut buf = [0u8; 2];

    /* the origin tells us where in memory to place the image */
    let addr: u16 = {
        file.read_exact(&mut buf)?;
        u16::from_be_bytes(buf)
    };
    println!("Program Origin: {:#x}", addr);

    let max_offset = (MEMORY_SIZE - (addr as usize)) as u16;
    for offset in 0..max_offset {
        if let Err(err) = file.read_exact(&mut buf) {
            match err.kind() {
                std::io::ErrorKind::UnexpectedEof => {println!("EOF"); break},
                _ => return Err(err)
            }
        };
        println!("data: {:#018b}", u16::from_be_bytes(buf));
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
