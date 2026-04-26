use std::convert::TryFrom;

use crate::op_code::OpCode;


#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    BC {
        op_code: OpCode,
        a: u8,
        b: u8,
        c: u8,
        aux: u32,
    },
    AD {
        op_code: OpCode,
        a: u8,
        d: i16,
        aux: u32,
    },
    E {
        op_code: OpCode,
        e: i32,
    },
}

impl Instruction {
    pub fn parse(insn: u32, encode_key: u8) -> Result<Instruction, nom::error::ErrorKind> {
        let op_code = (insn & 0xFF) as u8;
        let op_code = op_code.wrapping_mul(encode_key);
        match op_code {
            0
            | 1
            | 2
            | 3
            | 6..=11
            | 13..=18
            | 20..=22
            | 33..=53
            | 55
            | 60
            | 63
            | 65
            | 66
            | 68
            | 70
            | 71..=75
            | 81
            | 82
            | 83..=85
            | 251 => {
                let (a, b, c) = Self::parse_abc(insn);

                Ok(Self::BC {
                    op_code: OpCode::try_from(op_code).unwrap(),
                    a,
                    b,
                    c,
                    aux: 0,
                })
            }
            4 | 5 | 12 | 19 | 23..=32 | 54 | 56..=59 | 61 | 62 | 64 | 76..=80 => {
                let (a, d) = Self::parse_ad(insn);

                Ok(Self::AD {
                    op_code: OpCode::try_from(op_code).unwrap(),
                    a,
                    d,
                    aux: 0,
                })
            }
            67 | 69 => {
                let e = Self::parse_e(insn);

                Ok(Self::E {
                    op_code: OpCode::try_from(op_code).unwrap(),
                    e,
                })
            }
            97 => Ok(Self::BC {
                op_code: OpCode::try_from(0).unwrap(),
                a: 0,
                b: 0,
                c: 0,
                aux: 0,
            }),
            
            
            _ => Err(nom::error::ErrorKind::Tag),
        }
    }

    fn parse_abc(insn: u32) -> (u8, u8, u8) {
        let a = ((insn >> 8) & 0xFF) as u8;
        let b = ((insn >> 16) & 0xFF) as u8;
        let c = ((insn >> 24) & 0xFF) as u8;

        (a, b, c)
    }

    fn parse_ad(insn: u32) -> (u8, i16) {
        let a = ((insn >> 8) & 0xFF) as u8;
        let d = ((insn >> 16) & 0xFFFF) as i16;

        (a, d)
    }

    fn parse_e(insn: u32) -> i32 {
        (insn as i32) >> 8
    }
}
