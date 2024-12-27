use num_traits::FromPrimitive;

use crate::OP;
use crate::memory::Memory;

pub struct Registers {
    pub r0: u16,
    pub r1: u16,
    pub r2: u16,
    pub r3: u16,
    pub r4: u16,
    pub r5: u16,
    pub r6: u16,
    pub r7: u16,
    pub program_count: u16,
    pub condition: u16,
}

impl Default for Registers {
    fn default() -> Self {
        Self {
            r0: 0, r1: 0, r2: 0, r3: 0, r4: 0, r5: 0, r6: 0, r7: 0,
            /* set the PC to starting position - 0x3000 is the default */
            program_count: 0x3000,
            /* since exactly one condition flag should be set at any given time, set the Z flag */
            condition: 0b010,
        }
    }
}

// impl Display for Registers {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_str(
//             &format!("|{:#04x}|{:#04x}|{:#04x}|{:#04x}|{:#04x}|{:#04x}|{:#04x}|{:#04x}|{:?}|{:#04x}|",
//                      self._0, self._1, self._2, self._3, self._4, self._5, self._6, self._7, self.condition, self.program_count))
//     }
// }

impl Registers {
    #[allow(unreachable_code)]
    pub fn get(&self, r: u16) -> u16 {
        match r & 0x7 {
            0 => self.r0,
            1 => self.r1,
            2 => self.r2,
            3 => self.r3,
            4 => self.r4,
            5 => self.r5,
            6 => self.r6,
            7 => self.r7,
            _ => !unreachable!(),
        }
    }

    #[allow(unreachable_code)]
    pub fn set(&mut self, r: u16, value: u16) {
        match r & 0x7 {
            0 => self.r0 = value,
            1 => self.r1 = value,
            2 => self.r2 = value,
            3 => self.r3 = value,
            4 => self.r4 = value,
            5 => self.r5 = value,
            6 => self.r6 = value,
            7 => self.r7 = value,
            _ => !unreachable!(),
        }

        // Set the condition flag.
        self.condition = match value {
            0        => 0b010,
            0x8000.. => 0b100,
            _        => 0b001,
        };
    }

    pub fn next(&mut self, memory: &mut Memory) -> (u16, Option<OP>) {
        let pc = self.program_count;
        self.program_count += 1;
        let instruction = memory.read(pc);
        let operation = OP::from_u16(instruction >> 12);
        (instruction, operation)
    }
}