#[derive(Debug, PartialEq, Clone)]
pub struct Reg(pub usize);

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
    pub fn decode(code: u16) -> Option<Self> {
        let nib3 = ((code & 0xf000) >> 12) as u8;
        let nib2 = ((code & 0xf00) >> 8) as u8;
        let nib1 = ((code & 0xf0) >> 4) as u8;
        let nib0 = (code & 0xf) as u8;

        let nnn = code & 0xfff;
        let x = Reg(nib2 as usize);
        let y = Reg(nib1 as usize);
        let kk = (code & 0xff) as u8;

        match (nib3, nib2, nib1, nib0) {
            (0, 0, 0xe, 0) => Some(Op::Cls),
            (0, 0, 0xe, 0xe) => Some(Op::Ret),
            (0, _, _, _) => Some(Op::Sys(nnn)),
            (1, _, _, _) => Some(Op::Jmp(nnn)),
            (2, _, _, _) => Some(Op::Call(nnn)),
            (3, _, _, _) => Some(Op::Se(x, kk)),
            (4, _, _, _) => Some(Op::Sne(x, kk)),
            (5, _, _, 0) => Some(Op::Sre(x, y)),
            (6, _, _, _) => Some(Op::Ld(x, kk)),
            (7, _, _, _) => Some(Op::Add(x, kk)),
            (8, _, _, 0) => Some(Op::Mov(x, y)),
            (8, _, _, 1) => Some(Op::Or(x, y)),
            (8, _, _, 2) => Some(Op::And(x, y)),
            (8, _, _, 3) => Some(Op::Xor(x, y)),
            (8, _, _, 4) => Some(Op::Addr(x, y)),
            (8, _, _, 5) => Some(Op::Subr(x, y)),
            (8, _, _, 6) => Some(Op::Shr(y, x)),
            (8, _, _, 7) => Some(Op::Subnr(x, y)),
            (8, _, _, 0xe) => Some(Op::Shl(y, x)),
            (9, _, _, 0) => Some(Op::Srne(x, y)),
            (0xa, _, _, _) => Some(Op::Ldi(nnn)),
            (0xb, _, _, _) => Some(Op::Jmpi(nnn)),
            (0xc, _, _, _) => Some(Op::Rand(x, kk)),
            (0xd, _, _, m) => Some(Op::Draw(x, y, m)),
            (0xe, _, 9, 0xe) => Some(Op::Skp(x)),
            (0xe, _, 0xa, 1) => Some(Op::Sknp(x)),
            (0xf, _, 0, 7) => Some(Op::Movd(x)),
            (0xf, _, 0, 0xa) => Some(Op::Key(x)),
            (0xf, _, 1, 5) => Some(Op::Ldd(x)),
            (0xf, _, 1, 8) => Some(Op::Lds(x)),
            (0xf, _, 1, 0xe) => Some(Op::Addi(x)),
            (0xf, _, 2, 9) => Some(Op::Ldspr(x)),
            (0xf, _, 3, 3) => Some(Op::Bcd(x)),
            (0xf, _, 5, 5) => Some(Op::Str(x)),
            (0xf, _, 6, 5) => Some(Op::Read(x)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_decode() {
        assert_eq!(Op::decode(0x00e0), Some(Op::Cls));
        assert_eq!(Op::decode(0x00ee), Some(Op::Ret));
        assert_eq!(Op::decode(0x0123), Some(Op::Sys(0x123)));
        assert_eq!(Op::decode(0x1456), Some(Op::Jmp(0x456)));
        assert_eq!(Op::decode(0x2789), Some(Op::Call(0x789)));
        assert_eq!(Op::decode(0x3abc), Some(Op::Se(Reg(0xa), 0xbc)));
        assert_eq!(Op::decode(0x4ef0), Some(Op::Sne(Reg(0xe), 0xf0)));
        assert_eq!(Op::decode(0x5010), Some(Op::Sre(Reg(0), Reg(1))));
        assert_eq!(Op::decode(0x6234), Some(Op::Ld(Reg(2), 0x34)));
        assert_eq!(Op::decode(0x7567), Some(Op::Add(Reg(5), 0x67)));
        assert_eq!(Op::decode(0x8890), Some(Op::Mov(Reg(8), Reg(9))));
        assert_eq!(Op::decode(0x8ab1), Some(Op::Or(Reg(0xa), Reg(0xb))));
        assert_eq!(Op::decode(0x8cd2), Some(Op::And(Reg(0xc), Reg(0xd))));
        assert_eq!(Op::decode(0x8ef3), Some(Op::Xor(Reg(0xe), Reg(0xf))));
        assert_eq!(Op::decode(0x8014), Some(Op::Addr(Reg(0), Reg(1))));
        assert_eq!(Op::decode(0x8235), Some(Op::Subr(Reg(2), Reg(3))));
        assert_eq!(Op::decode(0x8456), Some(Op::Shr(Reg(5), Reg(4))));
        assert_eq!(Op::decode(0x8677), Some(Op::Subnr(Reg(6), Reg(7))));
        assert_eq!(Op::decode(0x889e), Some(Op::Shl(Reg(9), Reg(8))));
        assert_eq!(Op::decode(0x9ab0), Some(Op::Srne(Reg(0xa), Reg(0xb))));
        assert_eq!(Op::decode(0xacde), Some(Op::Ldi(0xcde)));
        assert_eq!(Op::decode(0xbef0), Some(Op::Jmpi(0xef0)));
        assert_eq!(Op::decode(0xc123), Some(Op::Rand(Reg(1), 0x23)));
        assert_eq!(Op::decode(0xd456), Some(Op::Draw(Reg(4), Reg(5), 6)));
        assert_eq!(Op::decode(0xe79e), Some(Op::Skp(Reg(7))));
        assert_eq!(Op::decode(0xe8a1), Some(Op::Sknp(Reg(8))));
        assert_eq!(Op::decode(0xf907), Some(Op::Movd(Reg(9))));
        assert_eq!(Op::decode(0xfa0a), Some(Op::Key(Reg(0xa))));
        assert_eq!(Op::decode(0xfb15), Some(Op::Ldd(Reg(0xb))));
        assert_eq!(Op::decode(0xfc18), Some(Op::Lds(Reg(0xc))));
        assert_eq!(Op::decode(0xfd1e), Some(Op::Addi(Reg(0xd))));
        assert_eq!(Op::decode(0xfe29), Some(Op::Ldspr(Reg(0xe))));
        assert_eq!(Op::decode(0xff33), Some(Op::Bcd(Reg(0xf))));
        assert_eq!(Op::decode(0xf055), Some(Op::Str(Reg(0))));
        assert_eq!(Op::decode(0xf165), Some(Op::Read(Reg(1))));
        assert_eq!(Op::decode(0xffff), None);
    }
}

