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
        let op_code_raw = (insn & 0xFF) as u8;
        let op_code_raw = op_code_raw.wrapping_mul(encode_key);
        
        let op_code = match OpCode::try_from(op_code_raw) {
            Ok(op) => op,
            Err(_) => return Err(nom::error::ErrorKind::Tag),
        };

        match op_code {
            OpCode::LOP_JUMPX | OpCode::LOP_COVERAGE => {
                let e = Self::parse_e(insn);
                Ok(Self::E { op_code, e })
            }
            OpCode::LOP_LOADN
            | OpCode::LOP_LOADK
            | OpCode::LOP_GETIMPORT
            | OpCode::LOP_NEWCLOSURE
            | OpCode::LOP_JUMP
            | OpCode::LOP_JUMPBACK
            | OpCode::LOP_JUMPIF
            | OpCode::LOP_JUMPIFNOT
            | OpCode::LOP_JUMPIFEQ
            | OpCode::LOP_JUMPIFLE
            | OpCode::LOP_JUMPIFLT
            | OpCode::LOP_JUMPIFNOTEQ
            | OpCode::LOP_JUMPIFNOTLE
            | OpCode::LOP_JUMPIFNOTLT
            | OpCode::LOP_DUPTABLE
            | OpCode::LOP_FORNPREP
            | OpCode::LOP_FORNLOOP
            | OpCode::LOP_FORGLOOP
            | OpCode::LOP_FORGPREP_INEXT
            | OpCode::LOP_FORGPREP_NEXT
            | OpCode::LOP_NATIVECALL
            | OpCode::LOP_DUPCLOSURE
            | OpCode::LOP_FORGPREP
            | OpCode::LOP_JUMPXEQKNIL
            | OpCode::LOP_JUMPXEQKB
            | OpCode::LOP_JUMPXEQKN
            | OpCode::LOP_JUMPXEQKS
            | OpCode::LOP_CMPPROTO => {
                let (a, d) = Self::parse_ad(insn);
                Ok(Self::AD {
                    op_code,
                    a,
                    d,
                    aux: 0,
                })
            }
            OpCode::LOP_BITAND
            | OpCode::LOP_BITOR
            | OpCode::LOP_BITXOR
            | OpCode::LOP_BITNOT
            | OpCode::LOP_BITLSHIFT
            | OpCode::LOP_BITRSHIFT
            | OpCode::LOP_BITARSHIFT
            | OpCode::LOP_BITANDK
            | OpCode::LOP_BITORK
            | OpCode::LOP_BITXORK
            | OpCode::LOP_SUBRK
            | OpCode::LOP_DIVRK => {
                 let (a, b, c) = Self::parse_abc(insn);
                 Ok(Self::BC {
                    op_code,
                    a,
                    b,
                    c,
                    aux: 0,
                })
            }
            _ => {
                let (a, b, c) = Self::parse_abc(insn);
                Ok(Self::BC {
                    op_code,
                    a,
                    b,
                    c,
                    aux: 0,
                })
            }
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
