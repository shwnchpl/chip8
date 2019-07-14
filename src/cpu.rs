
// TODO: Add Cpu struct method tests.
// TODO: Add documentation.
// TODO: Verify that the Orderings used for atomic operations are correct.
// TODO: Verify that mutability for sound driver will actually work.
// TODO: Implement drop and thread join.

extern crate rand;

use std::error;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU8, Ordering};
use std::thread;
use std::time::Duration;

use super::driver;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    BadInstruction,
    DataAbort,
    DriverMissing,
    LoadFailure,
    MalformedOp(Op),
    PrefetchAbort,
    StackOverflow,
    StackUnderflow,
    UnimplementedOp(Op),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

type SoundDriver = Arc<Mutex<Option<Box<dyn driver::Sound>>>>;

pub struct Cpu {
    pc: u16,
    sp: u8,
    i: u16,
    dt: Arc<AtomicU8>,
    st: Arc<AtomicU8>,
    v: [u8; Self::REG_COUNT],
    ram: [u8; Self::RAM_BYTES],
    vram: [bool; Self::VRAM_BYTES],
    stack: [u16; Self::MAX_STACK_DEPTH],
    display_driver: Option<Box<dyn driver::Display>>,
    sound_driver: SoundDriver,
    input_driver: Option<Box<dyn driver::Input>>,
    timer_thread: thread::JoinHandle<()>,
}

type Result<T> = std::result::Result<T, Error>;

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
        let dt = Arc::new(AtomicU8::new(0x00));
        let st = Arc::new(AtomicU8::new(0x00));
        let sound_driver: SoundDriver = Arc::new(Mutex::new(None));

        let dt_clone = Arc::clone(&dt);
        let st_clone = Arc::clone(&st);
        let sound_driver_clone = Arc::clone(&sound_driver);

        let timer_thread = thread::spawn(move || {
            let mut st_was_pos = false;

            loop {
                let v = dt_clone.load(Ordering::Relaxed);
                if v > 0 {
                    /* Only decrement if the value didn't just change out from
                       under us. If it did, we'll catch up next cycle. Same
                       goes for the sound timer below. */
                    dt_clone.compare_and_swap(v, v - 1, Ordering::Relaxed);
                }

                let mut v = st_clone.load(Ordering::Relaxed);
                if v > 0 {
                    v = st_clone.compare_and_swap(v, v - 1, Ordering::Relaxed);
                }

                if v <= 1 && st_was_pos {
                    let mut lock = sound_driver_clone.try_lock();
                    if let Ok(ref mut mutex) = lock {
                        if let Some(sound_driver) = &mut **mutex {
                            sound_driver.stop_buzz();
                        }
                        st_was_pos = false;
                    }
                } else if  v > 1 && !st_was_pos {
                    let mut lock = sound_driver_clone.try_lock();
                    if let Ok(ref mut mutex) = lock {
                        if let Some(sound_driver) = &mut **mutex {
                            sound_driver.start_buzz();
                        }
                        st_was_pos = true;
                    }
                }

                thread::sleep(Duration::from_millis(16)); // Decent estimation of 60hz
            }
        });

        let mut ram = [0xff; Self::RAM_BYTES];
        ram[Self::FONT_SPRITES_RAM_START..Self::FONT_SPRITES_RAM_END]
            .copy_from_slice(&Self::FONT_SPRITES);

        Cpu {
            pc: 0x0000,
            sp: 0x00,
            i: 0x0000,
            dt: dt,
            st: st,
            v: [0x00; Self::REG_COUNT],
            ram: ram,
            vram: [false; Self::VRAM_BYTES],
            stack: [0x0000; Self::MAX_STACK_DEPTH],
            display_driver: None,
            sound_driver: sound_driver,
            input_driver: None,
            timer_thread: timer_thread,
        }
    }

    pub fn load(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > self.ram.len() - Self::LOAD_OFFSET {
            Err(Error::LoadFailure)
        } else {
            self.ram[Self::LOAD_OFFSET..data.len()].copy_from_slice(data);
            self.pc = Self::LOAD_OFFSET as u16;
            Ok(())
        }
    }

    pub fn set_display_driver(&mut self, driver: Option<Box<dyn driver::Display>>) {
        self.display_driver = driver;
    }

    pub fn set_sound_driver(&mut self, driver: Option<Box<dyn driver::Sound>>) {
        let d = Arc::clone(&self.sound_driver);
        let mut d = d.lock().unwrap();
        *d = driver;
    }

    pub fn set_input_driver(&mut self, driver: Option<Box<dyn driver::Input>>) {
        self.input_driver = driver;
    }

    // TODO: Add a function to tick appropriately to some clock speed.
    pub fn tick(&mut self) -> Result<()> {
        let opcode = self.fetch()?;
        let op = Op::decode(opcode)?;
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
                    self.sp += 2;
                }
                Ok(())
            },
            Op::Sne(Reg(x @ 0..=Self::MAX_REG), kk) => {
                if self.v[x] != kk {
                    self.sp += 2;
                }
                Ok(())
            },
            Op::Sre(Reg(x @ 0..=Self::MAX_REG), Reg(y @ 0..=Self::MAX_REG)) => {
                if self.v[x] == self.v[y] {
                    self.sp += 2;
                }
                Ok(())
            },
            Op::Ld(Reg(x @ 0..=Self::MAX_REG), kk) => {
                self.v[x] = kk;
                Ok(())
            },
            Op::Add(Reg(x @ 0..=Self::MAX_REG), kk) => {
                /* per spec, carry flag intentionally not changed */
                self.v[x] += kk;
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
                    self.sp += 2;
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
                            let set = (spr_byte & (1 << h)) != 0;
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
                    if input_driver.poll(x as u8) {
                        self.sp += 2;
                    }
                    Ok(())
                } else {
                    Err(Error::DriverMissing)
                }
            },
            Op::Sknp(Reg(x @ 0..=Self::MAX_REG)) => {
                if let Some(input_driver) = &self.input_driver {
                    if !input_driver.poll(x as u8) {
                        self.sp += 2;
                    }
                    Ok(())
                } else {
                    /* Assume that no input driver means no key press, ever. */
                    self.sp += 2;
                    Err(Error::DriverMissing)
                }
            },
            Op::Movd(Reg(x @ 0..=Self::MAX_REG)) => {
                self.v[x] = self.dt.load(Ordering::Relaxed);
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
                self.dt.store(self.v[x], Ordering::Relaxed);
                Ok(())
            },
            Op::Lds(Reg(x @ 0..=Self::MAX_REG)) => {
                self.st.store(self.v[x], Ordering::Relaxed);
                Ok(())
            },
            Op::Addi(Reg(x @ 0..=Self::MAX_REG)) => {
                self.i += self.v[x] as u16;
                Ok(())
            },
            Op::Ldspr(Reg(x @ 0..=Self::MAX_REG)) => {
                self.i = Self::FONT_SPRITES_RAM_START as u16 +
                         Self::FONT_SPRITE_BYTES_PER as u16 *
                         x as u16;
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

#[derive(Debug, PartialEq, Clone)]
pub struct Reg(usize);

#[derive(Debug, PartialEq, Clone)]
pub enum Op {
    Cls,
    Ret,
    Sys(u16),
    Jmp(u16),
    Call(u16),
    Se(Reg, u8),
    Sne(Reg, u8),
    Sre(Reg, Reg),
    Ld(Reg, u8),
    Add(Reg, u8),
    Mov(Reg, Reg),
    Or(Reg, Reg),
    And(Reg, Reg),
    Xor(Reg, Reg),
    Addr(Reg, Reg),
    Subr(Reg, Reg),
    Shr(Reg, Reg),
    Subnr(Reg, Reg),
    Shl(Reg, Reg),
    Srne(Reg, Reg),
    Ldi(u16),
    Jmpi(u16),
    Rand(Reg, u8),
    Draw(Reg, Reg, u8),
    Skp(Reg),
    Sknp(Reg),
    Movd(Reg),
    Key(Reg),
    Ldd(Reg),
    Lds(Reg),
    Addi(Reg),
    Ldspr(Reg),
    Bcd(Reg),
    Str(Reg),
    Read(Reg),
}

impl Op {
    pub fn decode(code: u16) -> Result<Self> {
        let nib3 = ((code & 0xf000) >> 12) as u8;
        let nib2 = ((code & 0xf00) >> 8) as u8;
        let nib1 = ((code & 0xf0) >> 4) as u8;
        let nib0 = (code & 0xf) as u8;

        let nnn = code & 0xfff;
        let x = Reg(nib2 as usize);
        let y = Reg(nib1 as usize);
        let kk = (code & 0xff) as u8;

        match (nib3, nib2, nib1, nib0) {
            (0, 0, 0xe, 0) => Ok(Op::Cls),
            (0, 0, 0xe, 0xe) => Ok(Op::Ret),
            (0, _, _, _) => Ok(Op::Sys(nnn)),
            (1, _, _, _) => Ok(Op::Jmp(nnn)),
            (2, _, _, _) => Ok(Op::Call(nnn)),
            (3, _, _, _) => Ok(Op::Se(x, kk)),
            (4, _, _, _) => Ok(Op::Sne(x, kk)),
            (5, _, _, 0) => Ok(Op::Sre(x, y)),
            (6, _, _, _) => Ok(Op::Ld(x, kk)),
            (7, _, _, _) => Ok(Op::Add(x, kk)),
            (8, _, _, 0) => Ok(Op::Mov(x, y)),
            (8, _, _, 1) => Ok(Op::Or(x, y)),
            (8, _, _, 2) => Ok(Op::And(x, y)),
            (8, _, _, 3) => Ok(Op::Xor(x, y)),
            (8, _, _, 4) => Ok(Op::Addr(x, y)),
            (8, _, _, 5) => Ok(Op::Subr(x, y)),
            (8, _, _, 6) => Ok(Op::Shr(y, x)),
            (8, _, _, 7) => Ok(Op::Subnr(x, y)),
            (8, _, _, 0xe) => Ok(Op::Shl(y, x)),
            (9, _, _, 0) => Ok(Op::Srne(x, y)),
            (0xa, _, _, _) => Ok(Op::Ldi(nnn)),
            (0xb, _, _, _) => Ok(Op::Jmpi(nnn)),
            (0xc, _, _, _) => Ok(Op::Rand(x, kk)),
            (0xd, _, _, m) => Ok(Op::Draw(x, y, m)),
            (0xe, _, 9, 0xe) => Ok(Op::Skp(x)),
            (0xe, _, 0xa, 1) => Ok(Op::Sknp(x)),
            (0xf, _, 0, 7) => Ok(Op::Movd(x)),
            (0xf, _, 0, 0xa) => Ok(Op::Key(x)),
            (0xf, _, 1, 5) => Ok(Op::Ldd(x)),
            (0xf, _, 1, 8) => Ok(Op::Lds(x)),
            (0xf, _, 1, 0xe) => Ok(Op::Addi(x)),
            (0xf, _, 2, 9) => Ok(Op::Ldspr(x)),
            (0xf, _, 3, 3) => Ok(Op::Bcd(x)),
            (0xf, _, 5, 5) => Ok(Op::Str(x)),
            (0xf, _, 6, 5) => Ok(Op::Read(x)),
            _ => Err(Error::BadInstruction),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_decode() {
        assert_eq!(Op::decode(0x00e0), Ok(Op::Cls));
        assert_eq!(Op::decode(0x00ee), Ok(Op::Ret));
        assert_eq!(Op::decode(0x0123), Ok(Op::Sys(0x123)));
        assert_eq!(Op::decode(0x1456), Ok(Op::Jmp(0x456)));
        assert_eq!(Op::decode(0x2789), Ok(Op::Call(0x789)));
        assert_eq!(Op::decode(0x3abc), Ok(Op::Se(Reg(0xa), 0xbc)));
        assert_eq!(Op::decode(0x4ef0), Ok(Op::Sne(Reg(0xe), 0xf0)));
        assert_eq!(Op::decode(0x5010), Ok(Op::Sre(Reg(0), Reg(1))));
        assert_eq!(Op::decode(0x6234), Ok(Op::Ld(Reg(2), 0x34)));
        assert_eq!(Op::decode(0x7567), Ok(Op::Add(Reg(5), 0x67)));
        assert_eq!(Op::decode(0x8890), Ok(Op::Mov(Reg(8), Reg(9))));
        assert_eq!(Op::decode(0x8ab1), Ok(Op::Or(Reg(0xa), Reg(0xb))));
        assert_eq!(Op::decode(0x8cd2), Ok(Op::And(Reg(0xc), Reg(0xd))));
        assert_eq!(Op::decode(0x8ef3), Ok(Op::Xor(Reg(0xe), Reg(0xf))));
        assert_eq!(Op::decode(0x8014), Ok(Op::Addr(Reg(0), Reg(1))));
        assert_eq!(Op::decode(0x8235), Ok(Op::Subr(Reg(2), Reg(3))));
        assert_eq!(Op::decode(0x8456), Ok(Op::Shr(Reg(5), Reg(4))));
        assert_eq!(Op::decode(0x8677), Ok(Op::Subnr(Reg(6), Reg(7))));
        assert_eq!(Op::decode(0x889e), Ok(Op::Shl(Reg(9), Reg(8))));
        assert_eq!(Op::decode(0x9ab0), Ok(Op::Srne(Reg(0xa), Reg(0xb))));
        assert_eq!(Op::decode(0xacde), Ok(Op::Ldi(0xcde)));
        assert_eq!(Op::decode(0xbef0), Ok(Op::Jmpi(0xef0)));
        assert_eq!(Op::decode(0xc123), Ok(Op::Rand(Reg(1), 0x23)));
        assert_eq!(Op::decode(0xd456), Ok(Op::Draw(Reg(4), Reg(5), 6)));
        assert_eq!(Op::decode(0xe79e), Ok(Op::Skp(Reg(7))));
        assert_eq!(Op::decode(0xe8a1), Ok(Op::Sknp(Reg(8))));
        assert_eq!(Op::decode(0xf907), Ok(Op::Movd(Reg(9))));
        assert_eq!(Op::decode(0xfa0a), Ok(Op::Key(Reg(0xa))));
        assert_eq!(Op::decode(0xfb15), Ok(Op::Ldd(Reg(0xb))));
        assert_eq!(Op::decode(0xfc18), Ok(Op::Lds(Reg(0xc))));
        assert_eq!(Op::decode(0xfd1e), Ok(Op::Addi(Reg(0xd))));
        assert_eq!(Op::decode(0xfe29), Ok(Op::Ldspr(Reg(0xe))));
        assert_eq!(Op::decode(0xff33), Ok(Op::Bcd(Reg(0xf))));
        assert_eq!(Op::decode(0xf055), Ok(Op::Str(Reg(0))));
        assert_eq!(Op::decode(0xf165), Ok(Op::Read(Reg(1))));
        assert_eq!(Op::decode(0xffff), Err(Error::BadInstruction));
    }

    #[test]
    fn atomic() {
        let mut cpu = Cpu::new();

        cpu.exec(Op::Ld(Reg(0), 200));
        cpu.exec(Op::Ldd(Reg(0)));
        cpu.exec(Op::Movd(Reg(0)));

        assert_eq!(cpu.v[0], 200);
        thread::sleep(Duration::from_millis(500));

        cpu.exec(Op::Movd(Reg(0)));
        assert!(cpu.v[0] < 175 && cpu.v[0] > 165);

        cpu.exec(Op::Ld(Reg(0), 200));
        cpu.exec(Op::Lds(Reg(0)));
        thread::sleep(Duration::from_millis(250));

        let ds = cpu.st.load(Ordering::Relaxed);
        assert!(ds < 187 && ds > 183);
    }

    #[test]
    fn bcd() {
        let mut cpu = Cpu::new();

        cpu.exec(Op::Ld(Reg(0), 135));
        cpu.exec(Op::Ldi(0x400));
        cpu.exec(Op::Bcd(Reg(0)));
        cpu.exec(Op::Read(Reg(2)));

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

        cpu.exec(Op::Ld(Reg(0), sprite[0]));
        cpu.exec(Op::Ld(Reg(1), sprite[1]));
        cpu.exec(Op::Ld(Reg(2), sprite[2]));
        cpu.exec(Op::Ldi(0x400));
        cpu.exec(Op::Str(Reg(2)));
        cpu.exec(Op::Ld(Reg(3), 0x15));
        cpu.exec(Op::Ld(Reg(4), 0x05));
        cpu.exec(Op::Draw(Reg(3), Reg(4), 3));

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
        cpu.exec(Op::Draw(Reg(3), Reg(4), 3));

        assert_eq!(cpu.vram[row1_start..row1_end], [false, false, false, false, false, false, false, false]);
        assert_eq!(cpu.vram[row2_start..row2_end], [false, false, false, false, false, false, false, false]);
        assert_eq!(cpu.vram[row3_start..row3_end], [false, false, false, false, false, false, false, false]);
        assert_eq!(cpu.v[Cpu::FLAG_REG], 0x01);

        cpu.exec(Op::Ld(Reg(3), 60));
        cpu.exec(Op::Ld(Reg(4), 30));
        cpu.exec(Op::Draw(Reg(3), Reg(4), 3));

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
}
