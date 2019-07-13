
// TODO: Add Cpu struct method tests.
// TODO: Add documentation.

use std::fmt;
use std::error;

use super::driver;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    BadInstruction,
    LoadFailure,
    PrefetchAbort,
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

// TODO: Add clock speed.
pub struct Cpu {
    pc: u16,
    sp: u8,
    i: u16,
    dt: u8,
    st: u8,
    v: [u8; 0x10],
    ram: [u8; 0x1000],
    vram: [u8; 0x100],
    stack: [u16; 0x20],
    display_driver: Option<Box<dyn driver::Display>>,
    sound_driver: Option<Box<dyn driver::Sound>>,
    input_driver: Option<Box<dyn driver::Input>>,
}

type Result<T> = std::result::Result<T, Error>;

impl Cpu {
    const LOAD_OFFSET: usize = 0x200;

    pub fn new() -> Self {
        Cpu {
            pc: 0x0000,
            sp: 0x00,
            i: 0x0000,
            dt: 0x00,
            st: 0x00,
            v: [0x00; 0x10],
            ram: [0xff; 0x1000],
            vram: [0x00; 0x100],
            stack: [0x0000; 0x20],
            display_driver: None,
            sound_driver: None,
            input_driver: None,
        }
    }

    pub fn load(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > self.ram.len() - Self::LOAD_OFFSET {
            Err(Error::LoadFailure)
        } else {
            self.ram[..Self::LOAD_OFFSET].clone_from_slice(data);
            self.pc = Self::LOAD_OFFSET as u16;
            Ok(())
        }
    }

    pub fn set_display_driver(&mut self, driver: Option<Box<dyn driver::Display>>) {
        self.display_driver = driver;
    }

    pub fn set_sound_driver(&mut self, driver: Option<Box<dyn driver::Sound>>) {
        self.sound_driver = driver;
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
            // TODO: Implement all ops.
            _ => Ok(())
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Reg(u8);

#[derive(Debug, PartialEq)]
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
        let x = Reg(nib2);
        let y = Reg(nib1);
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
}
