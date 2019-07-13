pub struct Reg(u8);

// TODO: Reconsider some of these names.
// TODO: Do we really need an intermediary type?
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
    Undef,
}

impl Op {
    pub fn decode(code: u16) -> Self {
        let nnn = code & 0xfff;
        let x = Reg(((code & 0x0f00) >> 8) as u8);
        let y = Reg(((code & 0x00f0) >> 4) as u8);
        let kk = (code & 0xff) as u8;

        match (code & 0xf00 >> 12, code & 0xf00 >> 8, code & 0xf0 >> 4, code & 0xf) {
            (0, 0, 0xe, 0) => Op::Cls,
            (0, 0, 0xe, 0xe) => Op::Ret,
            (0, _, _, _) => Op::Sys(nnn),
            (1, _, _, _) => Op::Jmp(nnn),
            (2, _, _, _) => Op::Call(nnn),
            (3, _, _, _) => Op::Se(x, kk),
            (4, _, _, _) => Op::Sne(x, kk),
            (5, _, _, 0) => Op::Sre(x, y),
            (6, _, _, _) => Op::Ld(x, kk),
            (7, _, _, _) => Op::Add(x, kk),
            (8, _, _, 0) => Op::Mov(x, y),
            (8, _, _, 1) => Op::Or(x, y),
            (8, _, _, 2) => Op::And(x, y),
            (8, _, _, 3) => Op::Xor(x, y),
            (8, _, _, 4) => Op::Addr(x, y),
            (8, _, _, 5) => Op::Subr(x, y),
            (8, _, _, 6) => Op::Shr(y, x),
            (8, _, _, 7) => Op::Subnr(x, y),
            (8, _, _, 0xe) => Op::Shl(y, x),
            (9, _, _, 0) => Op::Srne(x, y),
            (0xa, _, _, _) => Op::Ldi(nnn),
            (0xb, _, _, _) => Op::Jmpi(nnn),
            (0xc, _, _, _) => Op::Rand(x, kk),
            (0xd, _, _, m) => Op::Draw(x, y, m as u8),
            (0xe, _, 9, 0xe) => Op::Skp(x),
            (0xe, _, 0xa, 1) => Op::Sknp(x),
            (0xf, _, 0, 7) => Op::Movd(x),
            (0xf, _, 0, 0xa) => Op::Key(x),
            (0xf, _, 1, 5) => Op::Ldd(x),
            (0xf, _, 1, 8) => Op::Lds(x),
            (0xf, _, 1, 0xe) => Op::Addi(x),
            (0xf, _, 2, 9) => Op::Ldspr(x),
            (0xf, _, 3, 3) => Op::Bcd(x),
            (0xf, _, 5, 5) => Op::Str(x),
            (0xf, _, 6, 5) => Op::Read(x),
            _ => Op::Undef,
        }
    }
}
