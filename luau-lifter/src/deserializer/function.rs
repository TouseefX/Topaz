use nom::{
    number::complete::{le_u32, le_u8},
    IResult,
};
use nom_leb128::leb128_usize;

use super::{
    constant::Constant,
    list::{parse_list, parse_list_len},
};
use crate::{instruction::*, op_code::OpCode};


#[derive(Debug, Clone)]
pub struct DebugLocal {
    
    pub name_index: usize,
    
    pub start_pc: usize,
    
    pub end_pc: usize,
    
    pub register: u8,
}


#[derive(Debug, Clone, Default)]
pub struct DebugInfo {
    pub locals: Vec<DebugLocal>,
    
    pub upvalue_names: Vec<usize>,
}


const OPCODES_WITH_AUX: &[OpCode] = &[
    OpCode::LOP_GETGLOBAL,
    OpCode::LOP_SETGLOBAL,
    OpCode::LOP_GETIMPORT,
    OpCode::LOP_GETTABLEKS,
    OpCode::LOP_SETTABLEKS,
    OpCode::LOP_NAMECALL,
    OpCode::LOP_JUMPIFEQ,
    OpCode::LOP_JUMPIFLE,
    OpCode::LOP_JUMPIFLT,
    OpCode::LOP_JUMPIFNOTEQ,
    OpCode::LOP_JUMPIFNOTLE,
    OpCode::LOP_JUMPIFNOTLT,
    OpCode::LOP_NEWTABLE,
    OpCode::LOP_SETLIST,
    OpCode::LOP_FORGLOOP,
    OpCode::LOP_LOADKX,
    OpCode::LOP_FASTCALL2,
    OpCode::LOP_FASTCALL2K,
    OpCode::LOP_FASTCALL3,
    OpCode::LOP_JUMPXEQKNIL,
    OpCode::LOP_JUMPXEQKB,
    OpCode::LOP_JUMPXEQKN,
    OpCode::LOP_JUMPXEQKS,
];


#[derive(Debug)]
pub struct Function {
    pub max_stack_size: u8,
    pub num_parameters: u8,
    pub num_upvalues: u8,
    pub is_vararg: bool,
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Constant>,
    
    pub functions: Vec<usize>,
    pub line_defined: usize,
    
    pub function_name: usize,

    
    pub line_gap_log2: Option<u8>,
    pub line_info_delta: Option<Vec<u8>>,
    pub abs_line_info_delta: Option<Vec<u32>>,

    
    pub debug_info: Option<DebugInfo>,
}

impl Function {
    

    fn decode_instructions(raw: &[u32], encode_key: u8) -> Result<Vec<Instruction>, nom::error::ErrorKind> {
        let mut out = Vec::with_capacity(raw.len());
        let mut pc = 0;

        while pc < raw.len() {
            let ins = Instruction::parse(raw[pc], encode_key)?;
            let op = match ins {
                Instruction::BC { op_code, .. }
                | Instruction::AD { op_code, .. }
                | Instruction::E { op_code, .. } => op_code,
            };

            if OPCODES_WITH_AUX.contains(&op) {
                
                
                let Some(&aux) = raw.get(pc + 1) else {
                    return Err(nom::error::ErrorKind::Eof);
                };
                pc += 2;

                
                match ins {
                    Instruction::BC { op_code, a, b, c, .. } => {
                        out.push(Instruction::BC { op_code, a, b, c, aux });
                    }
                    Instruction::AD { op_code, a, d, .. } => {
                        out.push(Instruction::AD { op_code, a, d, aux });
                    }
                    _ => unreachable!(),
                }

                
                out.push(Instruction::BC {
                    op_code: OpCode::LOP_NOP,
                    a: 0, b: 0, c: 0, aux: 0,
                });
            } else {
                out.push(ins);
                pc += 1;
            }
        }

        Ok(out)
    }

    
    fn parse_line_info(
        input: &[u8],
        has_line_info: u8,
        instruction_count: usize,
    ) -> IResult<&[u8], (Option<u8>, Option<Vec<u8>>, Option<Vec<u32>>)> {
        if has_line_info == 0 {
            return Ok((input, (None, None, None)));
        }

        let (input, line_gap_log2) = le_u8(input)?;
        let (input, line_info_delta) = parse_list_len(input, le_u8, instruction_count)?;
        let abs_line_count = ((instruction_count - 1) >> line_gap_log2) + 1;
        let (input, abs_line_info_delta) = parse_list_len(input, le_u32, abs_line_count)?;

        Ok((
            input,
            (Some(line_gap_log2), Some(line_info_delta), Some(abs_line_info_delta)),
        ))
    }

    
    fn parse_debug_info(input: &[u8]) -> IResult<&[u8], Option<DebugInfo>> {
        let (input, has_debug) = le_u8(input)?;
        if has_debug == 0 {
            return Ok((input, None));
        }

        
        let (mut input, num_locals) = leb128_usize(input)?;
        let mut locals = Vec::with_capacity(num_locals);
        for _ in 0..num_locals {
            let (rest, name_index) = leb128_usize(input)?;
            let (rest, start_pc) = leb128_usize(rest)?;
            let (rest, end_pc) = leb128_usize(rest)?;
            let (rest, register) = le_u8(rest)?;
            input = rest;
            locals.push(DebugLocal { name_index, start_pc, end_pc, register });
        }

        
        let (mut input, num_upvalues) = leb128_usize(input)?;
        let mut upvalue_names = Vec::with_capacity(num_upvalues);
        for _ in 0..num_upvalues {
            let (rest, name_index) = leb128_usize(input)?;
            input = rest;
            upvalue_names.push(name_index);
        }

        Ok((input, Some(DebugInfo { locals, upvalue_names })))
    }

    
    pub(crate) fn parse(input: &[u8], encode_key: u8) -> IResult<&[u8], Self> {
        
        let (input, max_stack_size) = le_u8(input)?;
        let (input, num_parameters) = le_u8(input)?;
        let (input, num_upvalues) = le_u8(input)?;
        let (input, is_vararg) = le_u8(input)?;
        let (input, _flags) = le_u8(input)?;
        let (input, _type_info) = parse_list(input, le_u8)?;

        
        let (input, raw_instructions) = parse_list(input, le_u32)?;
        let instructions = Self::decode_instructions(&raw_instructions, encode_key)
            .map_err(|kind| nom::Err::Error(nom::error::Error::new(input, kind)))?;
        let (input, constants) = parse_list(input, Constant::parse)?;
        let (input, functions) = parse_list(input, leb128_usize)?;

        
        let (input, line_defined) = leb128_usize(input)?;
        let (input, function_name) = leb128_usize(input)?;

        
        let (input, has_line_info) = le_u8(input)?;
        let (input, (line_gap_log2, line_info_delta, abs_line_info_delta)) =
            Self::parse_line_info(input, has_line_info, raw_instructions.len())?;

        
        let (input, debug_info) = Self::parse_debug_info(input)?;

        Ok((
            input,
            Self {
                max_stack_size,
                num_parameters,
                num_upvalues,
                is_vararg: is_vararg != 0,
                instructions,
                constants,
                functions,
                line_defined,
                function_name,
                line_gap_log2,
                line_info_delta,
                abs_line_info_delta,
                debug_info,
            },
        ))
    }
}
