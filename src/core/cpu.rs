
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::driver;
use super::error::{Result, Error};
use super::op::{Reg, Op};
use super::timer::Timer;

pub struct Cpu {
    pc: u16,
    sp: u8,
    i: u16,
    v: [u8; Self::REG_COUNT],
    ram: [u8; Self::RAM_BYTES],
    vram: [bool; Self::VRAM_BYTES],
    stack: [u16; Self::MAX_STACK_DEPTH],
    display_driver: Option<Box<dyn driver::Display>>,
    input_driver: Option<Box<dyn driver::Input>>,
    timer: Timer,
}

impl Cpu {
    pub const LOAD_OFFSET: usize = 0x200;
    pub const REG_COUNT: usize = 0x10;
    pub const RAM_BYTES: usize = 0x1000;
    pub const MAX_STACK_DEPTH: usize = 0x20;

    pub const MAX_REG: usize = 0x0f;
    pub const INDEX_REG: usize = 0x00;
    pub const FLAG_REG: usize = 0x0f;

    pub const DISPLAY_WIDTH: usize = 0x40;
    pub const DISPLAY_HEIGHT: usize = 0x20;

    pub const VRAM_BYTES: usize = Self::DISPLAY_WIDTH * Self::DISPLAY_HEIGHT;

    const FONT_SPRITES_BYTES: usize = 0x50;
    const FONT_SPRITES_RAM_START: usize = 0x0;
    const FONT_SPRITES_RAM_END: usize = 0x50;
    const FONT_SPRITE_BYTES_PER: usize = 0x05;

    const FONT_SPRITES: [u8; Self::FONT_SPRITES_BYTES] = [
        0xf0, 0x90, 0x90, 0x90, 0xf0,   /* 0 */
        0x20, 0x60, 0x20, 0x20, 0x70,   /* 1 */
        0xf0, 0x10, 0xf0, 0x80, 0xf0,   /* 2 */
        0xf0, 0x10, 0xf0, 0x10, 0xf0,   /* 3 */
        0x90, 0x90, 0xf0, 0x10, 0x10,   /* 4 */
        0xf0, 0x80, 0xf0, 0x10, 0xf0,   /* 5 */
        0xf0, 0x80, 0xf0, 0x90, 0xf0,   /* 6 */
        0xf0, 0x10, 0x20, 0x40, 0x40,   /* 7 */
        0xf0, 0x90, 0xf0, 0x90, 0xf0,   /* 8 */
        0xf0, 0x90, 0xf0, 0x10, 0xf0,   /* 9 */
        0xf0, 0x90, 0xf0, 0x90, 0x90,   /* A */
        0xe0, 0x90, 0xe0, 0x90, 0xe0,   /* B */
        0xf0, 0x80, 0x80, 0x80, 0xf0,   /* C */
        0xe0, 0x90, 0x90, 0x90, 0xe0,   /* D */
        0xf0, 0x80, 0xf0, 0x80, 0xf0,   /* E */
        0xf0, 0x80, 0xf0, 0x80, 0x80,   /* F */
    ];

    pub fn new() -> Self {
        let mut ram = [0xff; Self::RAM_BYTES];

        ram[Self::FONT_SPRITES_RAM_START..Self::FONT_SPRITES_RAM_END]
            .copy_from_slice(&Self::FONT_SPRITES);

        Cpu {
            pc: 0x0000,
            sp: 0x00,
            i: 0x0000,
            v: [0x00; Self::REG_COUNT],
            ram: ram,
            vram: [false; Self::VRAM_BYTES],
            stack: [0x0000; Self::MAX_STACK_DEPTH],
            display_driver: None,
            input_driver: None,
            timer: Timer::new(),
        }
    }

    pub fn load(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > self.ram.len() - Self::LOAD_OFFSET {
            Err(Error::LoadFailure)
        } else {
            let load_end = Self::LOAD_OFFSET + data.len();
            self.ram[Self::LOAD_OFFSET..load_end].copy_from_slice(data);
            self.pc = Self::LOAD_OFFSET as u16;
            Ok(())
        }
    }

    pub fn set_display_driver(&mut self, driver: Option<Box<dyn driver::Display>>) {
        self.display_driver = driver;
    }

    pub fn set_sound_driver(&mut self, driver: Option<Box<dyn driver::Sound>>) {
        let d = Arc::clone(&self.timer.sound_driver);
        let mut d = d.lock().unwrap();
        *d = driver;
    }

    pub fn set_input_driver(&mut self, driver: Option<Box<dyn driver::Input>>) {
        self.input_driver = driver;
    }

    pub fn tick(&mut self) -> Result<()> {
        let opcode = self.fetch()?;
        let op = Op::decode(opcode)
            .ok_or_else(|| Error::BadInstruction)?;
        self.exec(op)
    }

    pub fn fetch(&self) -> Result<u16> {
        if self.pc as usize > self.ram.len() - 1 {
            Err(Error::PrefetchAbort)
        } else {
            let h = self.ram[self.pc as usize] as u16;
            let l = self.ram[(self.pc + 1) as usize] as u16;
            Ok((h << 8) | l)
        }
    }

    pub fn exec(&mut self, op: Op) -> Result<()> {
        self.pc += 2;

        match op {
            Op::Sys(_) => Err(Error::UnimplementedOp(op)),
            Op::Cls => {
                for elem in self.vram.iter_mut() {
                    *elem = false;
                }
                if let Some(display_driver) = &mut self.display_driver {
                    display_driver.refresh(&self.vram);
                    Ok(())
                } else {
                    Err(Error::DriverMissing)
                }
            },
            Op::Ret => {
                if self.sp == 0 {
                    Err(Error::StackUnderflow)
                } else {
                    self.sp -= 1;
                    self.pc = self.stack[self.sp as usize];
                    Ok(())
                }
            },
            Op::Jmp(addr) => {
                self.pc = addr;
                Ok(())
            },
            Op::Call(addr) => {
                if self.sp as usize > self.stack.len() {
                    Err(Error::StackOverflow)
                } else {
                    self.stack[self.sp as usize] = self.pc;
                    self.sp += 1;
                    self.pc = addr;
                    Ok(())
                }
            },
            Op::Se(Reg(x @ 0..=Self::MAX_REG), kk) => {
                if self.v[x] == kk {
                    self.pc += 2;
                }
                Ok(())
            },
            Op::Sne(Reg(x @ 0..=Self::MAX_REG), kk) => {
                if self.v[x] != kk {
                    self.pc += 2;
                }
                Ok(())
            },
            Op::Sre(Reg(x @ 0..=Self::MAX_REG), Reg(y @ 0..=Self::MAX_REG)) => {
                if self.v[x] == self.v[y] {
                    self.pc += 2;
                }
                Ok(())
            },
            Op::Ld(Reg(x @ 0..=Self::MAX_REG), kk) => {
                self.v[x] = kk;
                Ok(())
            },
            Op::Add(Reg(x @ 0..=Self::MAX_REG), kk) => {
                /* per spec, carry flag intentionally not changed */
                let (val, _) = self.v[x].overflowing_add(kk);
                self.v[x] = val;
                Ok(())
            },
            Op::Mov(Reg(x @ 0..=Self::MAX_REG), Reg(y @ 0..=Self::MAX_REG)) => {
                self.v[x] = self.v[y];
                Ok(())
            },
            Op::Or(Reg(x @ 0..=Self::MAX_REG), Reg(y @ 0..=Self::MAX_REG)) => {
                self.v[x] |= self.v[y];
                Ok(())
            },
            Op::And(Reg(x @ 0..=Self::MAX_REG), Reg(y @ 0..=Self::MAX_REG)) => {
                self.v[x] &= self.v[y];
                Ok(())
            },
            Op::Xor(Reg(x @ 0..=Self::MAX_REG), Reg(y @ 0..=Self::MAX_REG)) => {
                self.v[x] ^= self.v[y];
                Ok(())
            },
            Op::Addr(Reg(x @ 0..=Self::MAX_REG), Reg(y @ 0..=Self::MAX_REG)) => {
                let (val, carry) = self.v[x].overflowing_add(self.v[y]);
                self.v[x] = val;
                self.v[Self::FLAG_REG] = carry as u8;
                Ok(())
            },
            Op::Subr(Reg(x @ 0..=Self::MAX_REG), Reg(y @ 0..=Self::MAX_REG)) => {
                let (val, carry) = self.v[x].overflowing_sub(self.v[y]);
                self.v[x] = val;
                self.v[Self::FLAG_REG] = !carry as u8;
                Ok(())
            },
            Op::Shr(Reg(x @ 0..=Self::MAX_REG), Reg(y @ 0..=Self::MAX_REG)) => {
                self.v[Self::FLAG_REG] = self.v[y] & 0x01;
                self.v[x] = self.v[y] >> 1;
                Ok(())
            },
            Op::Subnr(Reg(x @ 0..=Self::MAX_REG), Reg(y @ 0..=Self::MAX_REG)) => {
                let (val, carry) = self.v[y].overflowing_sub(self.v[x]);
                self.v[x] = val;
                self.v[Self::FLAG_REG] = !carry as u8;
                Ok(())
            },
            Op::Shl(Reg(x @ 0..=Self::MAX_REG), Reg(y @ 0..=Self::MAX_REG)) => {
                self.v[Self::FLAG_REG] = self.v[y] & 0x80;
                self.v[x] = self.v[y] << 1;
                Ok(())
            },
            Op::Srne(Reg(x @ 0..=Self::MAX_REG), Reg(y @ 0..=Self::MAX_REG)) => {
                if self.v[x] != self.v[y] {
                    self.pc += 2;
                }
                Ok(())
            },
            Op::Ldi(addr) => {
                self.i = addr;
                Ok(())
            },
            Op::Jmpi(addr) => {
                self.pc = addr + (self.v[Self::INDEX_REG] as u16);
                Ok(())
            },
            Op::Rand(Reg(x @ 0..=Self::MAX_REG), kk) => {
                self.v[x] = rand::random::<u8>() & kk;
                Ok(())
            },
            Op::Draw(Reg(x @ 0..=Self::MAX_REG), Reg(y @ 0..=Self::MAX_REG), m) => {
                if ((self.i + m as u16) as usize) < self.ram.len() {
                    let mut did_clear = false;
                    for n in 0..m {
                        let offset = self.i as usize + n as usize;
                        let spr_byte = self.ram[offset];
                        let v = (self.v[y] as usize + n as usize) % Self::DISPLAY_HEIGHT;
                        for h in 0..8 {
                            let set = (spr_byte & (1 << (7 - h))) != 0;
                            let h = (self.v[x] as usize + h) % Self::DISPLAY_WIDTH;
                            let vram_offset = v * Self::DISPLAY_WIDTH + h;
                            let will_clear = self.vram[vram_offset] && set;
                            if will_clear {
                                did_clear = true;
                            }
                            self.vram[vram_offset] ^= set;
                        }
                        self.v[Self::FLAG_REG] = did_clear as u8;
                    }

                    if let Some(display_driver) = &mut self.display_driver {
                        display_driver.refresh(&self.vram);
                        Ok(())
                    } else {
                        Err(Error::DriverMissing)
                    }
                } else {
                    Err(Error::DataAbort)
                }
            },
            Op::Skp(Reg(x @ 0..=Self::MAX_REG)) => {
                if let Some(input_driver) = &self.input_driver {
                    if input_driver.poll(self.v[x] as u8) {
                        self.pc += 2;
                    }
                    Ok(())
                } else {
                    Err(Error::DriverMissing)
                }
            },
            Op::Sknp(Reg(x @ 0..=Self::MAX_REG)) => {
                if let Some(input_driver) = &self.input_driver {
                    if !input_driver.poll(self.v[x] as u8) {
                        self.pc += 2;
                    }
                    Ok(())
                } else {
                    /* Assume that no input driver means no key press, ever. */
                    self.pc += 2;
                    Err(Error::DriverMissing)
                }
            },
            Op::Movd(Reg(x @ 0..=Self::MAX_REG)) => {
                self.v[x] = self.timer.dt.load(Ordering::Relaxed);
                Ok(())
            },
            Op::Key(Reg(x @ 0..=Self::MAX_REG)) => {
                if let Some(input_driver) = &self.input_driver {
                    self.v[x] = input_driver.block();
                    Ok(())
                } else {
                    Err(Error::DriverMissing)
                }
            },
            Op::Ldd(Reg(x @ 0..=Self::MAX_REG)) => {
                self.timer.dt.store(self.v[x], Ordering::Relaxed);
                Ok(())
            },
            Op::Lds(Reg(x @ 0..=Self::MAX_REG)) => {
                self.timer.st.store(self.v[x], Ordering::Relaxed);
                Ok(())
            },
            Op::Addi(Reg(x @ 0..=Self::MAX_REG)) => {
                self.i += self.v[x] as u16;
                Ok(())
            },
            Op::Ldspr(Reg(x @ 0..=Self::MAX_REG)) => {
                self.i = Self::FONT_SPRITES_RAM_START as u16 +
                         Self::FONT_SPRITE_BYTES_PER as u16 *
                         self.v[x] as u16;
                Ok(())
            },
            Op::Bcd(Reg(x @ 0..=Self::MAX_REG)) => {
                let i = self. i as usize;
                if i < self.ram.len() - 2 {
                    let vx = self.v[x];
                    let h = vx / 100;
                    let t = (vx - h * 100) / 10;
                    let o = vx - (h * 100) - (t * 10);

                    self.ram[i] = h;
                    self.ram[i + 1] = t;
                    self.ram[i + 2] = o;
                    Ok(())
                } else {
                    Err(Error::DataAbort)
                }
            },
            Op::Str(Reg(x @ 0..=Self::MAX_REG)) => {
                let i = self.i as usize;
                let j = i + x;
                if j < self.ram.len() {
                    self.ram[i..=j].copy_from_slice(&self.v[..=x]);
                    Ok(())
                } else {
                    Err(Error::DataAbort)
                }
            },
            Op::Read(Reg(x @ 0..=Self::MAX_REG)) => {
                let i = self.i as usize;
                let j = i + x;
                if j < self.ram.len() {
                    self.v[..=x].copy_from_slice(&self.ram[i..=j]);
                    Ok(())
                } else {
                    Err(Error::DataAbort)
                }
            },
            _ => Err(Error::MalformedOp(op)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use super::*;

    #[test]
    fn atomic() {
        let mut cpu = Cpu::new();

        cpu.exec(Op::Ld(Reg(0), 200)).unwrap();
        cpu.exec(Op::Ldd(Reg(0))).unwrap();
        cpu.exec(Op::Movd(Reg(0))).unwrap();

        assert_eq!(cpu.v[0], 200);
        thread::sleep(Duration::from_millis(500));

        cpu.exec(Op::Movd(Reg(0))).unwrap();
        assert!(cpu.v[0] < 175 && cpu.v[0] > 165);

        cpu.exec(Op::Ld(Reg(0), 200)).unwrap();
        cpu.exec(Op::Lds(Reg(0))).unwrap();
        thread::sleep(Duration::from_millis(250));

        let ds = cpu.timer.st.load(Ordering::Relaxed);
        assert!(ds < 187 && ds > 183);
    }

    #[test]
    fn bcd() {
        let mut cpu = Cpu::new();

        cpu.exec(Op::Ld(Reg(0), 135)).unwrap();
        cpu.exec(Op::Ldi(0x400)).unwrap();
        cpu.exec(Op::Bcd(Reg(0))).unwrap();
        cpu.exec(Op::Read(Reg(2))).unwrap();

        assert_eq!(cpu.v[0], 1);
        assert_eq!(cpu.v[1], 3);
        assert_eq!(cpu.v[2], 5);
    }

    #[test]
    fn draw() {
        let mut cpu = Cpu::new();

        let sprite: [u8; 3] = [
            0b11111111,
            0b10000001,
            0b11111111,
        ];

        cpu.exec(Op::Ld(Reg(0), sprite[0])).unwrap();
        cpu.exec(Op::Ld(Reg(1), sprite[1])).unwrap();
        cpu.exec(Op::Ld(Reg(2), sprite[2])).unwrap();
        cpu.exec(Op::Ldi(0x400)).unwrap();
        cpu.exec(Op::Str(Reg(2))).unwrap();
        cpu.exec(Op::Ld(Reg(3), 0x15)).unwrap();
        cpu.exec(Op::Ld(Reg(4), 0x05)).unwrap();
        assert_eq!(cpu.exec(Op::Draw(Reg(3), Reg(4), 3)), Err(Error::DriverMissing));

        let row1_start = 0x05 * Cpu::DISPLAY_WIDTH + 0x15;
        let row1_end = 0x05 * Cpu::DISPLAY_WIDTH + 0x1d;
        let row2_start = 0x06 * Cpu::DISPLAY_WIDTH + 0x15;
        let row2_end = 0x06 * Cpu::DISPLAY_WIDTH + 0x1d;
        let row3_start = 0x07 * Cpu::DISPLAY_WIDTH + 0x15;
        let row3_end = 0x07 * Cpu::DISPLAY_WIDTH + 0x1d;

        assert_eq!(cpu.vram[row1_start..row1_end], [true, true, true, true, true, true, true, true]);
        assert_eq!(cpu.vram[row2_start..row2_end], [true, false, false, false, false, false, false, true]);
        assert_eq!(cpu.vram[row3_start..row3_end], [true, true, true, true, true, true, true, true]);
        assert_eq!(cpu.v[Cpu::FLAG_REG], 0x00);

        /* Draw the same sprite again to clear it. */
        assert_eq!(cpu.exec(Op::Draw(Reg(3), Reg(4), 3)), Err(Error::DriverMissing));

        assert_eq!(cpu.vram[row1_start..row1_end], [false, false, false, false, false, false, false, false]);
        assert_eq!(cpu.vram[row2_start..row2_end], [false, false, false, false, false, false, false, false]);
        assert_eq!(cpu.vram[row3_start..row3_end], [false, false, false, false, false, false, false, false]);
        assert_eq!(cpu.v[Cpu::FLAG_REG], 0x01);

        cpu.exec(Op::Ld(Reg(3), 60)).unwrap();
        cpu.exec(Op::Ld(Reg(4), 30)).unwrap();
        assert_eq!(cpu.exec(Op::Draw(Reg(3), Reg(4), 3)), Err(Error::DriverMissing));

        let row1_unwrapped_start = 30 * Cpu::DISPLAY_WIDTH + 60;
        let row1_unwrapped_end = 30 * Cpu::DISPLAY_WIDTH + 64;
        let row2_unwrapped_start = 31 * Cpu::DISPLAY_WIDTH + 60;
        let row2_unwrapped_end = 31 * Cpu::DISPLAY_WIDTH + 64;
        let row3_unwrapped_start = 0 * Cpu::DISPLAY_WIDTH + 60;
        let row3_unwrapped_end = 0 * Cpu::DISPLAY_WIDTH + 64;
        let row1_wrapped_start = 30 * Cpu::DISPLAY_WIDTH + 0;
        let row1_wrapped_end = 30 * Cpu::DISPLAY_WIDTH + 4;
        let row2_wrapped_start = 31 * Cpu::DISPLAY_WIDTH + 0;
        let row2_wrapped_end = 31 * Cpu::DISPLAY_WIDTH + 4;
        let row3_wrapped_start = 0 * Cpu::DISPLAY_WIDTH + 0;
        let row3_wrapped_end = 0 * Cpu::DISPLAY_WIDTH + 4;

        assert_eq!(cpu.vram[row1_unwrapped_start..row1_unwrapped_end], [true, true, true, true]);
        assert_eq!(cpu.vram[row2_unwrapped_start..row2_unwrapped_end], [true, false, false, false]);
        assert_eq!(cpu.vram[row3_unwrapped_start..row3_unwrapped_end], [true, true, true, true]);
        assert_eq!(cpu.vram[row1_wrapped_start..row1_wrapped_end], [true, true, true, true]);
        assert_eq!(cpu.vram[row2_wrapped_start..row2_wrapped_end], [false, false, false, true]);
        assert_eq!(cpu.vram[row3_wrapped_start..row3_wrapped_end], [true, true, true, true]);
        assert_eq!(cpu.v[Cpu::FLAG_REG], 0x00);
    }

    #[test]
    fn load_and_tick() {
        let program: [u8; 6] = [
            0x60,
            0x12, /* ld r0, 0x12 */
            0x61,
            0x02, /* ld r1, 0x02 */
            0x80,
            0x14, /* addr r0, r1 */
        ];

        let mut cpu = Cpu::new();
        let lo = Cpu::LOAD_OFFSET as u16;
        cpu.load(&program).unwrap();
        assert_eq!(cpu.pc, lo);

        cpu.tick().unwrap();
        assert_eq!(cpu.pc, lo + 2);
        assert_eq!(cpu.v[0], 0x12);

        cpu.tick().unwrap();
        assert_eq!(cpu.pc, lo + 4);
        assert_eq!(cpu.v[1], 0x02);

        cpu.tick().unwrap();
        assert_eq!(cpu.pc, lo + 6);
        assert_eq!(cpu.v[0], 0x14);

        assert_eq!(cpu.tick(), Err(Error::BadInstruction));
        assert_eq!(cpu.pc, lo + 6);
    }
}
