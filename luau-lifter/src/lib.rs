pub mod deserializer;
pub mod instruction;
mod lifter;
pub mod op_code;

pub mod builtins;

use ast::{
    inline_gotos::inline_short_gotos, local_declarations::LocalDeclarer,
    name_locals::name_locals, replace_locals::replace_locals, Traverse,
};

use ast::post_process;
use by_address::ByAddress;
use cfg::{
    function::Function,
    ssa::{
        self,
        structuring::{structure_conditionals, structure_jumps},
    },
};
use indexmap::IndexMap;

use lifter::Lifter;

use clap::Parser;
use parking_lot::Mutex;
use petgraph::algo::dominators::simple_fast;
use rayon::prelude::*;

use anyhow::anyhow;
use rustc_hash::{FxHashMap, FxHashSet};
use triomphe::Arc;
use walkdir::WalkDir;

use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    time::Instant,
};

use deserializer::bytecode::Bytecode;

/// Decompile bytecode using the `ruau-bytecode` crate as the
/// deserializer. This bypasses Topaz's custom deserializer (which
/// has trouble with the typeinfo section in luau-compile 0.728
/// outputs) and instead uses the well-tested ruau-bytecode parser.
/// The parsed proto data is then adapted to Topaz's internal
/// `Function` representation and fed to the rest of the
/// decompilation pipeline.
pub fn decompile_bytecode_via_ruau(bytecode: &[u8], encode_key: u8) -> String {
    ast::reset_local_id_counter();
    match try_adapt_chunk_via_ruau(bytecode) {
        Ok(adapted) => decompile_adapted(adapted, encode_key),
        Err(e) => format!("failed to deserialize bytecode: {e}"),
    }
}

/// Default Luau decompilation path.
///
/// Always tries the ruau-backed deserializer first (supports bytecode
/// versions 5.1–11 and bypasses typeinfo issues in newer luau-compile
/// outputs). Falls back to Topaz's native deserializer only when ruau
/// cannot parse the input (e.g. encrypted/shuffled bytecode with a
/// non-default key).
pub fn decompile_bytecode_default(bytecode: &[u8], encode_key: u8) -> String {
    if let Ok(adapted) = try_adapt_chunk_via_ruau(bytecode) {
        ast::reset_local_id_counter();
        return decompile_adapted(adapted, encode_key);
    }

    let detected_key = detect_encode_key(bytecode, encode_key);
    decompile_bytecode(bytecode, detected_key)
}

/// Result of adapting ruau-bytecode output to Topaz's internal
/// representation. Wraps the data the rest of the decompilation
/// pipeline needs.
struct AdaptedChunk {
    string_table: Vec<Vec<u8>>,
    functions: Vec<deserializer::function::Function>,
    main: u32,
}

fn try_adapt_chunk_via_ruau(bytecode: &[u8]) -> Result<AdaptedChunk, String> {
    use ruau_bytecode::{BytecodeChunk, decode_chunk};

    let chunk = match decode_chunk(bytecode) {
        Ok(c) => c,
        Err(_) => {
            // The public decode API only accepts bytecode version 7.
            // Fall back to the upstream-fixture decoder which accepts
            // versions up to 11 (the current upstream baseline).
            ruau_bytecode::decode_upstream_fixture_chunk(bytecode)
                .map_err(|e| e.to_string())?
        }
    };

    let (strings, protos, main_proto) = match chunk {
        BytecodeChunk::Valid {
            strings,
            protos,
            main_proto,
            ..
        } => (strings, protos, main_proto),
        BytecodeChunk::Error { message } => {
            return Err(String::from_utf8_lossy(&message).into_owned());
        }
    };

    // Adapt ruau-bytecode Protos to Topaz's internal Function
    // representation. This is a one-time conversion: ruau-bytecode
    // has already handled the typeinfo section internally.
    Ok(ruau_to_topaz::adapt(strings, protos, main_proto))
}

fn adapted_chunk_into_chunk(adapted: AdaptedChunk) -> deserializer::chunk::Chunk {
    deserializer::chunk::Chunk {
        string_table: adapted.string_table,
        functions: adapted.functions,
        main: adapted.main,
    }
}

fn decompile_adapted(adapted: AdaptedChunk, _encode_key: u8) -> String {
    // Build a thin shim that lets the existing decompilation
    // pipeline consume the adapted data. For now we construct a
    // Bytecode::Chunk directly.
    decompile_from_chunk(adapted_chunk_into_chunk(adapted), _encode_key)
}

fn decompile_from_chunk(chunk: deserializer::chunk::Chunk, encode_key: u8) -> String {
    // Wrap the entire decompilation in catch_unwind to prevent
    // panics in the AST/restructure pipeline from killing the
    // process. The ruau-adapted data may have subtle differences
    // from the native-deserialized data that cause downstream
    // panics.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        decompile_from_chunk_inner(chunk, encode_key)
    }))
    .unwrap_or_else(|_| {
        "-- Decompiled with Topaz\n-- Error: decompilation panicked (likely due to ruau adapter mismatch)\n".to_string()
    })
}

fn decompile_from_chunk_inner(chunk: deserializer::chunk::Chunk, encode_key: u8) -> String {
    let mut lifted = Vec::new();
    let mut stack = vec![(Arc::<Mutex<ast::Function>>::default(), chunk.main)];
    while let Some((ast_func, func_id)) = stack.pop() {
        let (function, upvalues, child_functions) =
            Lifter::lift(&chunk.functions, &chunk.string_table, func_id as usize);
        lifted.push((ast_func, function, upvalues));
        stack.extend(child_functions.into_iter().map(|(a, f)| (a.0, f as u32)));
    }

    let (main, ..) = lifted.first().unwrap().clone();
    let mut upvalues = lifted
        .into_iter()
        .map(|(ast_function, function, upvalues_in)| {
            use std::{backtrace::Backtrace, cell::RefCell, fmt::Write, panic};

            thread_local! {
                static BACKTRACE: RefCell<Option<Backtrace>> = const { RefCell::new(None) };
            }

            let function_id = function.id;
            let mut args = std::panic::AssertUnwindSafe(Some((
                ast_function.clone(),
                function,
                upvalues_in,
            )));

            let prev_hook = panic::take_hook();
            panic::set_hook(Box::new(|_| {
                let trace = Backtrace::capture();
                BACKTRACE.with(move |b| b.borrow_mut().replace(trace));
            }));
            let result = panic::catch_unwind(move || {
                let (ast_function, function, upvalues_in) = args.take().unwrap();
                decompile_function(ast_function, function, upvalues_in)
            });
            panic::set_hook(prev_hook);

            match result {
                Ok(r) => r,
                Err(e) => {
                    let panic_information = match e.downcast::<String>() {
                        Ok(v) => *v,
                        Err(e) => match e.downcast::<&str>() {
                            Ok(v) => v.to_string(),
                            _ => "Unknown Source of Error".to_owned(),
                        },
                    };

                    let mut message = String::new();
                    writeln!(message, "failed to decompile").unwrap();

                    ast_function.lock().body.extend(
                        message
                            .trim_end()
                            .split('\n')
                            .map(|s| ast::Comment::new(s.to_string()).into()),
                    );
                    (ByAddress(ast_function), Vec::new())
                }
            }
        })
        .collect::<FxHashMap<_, _>>();

    let main = ByAddress(main);
    upvalues.remove(&main);
    let mut body = Arc::try_unwrap(main.0).unwrap().into_inner().body;
    link_upvalues(&mut body, &mut upvalues);
    ast::context_naming::apply_context_naming(&mut body);
    propagate_names(&mut body);
    inline_short_gotos(&mut body);
    ast::guard_clauses::apply_guard_clauses(&mut body);
    name_locals(&mut body, true);

    format!(
        "-- Decomplied with Topaz\n-- Created by: Andrew & TouseefX\n-- Key: {}\n\n{}",
        encode_key,
        body.to_string()
    )
}

/// Adapter module: converts ruau-bytecode's parsed representation
/// to Topaz's internal `Function` / `Chunk` representation.
mod ruau_to_topaz {
    use super::deserializer::constant::Constant;
    use super::deserializer::function::{DebugInfo, DebugLocal, Function};
    use super::instruction::Instruction;
    use super::op_code::OpCode;
    use super::AdaptedChunk;
    use ruau_bytecode::{Constant as RConstant, Proto as RProto};

    pub fn adapt(
        strings: Vec<Vec<u8>>,
        protos: Vec<RProto>,
        main_proto: u32,
    ) -> AdaptedChunk {
        let mut functions = Vec::with_capacity(protos.len());
        for rproto in &protos {
            functions.push(adapt_proto(rproto));
        }
        AdaptedChunk {
            string_table: strings,
            functions,
            main: main_proto,
        }
    }

    fn adapt_proto(rp: &RProto) -> Function {
        let nop = Instruction::BC {
            op_code: OpCode::LOP_NOP,
            a: 0,
            b: 0,
            c: 0,
            aux: 0,
        };
        let mut instructions = Vec::with_capacity(rp.code.len() * 2);
        for ri in &rp.code {
            instructions.push(adapt_instruction(ri));
            if ri.aux.is_some() {
                // The native deserializer pushes a NOP placeholder
                // for every AUX word so the Lifter's PC-based
                // indexing matches the raw word count. We do the
                // same here.
                instructions.push(nop);
            }
        }
        Function {
            max_stack_size: rp.max_stack_size,
            num_parameters: rp.num_params,
            num_upvalues: rp.num_upvalues,
            is_vararg: rp.is_vararg != 0,
            instructions,
            constants: rp.constants.iter().map(adapt_constant).collect(),
            functions: rp.child_protos.clone(),
            line_defined: rp.line_defined,
            function_name: rp.debug_name,
            line_gap_log2: rp.line_info.as_ref().map(|li| li.log2_span),
            line_info_delta: rp
                .line_info
                .as_ref()
                .map(|li| li.delta_bytes.clone()),
            abs_line_info_delta: rp
                .line_info
                .as_ref()
                .map(|li| li.baseline_deltas.clone()),
            debug_info: rp.debug_info.as_ref().map(adapt_debug_info),
        }
    }

    /// Adapt a single ruau-bytecode instruction to Topaz's
    /// `Instruction` enum. We construct the variant directly from
    /// ruau's already-decoded fields rather than round-tripping
    /// through `Instruction::parse`, because `Instruction::parse`
    /// discards the aux word (sets `aux: 0`) while ruau-bytecode
    /// stores it as a separate `Option<u32>`. Opcodes like
    /// `GETIMPORT`, `JUMPXEQKN`, and `CALLFB` require the aux
    /// word to be correct.
    fn adapt_instruction(ri: &ruau_bytecode::Instruction) -> Instruction {
        use ruau_bytecode::opcodes::Opcode;
        let aux = ri.aux.unwrap_or(0);
        let op = map_opcode(ri.opcode);
        match ri.opcode {
            Opcode::JumpX | Opcode::Coverage => Instruction::E {
                op_code: op,
                e: ri.e,
            },
            Opcode::LoadN
            | Opcode::LoadK
            | Opcode::GetImport
            | Opcode::NewClosure
            | Opcode::Jump
            | Opcode::JumpBack
            | Opcode::JumpIf
            | Opcode::JumpIfNot
            | Opcode::JumpIfEq
            | Opcode::JumpIfLe
            | Opcode::JumpIfLt
            | Opcode::JumpIfNotEq
            | Opcode::JumpIfNotLe
            | Opcode::JumpIfNotLt
            | Opcode::DupTable
            | Opcode::ForNPrep
            | Opcode::ForNLoop
            | Opcode::ForGLoop
            | Opcode::ForGPrepInext
            | Opcode::ForGPrepNext
            | Opcode::NativeCall
            | Opcode::DupClosure
            | Opcode::ForGPrep
            | Opcode::JumpXEqKNil
            | Opcode::JumpXEqKB
            | Opcode::JumpXEqKN
            | Opcode::JumpXEqKS
            | Opcode::CmpProto => Instruction::AD {
                op_code: op,
                a: ri.a,
                d: ri.d,
                aux,
            },
            _ => Instruction::BC {
                op_code: op,
                a: ri.a,
                b: ri.b,
                c: ri.c,
                aux,
            },
        }
    }

    /// Map a ruau-bytecode `Opcode` to a Topaz `OpCode`. The
    /// numeric values are identical between the two implementations
    /// (both follow the upstream Luau opcode enum), so this is a
    /// straightforward 1:1 mapping. ruau-bytecode uses unprefixed
    /// variant names (Nop, Break, etc.) while Topaz uses LOP_-prefixed
    /// names (LOP_NOP, LOP_BREAK, etc.).
    fn map_opcode(op: ruau_bytecode::opcodes::Opcode) -> OpCode {
        use ruau_bytecode::opcodes::Opcode as R;
        match op {
            R::Nop => OpCode::LOP_NOP,
            R::Break => OpCode::LOP_BREAK,
            R::LoadNil => OpCode::LOP_LOADNIL,
            R::LoadB => OpCode::LOP_LOADB,
            R::LoadN => OpCode::LOP_LOADN,
            R::LoadK => OpCode::LOP_LOADK,
            R::Move => OpCode::LOP_MOVE,
            R::GetGlobal => OpCode::LOP_GETGLOBAL,
            R::SetGlobal => OpCode::LOP_SETGLOBAL,
            R::GetUpval => OpCode::LOP_GETUPVAL,
            R::SetUpval => OpCode::LOP_SETUPVAL,
            R::CloseUpvals => OpCode::LOP_CLOSEUPVALS,
            R::GetImport => OpCode::LOP_GETIMPORT,
            R::GetTable => OpCode::LOP_GETTABLE,
            R::SetTable => OpCode::LOP_SETTABLE,
            R::GetTableKs => OpCode::LOP_GETTABLEKS,
            R::SetTableKs => OpCode::LOP_SETTABLEKS,
            R::GetTableN => OpCode::LOP_GETTABLEN,
            R::SetTableN => OpCode::LOP_SETTABLEN,
            R::NewClosure => OpCode::LOP_NEWCLOSURE,
            R::NameCall => OpCode::LOP_NAMECALL,
            R::Call => OpCode::LOP_CALL,
            R::Return => OpCode::LOP_RETURN,
            R::Jump => OpCode::LOP_JUMP,
            R::JumpBack => OpCode::LOP_JUMPBACK,
            R::JumpIf => OpCode::LOP_JUMPIF,
            R::JumpIfNot => OpCode::LOP_JUMPIFNOT,
            R::JumpIfEq => OpCode::LOP_JUMPIFEQ,
            R::JumpIfLe => OpCode::LOP_JUMPIFLE,
            R::JumpIfLt => OpCode::LOP_JUMPIFLT,
            R::JumpIfNotEq => OpCode::LOP_JUMPIFNOTEQ,
            R::JumpIfNotLe => OpCode::LOP_JUMPIFNOTLE,
            R::JumpIfNotLt => OpCode::LOP_JUMPIFNOTLT,
            R::Add => OpCode::LOP_ADD,
            R::Sub => OpCode::LOP_SUB,
            R::Mul => OpCode::LOP_MUL,
            R::Div => OpCode::LOP_DIV,
            R::Mod => OpCode::LOP_MOD,
            R::Pow => OpCode::LOP_POW,
            R::AddK => OpCode::LOP_ADDK,
            R::SubK => OpCode::LOP_SUBK,
            R::MulK => OpCode::LOP_MULK,
            R::DivK => OpCode::LOP_DIVK,
            R::ModK => OpCode::LOP_MODK,
            R::PowK => OpCode::LOP_POWK,
            R::And => OpCode::LOP_AND,
            R::Or => OpCode::LOP_OR,
            R::AndK => OpCode::LOP_ANDK,
            R::OrK => OpCode::LOP_ORK,
            R::Concat => OpCode::LOP_CONCAT,
            R::Not => OpCode::LOP_NOT,
            R::Minus => OpCode::LOP_MINUS,
            R::Length => OpCode::LOP_LENGTH,
            R::NewTable => OpCode::LOP_NEWTABLE,
            R::DupTable => OpCode::LOP_DUPTABLE,
            R::SetList => OpCode::LOP_SETLIST,
            R::ForNPrep => OpCode::LOP_FORNPREP,
            R::ForNLoop => OpCode::LOP_FORNLOOP,
            R::ForGLoop => OpCode::LOP_FORGLOOP,
            R::ForGPrepInext => OpCode::LOP_FORGPREP_INEXT,
            R::ForGPrepNext => OpCode::LOP_FORGPREP_NEXT,
            R::NativeCall => OpCode::LOP_NATIVECALL,
            R::GetVarargs => OpCode::LOP_GETVARARGS,
            R::DupClosure => OpCode::LOP_DUPCLOSURE,
            R::PrepVarargs => OpCode::LOP_PREPVARARGS,
            R::LoadKx => OpCode::LOP_LOADKX,
            R::JumpX => OpCode::LOP_JUMPX,
            R::FastCall => OpCode::LOP_FASTCALL,
            R::Coverage => OpCode::LOP_COVERAGE,
            R::Capture => OpCode::LOP_CAPTURE,
            R::SubRk => OpCode::LOP_SUBRK,
            R::DivRk => OpCode::LOP_DIVRK,
            R::FastCall1 => OpCode::LOP_FASTCALL1,
            R::FastCall2 => OpCode::LOP_FASTCALL2,
            R::FastCall2K => OpCode::LOP_FASTCALL2K,
            R::ForGPrep => OpCode::LOP_FORGPREP,
            R::JumpXEqKNil => OpCode::LOP_JUMPXEQKNIL,
            R::JumpXEqKB => OpCode::LOP_JUMPXEQKB,
            R::JumpXEqKN => OpCode::LOP_JUMPXEQKN,
            R::JumpXEqKS => OpCode::LOP_JUMPXEQKS,
            R::IDiv => OpCode::LOP_IDIV,
            R::IDivK => OpCode::LOP_IDIVK,
            R::GetUdataKs => OpCode::LOP_GETUDATAKS,
            R::SetUdataKs => OpCode::LOP_SETUDATAKS,
            R::NameCallUdata => OpCode::LOP_NAMECALLUDATA,
            R::NewClassMember => OpCode::LOP_NEWCLASSMEMBER,
            R::CallFb => OpCode::LOP_CALLFB,
            R::CmpProto => OpCode::LOP_CMPPROTO,
            _ => OpCode::LOP_NOP,
        }
    }

    fn adapt_constant(rc: &RConstant) -> Constant {
        match rc {
            RConstant::Nil => Constant::Nil,
            RConstant::Boolean { value } => Constant::Boolean(*value),
            RConstant::Number { bits } => Constant::Number(f64::from_bits(*bits)),
            RConstant::Integer { value } => Constant::Integer(*value),
            RConstant::String { string } => Constant::String(*string as usize),
            RConstant::Import { import_id } => Constant::Import(*import_id as usize),
            RConstant::Table { keys } => {
                Constant::Table(keys.iter().map(|k| *k as usize).collect())
            }
            RConstant::Closure { proto } => Constant::Closure(*proto as usize),
            RConstant::Vector { bits } => {
                Constant::Vector(
                    f32::from_bits(bits[0]),
                    f32::from_bits(bits[1]),
                    f32::from_bits(bits[2]),
                    f32::from_bits(bits[3]),
                )
            }
            RConstant::TableWithConstants { entries } => {
                let entries = entries
                    .iter()
                    .map(|e| {
                        super::deserializer::constant::TableConstantEntry {
                            key: e.key as usize,
                            value_index: e.value,
                        }
                    })
                    .collect();
                Constant::TableWithConstants(entries)
            }
            RConstant::ClassShape { .. } => Constant::Nil,
        }
    }

    fn adapt_debug_info(rd: &ruau_bytecode::DebugInfo) -> DebugInfo {
        let locals = rd
            .locals
            .iter()
            .map(|l| DebugLocal {
                name_index: l.name as usize,
                start_pc: l.start_pc as usize,
                end_pc: l.end_pc as usize,
                register: l.register,
            })
            .collect();
        DebugInfo {
            locals,
            upvalue_names: rd.upvalues.iter().map(|u| *u as usize).collect(),
        }
    }
}

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    paths: Vec<String>,

    #[clap(short, long, default_value_t = 0)]
    threads: usize,

    #[clap(short, long, default_value_t = 1)]
    key: u8,
    #[clap(short, long)]
    recursive: bool,
    #[clap(short, long)]
    verbose: bool,
}


pub fn detect_encode_key(bytecode: &[u8], preferred: u8) -> u8 {
    if deserializer::deserialize(bytecode, preferred).is_ok() {
        return preferred;
    }
    for &candidate in &[1u8, 203] {
        if candidate != preferred && deserializer::deserialize(bytecode, candidate).is_ok() {
            return candidate;
        }
    }
    preferred
}

fn dump_cfgs_from_chunk(chunk: deserializer::chunk::Chunk) -> Vec<cfg::CfgSnapshot> {
    let mut out = Vec::new();
    let mut visited = rustc_hash::FxHashSet::default();
    let mut stack = vec![chunk.main];
    while let Some(func_id) = stack.pop() {
        if !visited.insert(func_id) {
            continue;
        }
        let (function, _upvalues, child_functions) =
            Lifter::lift(&chunk.functions, &chunk.string_table, func_id as usize);
        let name = if func_id == chunk.main {
            "main".to_string()
        } else {
            format!("function #{func_id}")
        };
        out.push(cfg::CfgSnapshot::from_function(&function, name));
        stack.extend(child_functions.into_iter().map(|(_, f)| f as u32));
    }
    out
}

pub fn dump_cfgs_via_ruau(bytecode: &[u8]) -> Vec<cfg::CfgSnapshot> {
    ast::reset_local_id_counter();
    let adapted = match try_adapt_chunk_via_ruau(bytecode) {
        Ok(adapted) => adapted,
        Err(_) => return Vec::new(),
    };
    dump_cfgs_from_chunk(adapted_chunk_into_chunk(adapted))
}

pub fn dump_cfgs_default(bytecode: &[u8], encode_key: u8) -> Vec<cfg::CfgSnapshot> {
    let cfgs = dump_cfgs_via_ruau(bytecode);
    if !cfgs.is_empty() {
        return cfgs;
    }
    
	let detected_key = detect_encode_key(bytecode, encode_key);
    dump_cfgs(bytecode, detected_key)
}

pub fn dump_cfgs(bytecode: &[u8], encode_key: u8) -> Vec<cfg::CfgSnapshot> {
    ast::reset_local_id_counter();
    let encode_key = detect_encode_key(bytecode, encode_key);
    let chunk = match deserializer::deserialize(bytecode, encode_key) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let chunk = match chunk {
        Bytecode::Chunk(c) => c,
        Bytecode::Error(_) => return Vec::new(),
    };

    dump_cfgs_from_chunk(chunk)
}

pub fn decompile_bytecode(bytecode: &[u8], encode_key: u8) -> String {
    ast::reset_local_id_counter();
    let encode_key = detect_encode_key(bytecode, encode_key);
    let chunk = match deserializer::deserialize(bytecode, encode_key) {
        Ok(c) => c,
        Err(e) => return format!("failed to deserialize bytecode: {e}"),
    };
    match chunk {
        Bytecode::Error(msg) => msg,
        Bytecode::Chunk(chunk) => {
            let mut lifted = Vec::new();
            let mut stack = vec![(Arc::<Mutex<ast::Function>>::default(), chunk.main)];
            while let Some((ast_func, func_id)) = stack.pop() {
                let (function, upvalues, child_functions) =
                    Lifter::lift(&chunk.functions, &chunk.string_table, func_id as usize);
                lifted.push((ast_func, function, upvalues));
                stack.extend(child_functions.into_iter().map(|(a, f)| (a.0, f as u32)));
            }

            let (main, ..) = lifted.first().unwrap().clone();
            let mut upvalues = lifted
                .into_iter()
                .map(|(ast_function, function, upvalues_in)| {
                    use std::{backtrace::Backtrace, cell::RefCell, fmt::Write, panic};

                    thread_local! {
                        static BACKTRACE: RefCell<Option<Backtrace>> = const { RefCell::new(None) };
                    }

                    let function_id = function.id;
                    let mut args = std::panic::AssertUnwindSafe(Some((
                        ast_function.clone(),
                        function,
                        upvalues_in,
                    )));

                    let prev_hook = panic::take_hook();
                    panic::set_hook(Box::new(|_| {
                        let trace = Backtrace::capture();
                        BACKTRACE.with(move |b| b.borrow_mut().replace(trace));
                    }));
                    let result = panic::catch_unwind(move || {
                        let (ast_function, function, upvalues_in) = args.take().unwrap();
                        decompile_function(ast_function, function, upvalues_in)
                    });
                    panic::set_hook(prev_hook);

                    match result {
                        Ok(r) => r,
                        Err(e) => {
                            let panic_information = match e.downcast::<String>() {
                                Ok(v) => *v,
                                Err(e) => match e.downcast::<&str>() {
                                    Ok(v) => v.to_string(),
                                    _ => "Unknown Source of Error".to_owned(),
                                },
                            };

                            let mut message = String::new();
                            writeln!(message, "failed to decompile").unwrap();

                            ast_function.lock().body.extend(
                                message
                                    .trim_end()
                                    .split('\n')
                                    .map(|s| ast::Comment::new(s.to_string()).into()),
                            );
                            (ByAddress(ast_function), Vec::new())
                        }
                    }
                })
                .collect::<FxHashMap<_, _>>();

            let main = ByAddress(main);
            upvalues.remove(&main);
            let mut body = Arc::try_unwrap(main.0).unwrap().into_inner().body;
            link_upvalues(&mut body, &mut upvalues);
            ast::context_naming::apply_context_naming(&mut body);
            propagate_names(&mut body);
            inline_short_gotos(&mut body);
            ast::guard_clauses::apply_guard_clauses(&mut body);
            name_locals(&mut body, true);

            format!(
                "-- Decomplied with Topaz\n-- Created by: Andrew & TouseefX\n-- Key: {}\n\n{}",
                encode_key,
                body.to_string()
            )
        }
    }
}

fn propagate_names(body: &mut ast::Block) {
    let mut captured = FxHashSet::default();
    collect_captured_upvalues(body, &mut captured);
    propagate_names_block(body, &captured);
}

/// Collects every local that is captured (by copy or by reference) as an
/// upvalue of some nested closure, anywhere in the function tree rooted at
/// `block`.
///
/// These locals must never have their display name overwritten by the
/// generic "copy the name from the other side of a plain assignment"
/// heuristic in `propagate_names_block`: a captured variable is shared with
/// (and semantically distinct from) whatever unrelated locals happen to live
/// in the closures that capture it, and blindly renaming it to match a
/// sibling can make two different variables print with the identical name,
/// silently corrupting the decompiled source (e.g. turning
/// `aId = idCounter` into the textually-identical-looking but broken
/// `Id = Id` once both locals are named "Id").
fn collect_captured_upvalues(block: &mut ast::Block, out: &mut FxHashSet<ast::RcLocal>) {
    for stat in &mut block.0 {
        stat.traverse_rvalues(&mut |rvalue| {
            if let ast::RValue::Closure(closure) = rvalue {
                out.extend(closure.upvalues.iter().map(|u| match u {
                    ast::Upvalue::Copy(l) | ast::Upvalue::Ref(l) => l.clone(),
                }));
                collect_captured_upvalues(&mut closure.function.lock().body, out);
            }
        });
        match stat {
            ast::Statement::If(r#if) => {
                collect_captured_upvalues(&mut r#if.then_block.lock(), out);
                collect_captured_upvalues(&mut r#if.else_block.lock(), out);
            }
            ast::Statement::While(r#while) => {
                collect_captured_upvalues(&mut r#while.block.lock(), out);
            }
            ast::Statement::Repeat(repeat) => {
                collect_captured_upvalues(&mut repeat.block.lock(), out);
            }
            ast::Statement::NumericFor(numeric_for) => {
                collect_captured_upvalues(&mut numeric_for.block.lock(), out);
            }
            ast::Statement::GenericFor(generic_for) => {
                collect_captured_upvalues(&mut generic_for.block.lock(), out);
            }
            _ => {}
        }
    }
}

fn propagate_names_block(block: &mut ast::Block, captured: &FxHashSet<ast::RcLocal>) {
    for _ in 0..2 {
        for stat in block.0.iter() {
            if let ast::Statement::Assign(assign) = stat {
                if assign.left.len() == 1 && assign.right.len() == 1 {
                    if let Some(lhs) = assign.left[0].as_local() {
                        let lhs_name = lhs.0 .0.lock().0.clone();
                        if let Some(lhs_name) = lhs_name {
                            if let ast::RValue::Local(rhs) = &assign.right[0] {
                                if !captured.contains(rhs) {
                                    let mut rhs_lock = rhs.0 .0.lock();
                                    if rhs_lock.0.is_none() {
                                        rhs_lock.0 = Some(lhs_name);
                                    }
                                }
                            }
                        } else {
                            if let ast::RValue::Local(rhs) = &assign.right[0] {
                                if !captured.contains(lhs) {
                                    let rhs_name = rhs.0 .0.lock().0.clone();
                                    if let Some(rhs_name) = rhs_name {
                                        lhs.0 .0.lock().0 = Some(rhs_name);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    block.0.retain(|stat| {
        if let ast::Statement::Assign(assign) = stat {
            if assign.prefix && assign.left.len() == 1 && assign.right.len() == 1 {
                if let Some(lhs) = assign.left[0].as_local() {
                    if let ast::RValue::Local(rhs) = &assign.right[0] {
                        let lhs_name = lhs.0 .0.lock().0.clone();
                        let rhs_name = rhs.0 .0.lock().0.clone();
                        if let (Some(ln), Some(rn)) = (&lhs_name, &rhs_name) {
                            if ln == rn {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    });

    for stat in &mut block.0 {
        stat.traverse_rvalues(&mut |rvalue| {
            if let ast::RValue::Closure(closure) = rvalue {
                propagate_names_block(&mut closure.function.lock().body, captured);
            }
        });
        match stat {
            ast::Statement::If(r#if) => {
                propagate_names_block(&mut r#if.then_block.lock(), captured);
                propagate_names_block(&mut r#if.else_block.lock(), captured);
            }
            ast::Statement::While(r#while) => {
                propagate_names_block(&mut r#while.block.lock(), captured);
            }
            ast::Statement::Repeat(repeat) => {
                propagate_names_block(&mut repeat.block.lock(), captured);
            }
            ast::Statement::NumericFor(numeric_for) => {
                propagate_names_block(&mut numeric_for.block.lock(), captured);
            }
            ast::Statement::GenericFor(generic_for) => {
                propagate_names_block(&mut generic_for.block.lock(), captured);
            }
            _ => {}
        }
    }
}

fn decompile_function(
    ast_function: Arc<Mutex<ast::Function>>,
    mut function: Function,
    upvalues_in: Vec<ast::RcLocal>,
) -> (ByAddress<Arc<Mutex<ast::Function>>>, Vec<ast::RcLocal>) {
    let (local_count, local_groups, upvalue_in_groups, upvalue_passed_groups) =
        cfg::ssa::construct(&mut function, &upvalues_in);
    let upvalue_to_group = upvalue_in_groups
        .into_iter()
        .chain(
            upvalue_passed_groups
                .into_iter()
                .map(|m| (ast::RcLocal::default(), m)),
        )
        .flat_map(|(i, g)| g.into_iter().map(move |u| (u, i.clone())))
        .collect::<IndexMap<_, _>>();

    let local_to_group = local_groups
        .into_iter()
        .enumerate()
        .flat_map(|(i, g)| g.into_iter().map(move |l| (l, i)))
        .collect::<FxHashMap<_, _>>();

    let mut changed = true;
    while changed {
        changed = false;

        let dominators = simple_fast(function.graph(), function.entry().unwrap());
        changed |= structure_jumps(&mut function, &dominators);

        ssa::inline::inline(&mut function, &local_to_group, &upvalue_to_group);

        if structure_conditionals(&mut function)

        {
            changed = true;
        }
        let mut local_map = FxHashMap::default();

        if ssa::construct::remove_unnecessary_params(&mut function, &mut local_map) {
            changed = true;
        }
        ssa::construct::apply_local_map(&mut function, local_map);
    }

    ssa::Destructor::new(
        &mut function,
        upvalue_to_group,
        upvalues_in.iter().cloned().collect(),
        local_count,
    )
    .destruct();

    let params = std::mem::take(&mut function.parameters);
    let is_variadic = function.is_variadic;
    let func_line = function.line;
    let block = Arc::new(restructure::lift(function).into());
    LocalDeclarer::default().declare_locals(

        Arc::clone(&block),
        &upvalues_in.iter().chain(params.iter()).cloned().collect(),
    );

    {
        let mut ast_function = ast_function.lock();
        ast_function.body = Arc::try_unwrap(block).unwrap().into_inner();
        
        // Apply post-processing to this function's body
        post_process::apply_all(&mut ast_function.body);
        ast_function.parameters = params;
        ast_function.is_variadic = is_variadic;
        ast_function.line = func_line;
    }
    (ByAddress(ast_function), upvalues_in)
}

fn link_upvalues(
    body: &mut ast::Block,
    upvalues: &mut FxHashMap<ByAddress<Arc<Mutex<ast::Function>>>, Vec<ast::RcLocal>>,
) {
    for stat in &mut body.0 {
        stat.traverse_rvalues(&mut |rvalue| {
            if let ast::RValue::Closure(closure) = rvalue {
                let old_upvalues = &upvalues[&closure.function];
                let mut function = closure.function.lock();

                let mut local_map =
                    FxHashMap::with_capacity_and_hasher(old_upvalues.len(), Default::default());
                for (old, new) in
                    old_upvalues
                        .iter()
                        .zip(closure.upvalues.iter().map(|u| match u {
                            ast::Upvalue::Copy(l) | ast::Upvalue::Ref(l) => l,
                        }))
                {
                    let old_name = old.0.0.lock().0.clone();
                    if let Some(ref name) = old_name {
                        if !ast::name_locals::is_synthetic_name(name) {
                            let mut new_lock = new.0.0.lock();
                            if new_lock.0.is_none()
                                || new_lock
                                    .0
                                    .as_ref()
                                    .map(|s| ast::name_locals::is_synthetic_name(s))
                                    .unwrap_or(true)
                            {
                                new_lock.0 = Some(name.clone());
                            }
                        }
                    }
                    local_map.insert(old.clone(), new.clone());
                }
                link_upvalues(&mut function.body, upvalues);
                replace_locals(&mut function.body, &local_map);
            }
        });
        match stat {
            ast::Statement::If(r#if) => {
                link_upvalues(&mut r#if.then_block.lock(), upvalues);
                link_upvalues(&mut r#if.else_block.lock(), upvalues);
            }
            ast::Statement::While(r#while) => {
                link_upvalues(&mut r#while.block.lock(), upvalues);
            }
            ast::Statement::Repeat(repeat) => {
                link_upvalues(&mut repeat.block.lock(), upvalues);
            }
            ast::Statement::NumericFor(numeric_for) => {
                link_upvalues(&mut numeric_for.block.lock(), upvalues);
            }
            ast::Statement::GenericFor(generic_for) => {
                link_upvalues(&mut generic_for.block.lock(), upvalues);
            }
            _ => {}
        }
    }
}
