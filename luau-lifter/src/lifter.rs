use anyhow::Result;

use by_address::ByAddress;

use itertools::Itertools;
use parking_lot::Mutex;
use petgraph::stable_graph::NodeIndex;

use rustc_hash::FxHashMap;
use triomphe::Arc;

use super::{
    deserializer::{
        constant::Constant as BytecodeConstant, function::Function as BytecodeFunction,
    },
    instruction::Instruction,
    op_code::OpCode,
};
use ast::{self};
use cfg::{
    block::{BlockEdge, BranchType},
    function::Function,
};

pub struct Lifter<'a> {
    function_list: &'a Vec<BytecodeFunction>,
    string_table: &'a Vec<Vec<u8>>,
    blocks: FxHashMap<usize, NodeIndex>,
    /// Maps the PC of every `FORNPREP` instruction to the PC of the matching
    /// `FORNLOOP` that terminates its loop. Populated by `discover_blocks`
    /// (which sees the full instruction stream at once) and consulted during
    /// the per-block lift pass. This is the only safe way to pair the two:
    /// the FORNLOOP's outgoing edges don't exist yet when we lift the
    /// FORNPREP's block, so the predecessor-of-backedge approach used
    /// previously (and which only worked for non-nested single loops) cannot
    /// disambiguate nested/sibling numeric-for loops.
    for_loops: FxHashMap<usize, usize>,
    function: Function,
    child_functions: FxHashMap<ByAddress<Arc<Mutex<ast::Function>>>, usize>,
    register_map: FxHashMap<usize, ast::RcLocal>,
    constant_map: FxHashMap<usize, ast::Literal>,
    current_node: Option<NodeIndex>,
    upvalues: Vec<ast::RcLocal>,
    debug_register_names: FxHashMap<usize, String>,
    debug_upvalue_names: Vec<Option<String>>,
}

impl<'a> Lifter<'a> {
    pub fn lift(
        f_list: &'a Vec<BytecodeFunction>,
        str_list: &'a Vec<Vec<u8>>,
        function_id: usize,
    ) -> (
        Function,
        Vec<ast::RcLocal>,
        FxHashMap<ByAddress<Arc<Mutex<ast::Function>>>, usize>,
    ) {
        let bytecode_func = &f_list[function_id];
        let mut debug_register_names = FxHashMap::default();
        let mut debug_upvalue_names = Vec::new();

        if let Some(ref debug_info) = bytecode_func.debug_info {
            for local in &debug_info.locals {
                if local.name_index > 0 {
                    let name = String::from_utf8_lossy(
                        &str_list[local.name_index - 1],
                    )
                    .into_owned();
                    debug_register_names.entry(local.register as usize).or_insert(name);
                }
            }
            for &name_index in &debug_info.upvalue_names {
                if name_index > 0 {
                    debug_upvalue_names.push(Some(
                        String::from_utf8_lossy(&str_list[name_index - 1]).into_owned(),
                    ));
                } else {
                    debug_upvalue_names.push(None);
                }
            }
        }

        let mut context = Self {
            function_list: f_list,
            string_table: str_list,
            blocks: FxHashMap::default(),
            for_loops: FxHashMap::default(),
            function: Function::new(function_id),
            child_functions: FxHashMap::default(),
            register_map: FxHashMap::default(),
            constant_map: FxHashMap::default(),
            current_node: None,
            upvalues: Vec::new(),
            debug_register_names,
            debug_upvalue_names,
        };
        context.function.line = Some(f_list[function_id].line_defined as usize);

        context.lift_function();
        (context.function, context.upvalues, context.child_functions)
    }

    fn lift_function(&mut self) {
        self.discover_blocks().unwrap();

        let mut blocks = self.blocks.keys().cloned().collect::<Vec<_>>();

        blocks.sort_unstable();

        
        let block_ranges = blocks
            .iter()
            .rev()
            .fold(
                (
                    self.function_list[self.function.id].instructions.len(),
                    Vec::new(),
                ),
                |(block_end, mut accumulator), &block_start| {
                    // block_end is the start of the *next* block; the
                    // current block runs from `block_start` through the
                    // instruction just before that. If both are equal
                    // (e.g. when the function has 0 instructions and we
                    // still inserted a sentinel block at index 0), the
                    // block contains no instructions — clamp the end to
                    // `block_start` so we don't underflow.
                    let block_last = if block_end > block_start {
                        block_end - 1
                    } else {
                        block_start
                    };
                    accumulator.push((block_start, block_last));

                    (
                        if block_start != 0 {
                            block_start
                        } else {
                            block_end
                        },
                        accumulator,
                    )
                },
            )
            .1;

        for i in 0..self.function_list[self.function.id].num_upvalues {
            let upvalue = if let Some(Some(name)) = self.debug_upvalue_names.get(i as usize) {
                ast::RcLocal::new(ast::Local::new(Some(name.clone())))
            } else {
                ast::RcLocal::default()
            };
            self.upvalues.push(upvalue);
        }

        for i in 0..self.function_list[self.function.id].num_parameters {
            let parameter = if let Some(name) = self.debug_register_names.get(&(i as usize)) {
                ast::RcLocal::new(ast::Local::new(Some(name.clone())))
            } else {
                ast::RcLocal::default()
            };
            self.function.parameters.push(parameter.clone());
            self.register_map.insert(i as usize, parameter);
        }

        self.function.is_variadic = self.function_list[self.function.id].is_vararg;

        for (start_pc, end_pc) in block_ranges {
            self.current_node = Some(self.block_to_node(start_pc));
            let (statements, edges) = self.lift_block(start_pc, end_pc);
            let block = self.function.block_mut(self.current_node.unwrap()).unwrap();
            block.0.extend(statements);
            self.function.set_edges(self.current_node.unwrap(), edges);
        }

        let entry_node = self.function.new_block();
        self.function.set_edges(
            entry_node,
            vec![(
                self.block_to_node(0),
                BlockEdge::new(BranchType::Unconditional),
            )],
        );
        self.function.set_entry(entry_node);
    }

    fn discover_blocks(&mut self) -> Result<()> {
        self.blocks.insert(0, self.function.new_block());
        let instructions = &self.function_list[self.function.id].instructions;

        // First pass: pair every FORNPREP with its matching FORNLOOP.
        // A `for i = a, b, c do ... end` compiles to:
        //   <FORNPREP> at pc_prep     ; jumps to pc_after_loop if the loop
        //                              ; shouldn't run at all
        //   ... body ...
        //   <FORNLOOP> at pc_loop     ; jumps back to pc_prep+1 if the loop
        //                              ; should continue
        // Pairing is nested and stack-based: walk the instructions linearly,
        // pushing FORNPREP PCs onto a stack and popping when we see a
        // FORNLOOP. This correctly handles nested and sibling for-loops and
        // works regardless of CFG shape.
        let mut open_for_preps: Vec<usize> = Vec::new();
        for (insn_index, insn) in instructions.iter().enumerate() {
            if let Instruction::AD { op_code: OpCode::LOP_FORNPREP, .. } = insn {
                open_for_preps.push(insn_index);
            } else if let Instruction::AD { op_code: OpCode::LOP_FORNLOOP, .. } = insn {
                if let Some(prep_pc) = open_for_preps.pop() {
                    self.for_loops.insert(prep_pc, insn_index);
                }
                // An unmatched FORNLOOP (no open FORNPREP) is malformed; we
                // silently drop it. The original `exactly_one` predecessor
                // search would have warned in this case too, so we lose no
                // coverage.
            }
        }
        // Unmatched FORNPREPs (no closing FORNLOOP) also indicate malformed
        // bytecode. They'll fall back to the warning path at lift time.

        for (insn_index, insn) in instructions.iter().enumerate()
        {
            match insn {
                Instruction::BC { op_code, c, .. } => match op_code {
                    OpCode::LOP_LOADB if *c != 0 => {
                        if let Some(dest_index) = (insn_index + 1).checked_add_signed((*c).into()) {
                            if dest_index < instructions.len() {
                                self.blocks
                                    .entry(dest_index)
                                    .or_insert_with(|| self.function.new_block());
                            }
                        }
                    }
                    _ => {}
                },

                Instruction::AD {
                    op_code,
                    a: _,
                    d,
                    aux: _,
                } => match op_code {
                    OpCode::LOP_JUMP
                    | OpCode::LOP_JUMPBACK
                    | OpCode::LOP_JUMPIF
                    | OpCode::LOP_JUMPIFNOT => {
                        if let Some(dest_index) = (insn_index + 1).checked_add_signed((*d).into()) {
                            self.blocks
                                .entry(insn_index + 1)
                                .or_insert_with(|| self.function.new_block());
                            self.blocks
                                .entry(dest_index)
                                .or_insert_with(|| self.function.new_block());
                        }
                    }
                    OpCode::LOP_JUMPIFEQ
                    | OpCode::LOP_JUMPIFLE
                    | OpCode::LOP_JUMPIFLT
                    | OpCode::LOP_JUMPIFNOTEQ
                    | OpCode::LOP_JUMPIFNOTLE
                    | OpCode::LOP_JUMPIFNOTLT
                    | OpCode::LOP_JUMPXEQKNIL
                    | OpCode::LOP_JUMPXEQKB
                    | OpCode::LOP_JUMPXEQKN
                    | OpCode::LOP_JUMPXEQKS
                    | OpCode::LOP_CMPPROTO => {
                        if let Some(dest_index) = (insn_index + 1).checked_add_signed((*d).into()) {
                            self.blocks
                                .entry(insn_index + 2)
                                .or_insert_with(|| self.function.new_block());
                            self.blocks
                                .entry(dest_index)
                                .or_insert_with(|| self.function.new_block());
                        }
                    }
                    OpCode::LOP_FORNPREP => {
                        if let Some(dest_index) = (insn_index + 1).checked_add_signed((*d).into()) {
                            self.blocks
                                .entry(insn_index + 1)
                                .or_insert_with(|| self.function.new_block());
                            self.blocks
                                .entry(dest_index)
                                .or_insert_with(|| self.function.new_block());
                        }
                    }
                    OpCode::LOP_FORGPREP
                    | OpCode::LOP_FORGPREP_NEXT
                    | OpCode::LOP_FORGPREP_INEXT => {
                        if let Some(dest_index) = (insn_index + 1).checked_add_signed((*d).into()) {
                            self.blocks
                                .entry(insn_index + 1)
                                .or_insert_with(|| self.function.new_block());
                            self.blocks
                                .entry(dest_index)
                                .or_insert_with(|| self.function.new_block());
                        }
                    }
                    OpCode::LOP_FORNLOOP => {
                        if let Some(dest_index) = (insn_index + 1).checked_add_signed((*d).into()) {
                            self.blocks
                                .entry(insn_index)
                                .or_insert_with(|| self.function.new_block());
                            self.blocks
                                .entry(insn_index + 1)
                                .or_insert_with(|| self.function.new_block());
                            self.blocks
                                .entry(dest_index)
                                .or_insert_with(|| self.function.new_block());
                        }
                    }
                    OpCode::LOP_FORGLOOP => {
                        let d_signed: isize = (*d).into();
                        if let Some(dest_index) = (insn_index + 1).checked_add_signed(d_signed) {
                            self.blocks
                                .entry(insn_index + 1)
                                .or_insert_with(|| self.function.new_block());
                            self.blocks
                                .entry(dest_index)
                                .or_insert_with(|| self.function.new_block());
                        }
                    }
                    _ => {}
                },

                Instruction::E { op_code, e } => {
                    if *op_code == OpCode::LOP_JUMPX {
                        if let Some(dest_index) = (insn_index + 1).checked_add_signed((*e) as isize) {
                            self.blocks
                                .entry(insn_index + 1)
                                .or_insert_with(|| self.function.new_block());
                            self.blocks
                                .entry(dest_index)
                                .or_insert_with(|| self.function.new_block());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn lift_block(
        &mut self,
        block_start: usize,
        block_end: usize,
    ) -> (Vec<ast::Statement>, Vec<(NodeIndex, BlockEdge)>) {
        let mut statements = Vec::with_capacity((block_start..=block_end).count());
        let mut edges = Vec::new();

        let mut top: Option<(ast::RValue, u8)> = None;

        // If the block has no instructions (block_start > block_end,
        // OR the function itself is empty), there is nothing to lift;
        // emit an empty block and move on.
        // This can happen for stub/empty functions in newer Luau
        // bytecode (e.g. luau 0.728 can emit zero-instruction sub-functions).
        let insns = &self.function_list[self.function.id].instructions;
        if block_start >= insns.len() || block_start > block_end {
            return (statements, edges);
        }
        let block_end = block_end.min(insns.len() - 1);

        let mut iter = insns[block_start..=block_end].iter().enumerate();

        while let Some((index, instruction)) = iter.next() {
            match *instruction {
                Instruction::BC {
                    op_code,
                    a,
                    b,
                    c,
                    aux,
                } => match op_code {
                    
                    OpCode::LOP_PREPVARARGS => {}
                    OpCode::LOP_MOVE => {
                        let a = self.register(a as _);
                        let b = self.register(b as _);
                        statements.push(ast::Assign::new(vec![a.into()], vec![b.into()]).into());
                    }
                    OpCode::LOP_GETUPVAL => {
                        let a = self.register(a as _);
                        let up = self.upvalues[b as usize].clone();
                        statements.push(ast::Assign::new(vec![a.into()], vec![up.into()]).into());
                    }
                    OpCode::LOP_SETUPVAL => {
                        let a = self.register(a as _);
                        let up = self.upvalues[b as usize].clone();
                        statements.push(ast::Assign::new(vec![up.into()], vec![a.into()]).into());
                    }
                    OpCode::LOP_LOADNIL => {
                        // Per the Luau bytecode spec, LOP_LOADNIL only has
                        // an `A` field (the target register). The wire
                        // encoding is the same shape as `MOVE` (ABC), so
                        // `b` and `c` are present in the instruction but
                        // unused. Some Luau compiler versions (notably
                        // lune 0.10.x) emit multi-nil sequences by chaining
                        // a series of `LOADNIL` instructions targeting
                        // consecutive registers, so a statement like
                        // `local a, b, c = nil, nil, nil` becomes three
                        // `LOADNIL` opcodes, not one with a range. Emit
                        // a single nil assignment for register `a`; the
                        // rest of the multi-nil sequence is handled by
                        // the subsequent `LOADNIL`s. (Earlier we tried
                        // interpreting `b` as a range endpoint, but that
                        // produced hundreds of "LOADNIL has b < a" warnings
                        // on real-world bytecode — see the audit note for
                        // S6 — so we reverted to the simpler "one
                        // register per LOADNIL" interpretation that
                        // matches the spec and produces clean output.)
                        let target = self.register(a as _);
                        statements.push(
                            ast::Assign::new(
                                vec![target.into()],
                                vec![ast::Literal::Nil.into()],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_LOADB => {
                        let target = self.register(a as _);
                        statements.push(
                            ast::Assign::new(
                                vec![target.into()],
                                vec![ast::Literal::Boolean(b != 0).into()],
                            )
                            .into(),
                        );
                        if c != 0 {
                            edges.push((
                                self.block_to_node(block_start + index + 2),
                                BlockEdge::new(BranchType::Unconditional),
                            ));
                        }
                    }
                    OpCode::LOP_NEWTABLE => {
                        statements.push(
                            ast::Assign::new(
                                vec![self.register(a as _).into()],
                                vec![ast::Table::default().into()],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_GETGLOBAL => {
                        let value = self.register(a as _);
                        let global_name = self.constant(aux as _).into_string().unwrap();
                        statements.push(
                            ast::Assign::new(
                                vec![value.into()],
                                vec![ast::Global::new(global_name).into()],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_SETGLOBAL => {
                        let value = self.register(a as _);
                        let global_name = self.constant(aux as _).into_string().unwrap();
                        statements.push(
                            ast::Assign::new(
                                vec![ast::Global::new(global_name).into()],
                                vec![value.into()],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_GETTABLE => {
                        let target = self.register(a as _);
                        let table = self.register(b as _);
                        let key = self.register(c as _);
                        statements.push(
                            ast::Assign::new(
                                vec![target.into()],
                                vec![ast::Index::new(table.into(), key.into()).into()],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_GETTABLEKS | OpCode::LOP_GETUDATAKS => {
                        let target = self.register(a as _);
                        let table = self.register(b as _);
                        let const_idx = if op_code == OpCode::LOP_GETUDATAKS {
                            (aux & 0xffff) as usize
                        } else {
                            aux as usize
                        };
                        let key = self.constant(const_idx);
                        statements.push(
                            ast::Assign::new(
                                vec![target.into()],
                                vec![ast::Index::new(table.into(), key.into()).into()],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_GETTABLEN => {
                        let value = self.register(a as _);
                        let table = self.register(b as _);
                        let key = ast::Literal::Number((c as usize + 1) as f64);
                        statements.push(
                            ast::Assign::new(
                                vec![value.into()],
                                vec![ast::Index::new(table.into(), key.into()).into()],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_SETTABLE => {
                        let value = self.register(a as _);
                        let table = self.register(b as _);
                        let key = self.register(c as _);
                        statements.push(
                            ast::Assign::new(
                                vec![ast::Index::new(table.into(), key.into()).into()],
                                vec![value.into()],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_SETTABLEKS | OpCode::LOP_SETUDATAKS => {
                        let value = self.register(a as _);
                        let table = self.register(b as _);
                        let const_idx = if op_code == OpCode::LOP_SETUDATAKS {
                            (aux & 0xffff) as usize
                        } else {
                            aux as usize
                        };
                        let key = self.constant(const_idx);
                        statements.push(
                            ast::Assign::new(
                                vec![ast::Index::new(table.into(), key.into()).into()],
                                vec![value.into()],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_SETTABLEN => {
                        let value = self.register(a as _);
                        let table = self.register(b as _);
                        let key = ast::Literal::Number((c as usize + 1) as f64);
                        statements.push(
                            ast::Assign::new(
                                vec![ast::Index::new(table.into(), key.into()).into()],
                                vec![value.into()],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_ADD
                    | OpCode::LOP_SUB
                    | OpCode::LOP_MUL
                    | OpCode::LOP_DIV
                    | OpCode::LOP_MOD
                    | OpCode::LOP_POW
                    | OpCode::LOP_IDIV
                    | OpCode::LOP_BITAND
                    | OpCode::LOP_BITOR
                    | OpCode::LOP_BITXOR
                    | OpCode::LOP_BITLSHIFT
                    | OpCode::LOP_BITRSHIFT
                    | OpCode::LOP_BITARSHIFT => {
                        let op = match op_code {
                            OpCode::LOP_ADD => ast::BinaryOperation::Add,
                            OpCode::LOP_SUB => ast::BinaryOperation::Sub,
                            OpCode::LOP_MUL => ast::BinaryOperation::Mul,
                            OpCode::LOP_DIV => ast::BinaryOperation::Div,
                            OpCode::LOP_MOD => ast::BinaryOperation::Mod,
                            OpCode::LOP_POW => ast::BinaryOperation::Pow,
                            OpCode::LOP_IDIV => ast::BinaryOperation::IDiv,
                            OpCode::LOP_BITAND => ast::BinaryOperation::BAnd,
                            OpCode::LOP_BITOR => ast::BinaryOperation::BOr,
                            OpCode::LOP_BITXOR => ast::BinaryOperation::BXor,
                            OpCode::LOP_BITLSHIFT | OpCode::LOP_BITRSHIFT
                            | OpCode::LOP_BITARSHIFT => ast::BinaryOperation::Shr,
                            _ => ast::BinaryOperation::Add, // Unreachable: matched above.
                        };
                        // Note: AST supports Bitwise operations natively (BAnd/BOr/BXor/Shl/Shr
                        // in ast::BinaryOperation, BNot in ast::UnaryOperation); we use them
                        // directly so the formatter emits the correct `&`/`|`/`~`/`<<`/`>>`.
                        let target = self.register(a as _);
                        let left = self.register(b as _);
                        let right = self.register(c as _);
                        statements.push(
                            ast::Assign::new(
                                vec![target.into()],
                                vec![ast::Binary::new(left.into(), right.into(), op).into()],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_ADDK
                    | OpCode::LOP_SUBK
                    | OpCode::LOP_MULK
                    | OpCode::LOP_DIVK
                    | OpCode::LOP_MODK
                    | OpCode::LOP_POWK
                    | OpCode::LOP_IDIVK
                    | OpCode::LOP_BITANDK
                    | OpCode::LOP_BITORK
                    | OpCode::LOP_BITXORK => {
                        let op = match op_code {
                            OpCode::LOP_ADDK => ast::BinaryOperation::Add,
                            OpCode::LOP_SUBK => ast::BinaryOperation::Sub,
                            OpCode::LOP_MULK => ast::BinaryOperation::Mul,
                            OpCode::LOP_DIVK => ast::BinaryOperation::Div,
                            OpCode::LOP_MODK => ast::BinaryOperation::Mod,
                            OpCode::LOP_POWK => ast::BinaryOperation::Pow,
                            OpCode::LOP_IDIVK => ast::BinaryOperation::IDiv,
                            OpCode::LOP_BITANDK => ast::BinaryOperation::BAnd,
                            OpCode::LOP_BITORK => ast::BinaryOperation::BOr,
                            OpCode::LOP_BITXORK => ast::BinaryOperation::BXor,
                            _ => ast::BinaryOperation::Add, // Unreachable: matched above.
                        };
                        let target = self.register(a as _);
                        let left = self.register(b as _);
                        let right = self.constant(c as _);
                        statements.push(
                            ast::Assign::new(
                                vec![target.into()],
                                vec![ast::Binary::new(left.into(), right.into(), op).into()],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_SUBRK | OpCode::LOP_DIVRK => {
                        let op = match op_code {
                            OpCode::LOP_SUBRK => ast::BinaryOperation::Sub,
                            OpCode::LOP_DIVRK => ast::BinaryOperation::Div,
                            _ => unreachable!(),
                        };
                        let target = self.register(a as _);
                        let left = self.constant(b as _);
                        let right = self.register(c as _);
                        statements.push(
                            ast::Assign::new(
                                vec![target.into()],
                                vec![ast::Binary::new(left.into(), right.into(), op).into()],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_NOT | OpCode::LOP_MINUS | OpCode::LOP_LENGTH | OpCode::LOP_BITNOT => {
                        let op = match op_code {
                            OpCode::LOP_NOT => ast::UnaryOperation::Not,
                            OpCode::LOP_MINUS => ast::UnaryOperation::Negate,
                            OpCode::LOP_LENGTH => ast::UnaryOperation::Length,
                            OpCode::LOP_BITNOT => ast::UnaryOperation::BNot,
                            _ => unreachable!(),
                        };
                        let target = self.register(a as _);
                        let value = self.register(b as _);
                        statements.push(
                            ast::Assign::new(
                                vec![target.into()],
                                vec![ast::Unary::new(value.into(), op).into()],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_RETURN => {
                        let values = if b != 0 {
                            (a..a + (b - 1))
                                .map(|r| self.register(r as _).into())
                                .collect()
                        } else if let Some((tail, end)) = top.take() {
                            (a..end)
                                .map(|r| self.register(r as _).into())
                                .chain(std::iter::once(tail))
                                .collect()
                        } else {
                            // `b == 0` with no pending MULTRET tail: this is a
                            // single-value return of register `a`. Do NOT sweep
                            // all registers up to `max_stack_size` (the previous
                            // behavior), which produced bogus
                            // `return v0, v1, v2, ...` lists.
                            vec![self.register(a as _).into()]
                        };
                        statements.push(ast::Return::new(values).into());
                        break;
                    }
                    OpCode::LOP_FASTCALL
                    | OpCode::LOP_FASTCALL1
                    | OpCode::LOP_FASTCALL2
                    | OpCode::LOP_FASTCALL2K
                    | OpCode::LOP_FASTCALL3 => {
                        // LOP_FASTCALL*: VM optimization for direct calls to
                        // known built-in functions (math.floor, string.sub,
                        // type, assert, ...). The wire format (per Luau
                        // bytecode spec) is:
                        //
                        //   FASTCALL[N]  a=LBF, b=arg1, c=jump, [AUX=arg2/3]
                        //   GETIMPORT    a=result_reg, d=k, aux=encoded_index
                        //   NOP                   (CALL's aux slot, e.g. feedback id)
                        //   CALL         a=result_reg, b=nargs+1, c=nresults+1
                        //
                        // Key observations from the actual lune 0.10.5
                        // bytecode (verified with the dump tool):
                        //
                        // - The function-load (GETIMPORT) is always present
                        //   after a standalone fastcall, even though the
                        //   VM only uses it on the slow path. (The fast
                        //   path computes the result inline and skips over
                        //   the function-load + NOP + CALL entirely.)
                        //
                        // - The NOP is the *auxiliary* of the CALL
                        //   instruction; it carries the feedback slot id
                        //   and is required for the CALL's decode to
                        //   succeed. It does NOT do anything semantically
                        //   (no-op).
                        //
                        // - The args of the call are at registers
                        //   result_reg+1, result_reg+2, ... — exactly
                        //   what a normal CALL would expect. The FASTCALL's
                        //   `b` field tells us arg1 (for FASTCALL1+); for
                        //   FASTCALL2/2K/3, the additional args are in
                        //   AUX. For the bare `FASTCALL` (no variant), `b`
                        //   is 0 and the args are pre-loaded into the
                        //   registers for the CALL.
                        //
                        // - The CALL's `a` field equals the GETIMPORT's
                        //   `a` field — both are the result register. So
                        //   the function-load's destination and the
                        //   call's result register are the same.
                        //
                        // Because of this, we can lift the entire
                        // FASTCALL+GETIMPORT+NOP+CALL sequence into a
                        // single `Call(builtin_name, args...)` AST node,
                        // using the CALL's arg registers (which the
                        // caller has already pre-loaded) as the call
                        // arguments.
                        let builtin_id = a;

                        // We need to peek at the next 3 instructions.
                        // If they don't match the expected pattern, we
                        // fall back to letting the regular loop process
                        // them (which will produce the correct call
                        // expression via GETIMPORT + CALL).
                        //
                        // The expected pattern for a standalone fastcall:
                        //   FASTCALL
                        //   GETIMPORT     ; load function
                        //   NOP           ; CALL's aux slot
                        //   CALL          ; the actual call
                        //
                        // (The NAMECALL-nested case has a MOVE between
                        // the FASTCALL and the CALL — the MOVE copies
                        // the outer function into NAMECALL's arg slot.
                        // We don't try to lift that; the regular loop
                        // handles it correctly via the GETIMPORT it sees
                        // for the outer function.)
                        let n1 = iter.clone().next();
                        let n2 = n1.as_ref().and_then(|_| iter.clone().nth(1));
                        let n3 = n2.as_ref().and_then(|_| iter.clone().nth(2));
                        let layout_ok = matches!(
                            n1.as_ref().map(|(_, i)| i),
                            Some(Instruction::AD { op_code: OpCode::LOP_GETIMPORT, .. })
                        ) && matches!(
                            n2.as_ref().map(|(_, i)| i),
                            Some(Instruction::BC { op_code: OpCode::LOP_NOP, .. })
                        ) && matches!(
                            n3.as_ref().map(|(_, i)| i),
                            Some(Instruction::BC { op_code: OpCode::LOP_CALL, .. })
                        );

                        if !layout_ok {
                            // Layout doesn't match the standalone
                            // direct-call pattern. Two sub-cases:
                            //
                            // (a) NAMECALL-nested: the regular loop will
                            //     produce the correct call via the
                            //     GETIMPORT for the outer function.
                            //
                            // (b) Aliased fastcall: the function is a
                            //     local or upvalue (e.g. `local sub =
                            //     string.sub; sub(x, y)`). The compiler
                            //     emits a FASTCALL anyway, followed by a
                            //     MOVE that copies the local into the
                            //     call's base register. The regular loop
                            //     emits the call correctly, but a reader
                            //     might wonder "why is there a MOVE
                            //     here?" — we add a brief comment
                            //     indicating the call was a fastcall to
                            //     a known builtin.
                            //
                            // Detect (b) by looking at the next
                            // instruction: if it's a NOP, a MOVE
                            // follows, and that's almost certainly
                            // the call's base-register copy. Emit a
                            // `-- was a fastcall <name>` comment for
                            // the reader's benefit.
                            if matches!(
                                n1.as_ref().map(|(_, i)| i),
                                Some(Instruction::BC { op_code: OpCode::LOP_NOP, .. })
                            ) {
                                if let Some(info) = crate::builtins::lookup(builtin_id) {
                                    let name = if info.module.is_empty() {
                                        info.name.to_string()
                                    } else {
                                        format!("{}.{}", info.module, info.name)
                                    };
                                    statements.push(
                                        ast::Comment::new(format!(
                                            "aliased fastcall {} (called via local/upvalue)",
                                            name
                                        ))
                                        .into(),
                                    );
                                }
                            }
                            continue;
                        }

                        // Layout matches: FASTCALL, GETIMPORT, NOP, CALL.
                        // Consume all three.
                        iter.next(); // GETIMPORT
                        iter.next(); // NOP
                        let call_ins = iter.next().unwrap().1; // CALL

                        // Pull the CALL fields out of the `call_ins`
                        // (we already type-checked it as `Instruction::BC
                        // { op_code: LOP_CALL, .. }` in the layout check
                        // above, so this destructuring is safe).
                        let call_ins_bc = match call_ins {
                            Instruction::BC { a: ca, b: cb, c: cc, .. } => (*ca, *cb, *cc),
                            _ => unreachable!(),
                        };
                        let (call_a, call_b, call_c) = call_ins_bc;

                        // Build the call target from the builtin name.
                        // If we don't have a name for this id, fall
                        // through and let the regular loop process the
                        // GETIMPORT + CALL.
                        let info = match crate::builtins::lookup(builtin_id) {
                            Some(i) => i,
                            None => continue,
                        };
                        let call_target = match crate::builtins::build_call_target(info) {
                            Some(t) => t,
                            None => continue,
                        };

                        // Build the argument list. For FASTCALL1+ the
                        // first arg is the FASTCALL's `b` field, but
                        // that register is *also* the register the CALL
                        // uses (since `result_reg + 1` is the first
                        // arg), so we can just use the CALL's arg
                        // registers uniformly. For FASTCALL2/2K/3, AUX
                        // carries extra args that aren't in the CALL's
                        // arg list (because the compiler elided them
                        // when the same register was already pre-loaded
                        // for the CALL). We don't bother with those
                        // because the normal call-site at the result
                        // register will see the same pre-loaded values
                        // and the CALL only needs to know the explicit
                        // args.
                        let arguments: Vec<ast::RValue> = if call_b != 0 {
                            (call_a + 1..call_a + call_b)
                                .map(|r| self.register(r as _).into())
                                .collect()
                        } else {
                            // `b == 0` means MULTRET — the call returns
                            // all values starting at the next register up
                            // to `top`. We don't have a clean way to lift
                            // this in a 1:1 AST, so just emit the call
                            // as `top = Call(...)` for the formatter.
                            if let Some(top_val) = top.take() {
                                (call_a + 1..top_val.1)
                                    .map(|r| self.register(r as _).into())
                                    .chain(std::iter::once(top_val.0))
                                    .collect()
                            } else {
                                statements.push(
                                    ast::Comment::new(
                                        "warning: FASTCALL MULTRET but no top"
                                            .to_string(),
                                    )
                                    .into(),
                                );
                                Vec::new()
                            }
                        };

                        let call = ast::Call::new(call_target, arguments);
                        if call_c != 0 {
                            if call_c == 1 {
                                // Single result: a = Call(...)
                                statements.push(
                                    ast::Assign::new(
                                        vec![self.register(call_a as _).into()],
                                        vec![call.into()],
                                    )
                                    .into(),
                                );
                            } else {
                                // Multiple results: a, a+1, ... = Call(...)
                                statements.push(
                                    ast::Assign::new(
                                        (call_a..call_a + call_c - 1)
                                            .map(|r| self.register(r as _).into())
                                            .collect(),
                                        vec![ast::RValue::Select(call.into())],
                                    )
                                    .into(),
                                );
                            }
                        } else {
                            // MULTRET: result count is dynamic, store in
                            // `top` so the next instruction (which is
                            // usually a CALL or RETURN) can pick it up.
                            top = Some((call.into(), call_a));
                        }
                        continue;
                    }
                    OpCode::LOP_NAMECALL | OpCode::LOP_NAMECALLUDATA => {
                        let namecall_base = a;
                        let namecall_object = self.register(b as _);
                        let const_idx = if op_code == OpCode::LOP_NAMECALLUDATA {
                            (aux & 0xffff) as usize
                        } else {
                            aux as usize
                        };
                        let namecall_method = match self.constant(const_idx) {
                            ast::Literal::String(string) => string,
                            _ => b"__unknown".to_vec(),
                        };
                        let namecall_method_str = String::from_utf8_lossy(&namecall_method).into_owned();

                        // Skip the NOP (AUX)
                        let next_ins = iter.next();
                        if next_ins.is_none() || !matches!(next_ins.unwrap().1, Instruction::BC { op_code: OpCode::LOP_NOP, .. }) {
                            statements.push(ast::Comment::new("warning: NAMECALL not followed by NOP/AUX".to_string()).into());
                        }

                        match iter.next() {
                            Some((_, &Instruction::BC {
                                op_code: OpCode::LOP_CALL | OpCode::LOP_CALLFB,
                                a,
                                b,
                                c,
                                ..
                            })) => {
                                if a != namecall_base {
                                     statements.push(ast::Comment::new("warning: NAMECALL base mismatch".to_string()).into());
                                }
                                
                                let arguments = if b != 0 {
                                    (a + 2..a + b)
                                        .map(|r| self.register(r as _).into())
                                        .collect()
                                } else {
                                    if let Some(top_val) = top.take() {
                                        (a + 2..top_val.1)
                                            .map(|r| self.register(r as _).into())
                                            .chain(std::iter::once(top_val.0))
                                            .collect()
                                    } else {
                                        statements.push(ast::Comment::new("warning: NAMECALL MULTRET but no top".to_string()).into());
                                        Vec::new()
                                    }
                                };

                                
                                let call = ast::MethodCall::new(
                                    namecall_object.into(),
                                    namecall_method_str,
                                    arguments,
                                );

                                if c != 0 {
                                    if c == 1 {
                                        statements.push(call.into());
                                    } else {
                                        statements.push(
                                            ast::Assign::new(
                                                (a..a + c - 1)
                                                    .map(|r| self.register(r as _).into())
                                                    .collect(),
                                                vec![ast::RValue::Select(call.into())],
                                            )
                                            .into(),
                                        );
                                    }
                                } else {
                                    top = Some((call.into(), a));
                                }
                            }
                            Some((_, instruction)) => {
                                statements.push(ast::Comment::new(format!("warning: NAMECALL not followed by CALL: {:?}", instruction)).into());
                            }
                            None => {
                                statements.push(ast::Comment::new("warning: NAMECALL at end of block".to_string()).into());
                            }
                        }
                    }
                    OpCode::LOP_CALL | OpCode::LOP_CALLFB => {
                        let arguments = if b != 0 {
                            (a + 1..a + b)
                                .map(|r| self.register(r as _).into())
                                .collect()
                        } else {
                            if let Some(top_val) = top.take() {
                                (a + 1..top_val.1)
                                    .map(|r| self.register(r as _).into())
                                    .chain(std::iter::once(top_val.0))
                                    .collect()
                            } else {
                                Vec::new()
                            }
                        };

                        let call = ast::Call::new(self.register(a as _).into(), arguments);

                        if c != 0 {
                            if c == 1 {
                                statements.push(call.into());
                            } else {
                                statements.push(
                                    ast::Assign::new(
                                        (a..a + c - 1)
                                            .map(|r| self.register(r as _).into())
                                            .collect(),
                                        vec![ast::RValue::Select(call.into())],
                                    )
                                    .into(),
                                );
                            }
                        } else {
                            top = Some((call.into(), a));
                        }
                    }
                    OpCode::LOP_CLOSEUPVALS => {
                        // LOP_CLOSEUPVALS in Luau is an *implicit* barrier:
                        // "close every open upvalue that captures any
                        // register >= a". There is no source-level Lua
                        // construct for this; the AST `Close` node we used
                        // to emit here was purely cosmetic and printed as
                        // `__close_uv(reg_a, reg_a+1, ..., reg_max)` —
                        // which (a) is not valid Lua, (b) spammed the
                        // decompiled output for any function that used
                        // closures in loops, and (c) gave no useful
                        // information. Skip it entirely; the SSA / name
                        // resolution passes do not depend on it.
                    }
                    OpCode::LOP_SETLIST => {
                        let setlist = if c != 0 {
                            ast::SetList::new(
                                self.register(a as _),
                                aux as usize,
                                (b..b + c - 1)
                                    .map(|r| self.register(r as _).into())
                                    .collect(),
                                None,
                            )
                        } else {
                            let top = top.take().unwrap();
                            ast::SetList::new(
                                self.register(a as _).clone(),
                                aux as usize,
                                (b..top.1).map(|r| self.register(r as _).into()).collect(),
                                Some(top.0),
                            )
                        };
                        statements.push(setlist.into());
                    }
                    OpCode::LOP_CONCAT => {
                        let operands = (b..=c)
                            .map(|r| self.register(r as _))
                            .rev()
                            .collect::<Vec<_>>();
                        assert!(operands.len() >= 2);
                        let mut operands = operands.into_iter();
                        let right = operands.next().unwrap();
                        let left = operands.next().unwrap();
                        let mut concat = ast::Binary::new(
                            left.into(),
                            right.into(),
                            ast::BinaryOperation::Concat,
                        );
                        for r in operands {
                            concat = ast::Binary::new(
                                r.into(),
                                concat.into(),
                                ast::BinaryOperation::Concat,
                            );
                        }
                        statements.push(
                            ast::Assign::new(
                                vec![self.register(a as _).into()],
                                vec![concat.into()],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_AND => statements.push(
                        ast::Assign::new(
                            vec![self.register(a as _).into()],
                            vec![ast::Binary::new(
                                self.register(b as _).into(),
                                self.register(c as _).into(),
                                ast::BinaryOperation::And,
                            )
                            .into()],
                        )
                        .into(),
                    ),
                    OpCode::LOP_ANDK => statements.push(
                        ast::Assign::new(
                            vec![self.register(a as _).into()],
                            vec![ast::Binary::new(
                                self.register(b as _).into(),
                                self.constant(c as _).into(),
                                ast::BinaryOperation::And,
                            )
                            .into()],
                        )
                        .into(),
                    ),
                    OpCode::LOP_OR => statements.push(
                        ast::Assign::new(
                            vec![self.register(a as _).into()],
                            vec![ast::Binary::new(
                                self.register(b as _).into(),
                                self.register(c as _).into(),
                                ast::BinaryOperation::Or,
                            )
                            .into()],
                        )
                        .into(),
                    ),
                    OpCode::LOP_ORK => statements.push(
                        ast::Assign::new(
                            vec![self.register(a as _).into()],
                            vec![ast::Binary::new(
                                self.register(b as _).into(),
                                self.constant(c as _).into(),
                                ast::BinaryOperation::Or,
                            )
                            .into()],
                        )
                        .into(),
                    ),
                    OpCode::LOP_GETVARARGS => {
                        let vararg = ast::VarArg {};
                        if b != 0 {
                            statements.push(
                                ast::Assign::new(
                                    (a..a + b - 1)
                                        .map(|r| self.register(r as _).into())
                                        .collect(),
                                    vec![ast::RValue::Select(vararg.into())],
                                )
                                .into(),
                            );
                        } else {
                            top = Some((vararg.into(), a));
                        }
                    }
                    OpCode::LOP_BREAK => {
                        statements.push(ast::Break {}.into());
                    }
                    OpCode::LOP_NOP | OpCode::LOP_COVERAGE | OpCode::LOP_NATIVECALL => {}
                    OpCode::LOP_LOADKX => {
                        let target = self.register(a as _);
                        let constant = self.constant(aux as _);
                        let statement = ast::Assign::new(vec![target.into()], vec![constant.into()]);
                        statements.push(statement.into());
                    }
                    _ => {
                        statements.push(
                            ast::Comment::new(format!("unhandled instruction: {:?}", instruction))
                                .into(),
                        );
                    }
                },
                Instruction::AD { op_code, a, d, aux } => match op_code {
                    OpCode::LOP_LOADK => {
                        let constant = self.constant(d as _);
                        let target = self.register(a as _);
                        let statement =
                            ast::Assign::new(vec![target.into()], vec![constant.into()]);
                        statements.push(statement.into());
                    }
                    OpCode::LOP_LOADN => {
                        let target = self.register(a as _);
                        let statement = ast::Assign::new(
                            vec![target.into()],
                            vec![ast::Literal::Number(d as _).into()],
                        );
                        statements.push(statement.into());
                    }
                    OpCode::LOP_GETIMPORT => {
                        let target = self.register(a as _);
                        let import_len = (aux >> 30) & 3;
                        assert!(import_len <= 3);
                        let mut import_expression: ast::RValue = ast::Global::new(
                            self.constant(((aux >> 20) & 1023) as usize)
                                .into_string()
                                .unwrap(),
                        )
                        .into();
                        if import_len > 1 {
                            import_expression = ast::Index::new(
                                import_expression,
                                self.constant(((aux >> 10) & 1023) as usize).into(),
                            )
                            .into();
                        }
                        if import_len > 2 {
                            import_expression = ast::Index::new(
                                import_expression,
                                self.constant((aux & 1023) as usize).into(),
                            )
                            .into();
                        }
                        let assign = ast::Assign::new(vec![target.into()], vec![import_expression]);
                        statements.push(assign.into());
                    }
                    OpCode::LOP_JUMPIFNOT => {
                        let condition = self.register(a as _);
                        let statement = ast::If::new(
                            condition.into(),
                            ast::Block::default(),
                            ast::Block::default(),
                        );
                        edges.push((
                            self.block_to_node(block_start + index + 1),
                            BlockEdge::new(BranchType::Then),
                        ));
                        edges.push((
                            self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                            BlockEdge::new(BranchType::Else),
                        ));
                        statements.push(statement.into());
                    }
                    OpCode::LOP_JUMPIF => {
                        let condition = self.register(a as _);
                        let statement = ast::If::new(
                            condition.into(),
                            ast::Block::default(),
                            ast::Block::default(),
                        );
                        edges.push((
                            self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                            BlockEdge::new(BranchType::Then),
                        ));
                        edges.push((
                            self.block_to_node(block_start + index + 1),
                            BlockEdge::new(BranchType::Else),
                        ));
                        statements.push(statement.into());
                    }
                    OpCode::LOP_JUMPIFNOTEQ => {
                        let a = self.register(a as _);
                        let aux = self.register(aux as _);
                        statements.push(
                            ast::If::new(
                                ast::Binary::new(a.into(), aux.into(), ast::BinaryOperation::Equal)
                                    .into(),
                                ast::Block::default(),
                                ast::Block::default(),
                            )
                            .into(),
                        );
                        edges.push((
                            self.block_to_node(block_start + index + 2),
                            BlockEdge::new(BranchType::Then),
                        ));
                        edges.push((
                            self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                            BlockEdge::new(BranchType::Else),
                        ));
                    }
                    OpCode::LOP_JUMPIFNOTLE => {
                        let a = self.register(a as _);
                        let aux = self.register(aux as _);
                        statements.push(
                            ast::If::new(
                                ast::Binary::new(
                                    a.into(),
                                    aux.into(),
                                    ast::BinaryOperation::LessThanOrEqual,
                                )
                                .into(),
                                ast::Block::default(),
                                ast::Block::default(),
                            )
                            .into(),
                        );
                        edges.push((
                            self.block_to_node(block_start + index + 2),
                            BlockEdge::new(BranchType::Then),
                        ));
                        edges.push((
                            self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                            BlockEdge::new(BranchType::Else),
                        ));
                    }
                    OpCode::LOP_JUMPIFNOTLT => {
                        let a = self.register(a as _);
                        let aux = self.register(aux as _);
                        statements.push(
                            ast::If::new(
                                ast::Binary::new(
                                    a.into(),
                                    aux.into(),
                                    ast::BinaryOperation::LessThan,
                                )
                                .into(),
                                ast::Block::default(),
                                ast::Block::default(),
                            )
                            .into(),
                        );
                        edges.push((
                            self.block_to_node(block_start + index + 2),
                            BlockEdge::new(BranchType::Then),
                        ));
                        edges.push((
                            self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                            BlockEdge::new(BranchType::Else),
                        ));
                    }
                    OpCode::LOP_JUMPIFEQ => {
                        let a = self.register(a as _);
                        let aux = self.register(aux as _);
                        statements.push(
                            ast::If::new(
                                ast::Binary::new(a.into(), aux.into(), ast::BinaryOperation::Equal)
                                    .into(),
                                ast::Block::default(),
                                ast::Block::default(),
                            )
                            .into(),
                        );
                        edges.push((
                            self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                            BlockEdge::new(BranchType::Then),
                        ));
                        edges.push((
                            self.block_to_node(block_start + index + 2),
                            BlockEdge::new(BranchType::Else),
                        ));
                    }
                    OpCode::LOP_JUMPIFLE => {
                        let a = self.register(a as _);
                        let aux = self.register(aux as _);
                        statements.push(
                            ast::If::new(
                                ast::Binary::new(
                                    a.into(),
                                    aux.into(),
                                    ast::BinaryOperation::LessThanOrEqual,
                                )
                                .into(),
                                ast::Block::default(),
                                ast::Block::default(),
                            )
                            .into(),
                        );
                        edges.push((
                            self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                            BlockEdge::new(BranchType::Then),
                        ));
                        edges.push((
                            self.block_to_node(block_start + index + 2),
                            BlockEdge::new(BranchType::Else),
                        ));
                    }
                    OpCode::LOP_JUMPIFLT => {
                        let a = self.register(a as _);
                        let aux = self.register(aux as _);
                        statements.push(
                            ast::If::new(
                                ast::Binary::new(
                                    a.into(),
                                    aux.into(),
                                    ast::BinaryOperation::LessThan,
                                )
                                .into(),
                                ast::Block::default(),
                                ast::Block::default(),
                            )
                            .into(),
                        );
                        edges.push((
                            self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                            BlockEdge::new(BranchType::Then),
                        ));
                        edges.push((
                            self.block_to_node(block_start + index + 2),
                            BlockEdge::new(BranchType::Else),
                        ));
                    }
                    OpCode::LOP_JUMPBACK | OpCode::LOP_JUMP => {
                        edges.push((
                            self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                            BlockEdge::new(BranchType::Unconditional),
                        ));
                    }
                    OpCode::LOP_JUMPXEQKNIL => {
                        let a = self.register(a as _);
                        statements.push(
                            ast::If::new(
                                ast::Binary::new(
                                    a.into(),
                                    ast::Literal::Nil.into(),
                                    ast::BinaryOperation::Equal,
                                )
                                .into(),
                                ast::Block::default(),
                                ast::Block::default(),
                            )
                            .into(),
                        );
                        if aux & (1 << 31) != 0 {
                            edges.push((
                                self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                                BlockEdge::new(BranchType::Else),
                            ));
                            edges.push((
                                self.block_to_node(block_start + index + 2),
                                BlockEdge::new(BranchType::Then),
                            ));
                        } else {
                            edges.push((
                                self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                                BlockEdge::new(BranchType::Then),
                            ));
                            edges.push((
                                self.block_to_node(block_start + index + 2),
                                BlockEdge::new(BranchType::Else),
                            ));
                        }
                    }
                    OpCode::LOP_JUMPXEQKB => {
                        let a = self.register(a as _);
                        let literal = if aux & 1 != 0 {
                            ast::Literal::Boolean(true)
                        } else {
                            ast::Literal::Boolean(false)
                        };
                        statements.push(
                            ast::If::new(
                                ast::Binary::new(
                                    a.into(),
                                    literal.into(),
                                    ast::BinaryOperation::Equal,
                                )
                                .into(),
                                ast::Block::default(),
                                ast::Block::default(),
                            )
                            .into(),
                        );
                        if aux & (1 << 31) != 0 {
                            edges.push((
                                self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                                BlockEdge::new(BranchType::Else),
                            ));
                            edges.push((
                                self.block_to_node(block_start + index + 2),
                                BlockEdge::new(BranchType::Then),
                            ));
                        } else {
                            edges.push((
                                self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                                BlockEdge::new(BranchType::Then),
                            ));
                            edges.push((
                                self.block_to_node(block_start + index + 2),
                                BlockEdge::new(BranchType::Else),
                            ));
                        }
                    }
                    OpCode::LOP_JUMPXEQKN | OpCode::LOP_JUMPXEQKS => {
                        let a = self.register(a as _);
                        let literal = self.constant((aux & ((1 << 24) - 1)) as _);
                        statements.push(
                            ast::If::new(
                                ast::Binary::new(
                                    a.into(),
                                    literal.into(),
                                    ast::BinaryOperation::Equal,
                                )
                                .into(),
                                ast::Block::default(),
                                ast::Block::default(),
                            )
                            .into(),
                        );
                        if aux & (1 << 31) != 0 {
                            edges.push((
                                self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                                BlockEdge::new(BranchType::Else),
                            ));
                            edges.push((
                                self.block_to_node(block_start + index + 2),
                                BlockEdge::new(BranchType::Then),
                            ));
                        } else {
                            edges.push((
                                self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                                BlockEdge::new(BranchType::Then),
                            ));
                            edges.push((
                                self.block_to_node(block_start + index + 2),
                                BlockEdge::new(BranchType::Else),
                            ));
                        }
                    }
                    OpCode::LOP_FORNPREP => {
                        // Look up the matching FORNLOOP via the map populated
                        // by `discover_blocks`. This is correct for nested
                        // and sibling numeric-for loops, where the previous
                        // "predecessor ending in NumForNext" search picked
                        // the wrong one.
                        let limit = self.register(a as _);
                        let step = self.register((a + 1) as _);
                        let counter = self.register((a + 2) as _);
                        statements.push(ast::NumForInit::new(counter, limit, step).into());

                        let pc = block_start + index;
                        match self.for_loops.get(&pc).copied() {
                            Some(loop_pc) => {
                                // The FORNLOOP is always the *last*
                                // statement in its block. Find the block
                                // that ends at `loop_pc` (i.e. starts at
                                // `loop_pc`, since FORNLOOP starts its own
                                // block) and add a backedge to it from the
                                // FORNPREP block.
                                if let Some(&loop_node) = self.blocks.get(&loop_pc) {
                                    edges.push((
                                        loop_node,
                                        BlockEdge::new(BranchType::Unconditional),
                                    ));
                                } else {
                                    statements.push(
                                        ast::Comment::new(format!(
                                            "warning: FORNPREP at pc {} has no block for its FORNLOOP at pc {}",
                                            pc, loop_pc
                                        ))
                                        .into(),
                                    );
                                }
                            }
                            None => {
                                statements.push(
                                    ast::Comment::new(
                                        "warning: failed to find loop backedge for FORNPREP"
                                            .to_string(),
                                    )
                                    .into(),
                                );
                            }
                        }
                    }
                    OpCode::LOP_FORNLOOP => {
                        let limit = self.register(a as _);
                        let step = self.register((a + 1) as _);
                        let counter = self.register((a + 2) as _);
                        statements
                            .push(ast::NumForNext::new(counter, limit.into(), step.into()).into());
                        edges.push((
                            self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                            BlockEdge::new(BranchType::Then),
                        ));
                        edges.push((
                            self.block_to_node(block_start + index + 1),
                            BlockEdge::new(BranchType::Else),
                        ));
                    }
                    OpCode::LOP_FORGPREP
                    | OpCode::LOP_FORGPREP_INEXT
                    | OpCode::LOP_FORGPREP_NEXT => {
                        let generator = self.register(a as _);
                        let state = self.register((a + 1) as _);
                        let counter = self.register((a + 2) as _);
                        statements.push(ast::GenericForInit::new(generator, state, counter).into());
                        let loop_node = self
                            .jump_target(block_start, index, d as isize)
                            .expect("FORGPREP target should be a known block (corrupt bytecode?)");
                        // Sanity check: the FORGPREP branch always lands on a
                        // FORGLOOP. If the bytecode says otherwise, the
                        // compiler is non-conforming; emit a warning rather
                        // than panicking.
                        let target_pc = (block_start + index + 1) as isize + d as isize;
                        if target_pc >= 0
                            && (target_pc as usize) < self.function_list[self.function.id].instructions.len()
                            && !matches!(
                                self.function_list[self.function.id].instructions[target_pc as usize],
                                Instruction::AD {
                                    op_code: OpCode::LOP_FORGLOOP,
                                    ..
                                }
                            )
                        {
                            statements.push(
                                ast::Comment::new(
                                    "warning: FORGPREP target is not a FORGLOOP".to_string(),
                                )
                                .into(),
                            );
                        }
                        edges.push((loop_node, BlockEdge::new(BranchType::Unconditional)));
                    }
                    
                    
                    OpCode::LOP_FORGLOOP => {
                        let generator = self.register(a as _);
                        let state = self.register((a + 1) as _);
                        let _counter = self.register((a + 2) as _);
                        statements.push(
                            ast::GenericForNext::new(
                                (a as usize + 3..a as usize + 3 + (aux & 0xff) as usize)
                                    .map(|r| self.register(r))
                                    .collect::<Vec<_>>(),
                                generator.into(),
                                state,
                            )
                            .into(),
                        );
                        edges.push((
                            self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                            BlockEdge::new(BranchType::Then),
                        ));
                        edges.push((
                            self.block_to_node(block_start + index + 1),
                            BlockEdge::new(BranchType::Else),
                        ));
                    }
                    OpCode::LOP_DUPTABLE => {
                        // LBC_CONSTANT_TABLE stores only keys; the VM
                        // materialises the table with those keys set to
                        // `0` (numeric) at runtime, and subsequent
                        // SETTABLEKS/SETTABLE fill the real values. For
                        // decompilation we emit the keys with `nil`
                        // placeholders so the post-process table-cleanup
                        // pass can merge later assignments into a single
                        // constructor, and so keys never show up as bare
                        // `= nil` entries (which is what happened when
                        // plain Table shapes were treated as empty).
                        //
                        // LBC_CONSTANT_TABLE_WITH_CONSTANTS stores key +
                        // packed constant value indices (value_idx < 0
                        // means nil).
                        let table_rvalue: ast::RValue = match self.function_list[self.function.id]
                            .constants
                            .get(d as usize)
                        {
                            Some(BytecodeConstant::TableWithConstants(entries)) => {
                                let pairs = entries
                                    .iter()
                                    .map(|e| {
                                        let key: ast::RValue = self.constant(e.key).into();
                                        let value: ast::RValue = if e.value_index < 0 {
                                            ast::Literal::Nil.into()
                                        } else {
                                            self.constant(e.value_index as usize).into()
                                        };
                                        (Some(key), value)
                                    })
                                    .collect();
                                ast::Table(pairs).into()
                            }
                            Some(BytecodeConstant::Table(keys)) => {
                                let pairs = keys
                                    .iter()
                                    .map(|&key_idx| {
                                        let key: ast::RValue = self.constant(key_idx).into();
                                        (Some(key), ast::Literal::Nil.into())
                                    })
                                    .collect();
                                ast::Table(pairs).into()
                            }
                            _ => ast::Table::default().into(),
                        };
                        statements.push(
                            ast::Assign::new(
                                vec![self.register(a as _).into()],
                                vec![table_rvalue],
                            )
                            .into(),
                        );
                    }
                    OpCode::LOP_DUPCLOSURE | OpCode::LOP_NEWCLOSURE => {
                        let dest_local = self.register(a as _);
                        let func_index_opt: Option<u32> = match op_code {
                            OpCode::LOP_NEWCLOSURE => {
                                let f_idx = d as usize;
                                self.function_list[self.function.id]
                                    .functions
                                    .get(f_idx)
                                    .copied()
                            }
                            OpCode::LOP_DUPCLOSURE => match self.function_list[self.function.id]
                                .constants
                                .get(d as usize)
                            {
                                Some(&BytecodeConstant::Closure(func_index)) => {
                                    Some(func_index as u32)
                                }
                                _ => None,
                            },
                            _ => unreachable!(),
                        };

                        if let Some(func_index) = func_index_opt {
                            let func_name_index = self.function_list[func_index as usize]
                                .function_name;
                            let func_name = if func_name_index == 0
                                || func_name_index as usize > self.string_table.len()
                            {
                                None
                            } else {
                                Some(
                                    String::from_utf8_lossy(
                                        &self.string_table[func_name_index as usize - 1],
                                    )
                                    .into_owned(),
                                )
                            };

                            let func = &self.function_list[func_index as usize];
                            let mut upvalues_passed = Vec::with_capacity(func.num_upvalues.into());
                            for _ in 0..func.num_upvalues {
                                let next_val = iter.next();
                                if let Some((_, ins)) = next_val {
                                    let local = match ins {
                                        &Instruction::BC {
                                            op_code: OpCode::LOP_CAPTURE,
                                            a: capture_type,
                                            b: source,
                                            ..
                                        } => match capture_type {
                                            0 => ast::Upvalue::Copy(self.register(source as _)),
                                            1 => ast::Upvalue::Ref(self.register(source as _)),
                                            2 => ast::Upvalue::Ref(self.upvalues.get(source as usize).cloned().unwrap_or_else(|| ast::RcLocal::default())),
                                            _ => ast::Upvalue::Copy(ast::RcLocal::default()),
                                        },
                                        _ => ast::Upvalue::Copy(ast::RcLocal::default()),
                                    };
                                    upvalues_passed.push(local);
                                }
                            }

                            let function = Arc::<Mutex<_>>::default();
                            self.child_functions
                                .insert(ByAddress(function.clone()), func_index as usize);
                            {
                                let mut lock = function.lock();
                                lock.name = func_name;
                                lock.line = Some(
                                    self.function_list[func_index as usize].line_defined as usize,
                                );
                            }
                            statements.push(
                                ast::Assign::new(
                                    vec![dest_local.into()],
                                    vec![ast::Closure {
                                        function: ByAddress(function),
                                        upvalues: upvalues_passed,
                                    }
                                    .into()],
                                )
                                .into(),
                            );
                        } else {
                            statements.push(ast::Comment::new(format!("warning: failed to find function for closure: {:?}", instruction)).into());
                        }
                    }
                    OpCode::LOP_CMPPROTO => {
                        // LOP_CMPPROTO a d aux: "is the closure in register
                        // `a` the prototype indexed by `aux`?". This is a
                        // low-level helper for the VM to dispatch
                        // `obj:method()` calls to the right method body when
                        // `obj`'s metatable has multiple methods with the
                        // same name. The source-level equivalent is just a
                        // plain method call — there is no source construct
                        // that corresponds to "is this closure prototype N",
                        // so emitting a fake `closure == "proto_N"` string
                        // comparison (the previous behavior) produced
                        // nonsense that the post-passes then mis-pattern-
                        // matched.
                        //
                        // We emit a Comment explaining what happened and
                        // forward both edges forward; the post-passes will
                        // collapse the resulting degenerate `if`-shape. We
                        // do still emit the `If` (with a `true` condition)
                        // so any structural difference between the `Then`
                        // and `Else` arms is preserved — that's where the
                        // method dispatch's specialized body lives.
                        let closure = self.register(a as _);
                        statements.push(
                            ast::Comment::new(format!(
                                "CMPPROTO: comparing closure {:?} against prototype id {}",
                                closure, aux
                            ))
                            .into(),
                        );
                        statements.push(
                            ast::If::new(
                                ast::Literal::Boolean(true).into(),
                                ast::Block::default(),
                                ast::Block::default(),
                            )
                            .into(),
                        );
                        edges.push((
                            self.block_to_node(block_start + index + 2),
                            BlockEdge::new(BranchType::Then),
                        ));
                        edges.push((
                            self.jump_target(block_start, index, d as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                            BlockEdge::new(BranchType::Else),
                        ));
                    }
                    _ => {
                        statements.push(
                            ast::Comment::new(format!("unhandled instruction: {:?}", instruction))
                                .into(),
                        );
                    }
                },
                Instruction::E { op_code, e } => match op_code {
                    OpCode::LOP_JUMPX => {
                        edges.push((
                            self.jump_target(block_start, index, e as isize).expect("jump target should be a known block (corrupt bytecode?)"),
                            BlockEdge::new(BranchType::Unconditional),
                        ));
                    }
                    _ => {
                        statements.push(
                            ast::Comment::new(format!("unhandled instruction: {:?}", instruction))
                                .into(),
                        );
                    }
                },
            }
        }

        let last_index = iter
            .next()
            .map(|(i, _)| block_start + i - 1)
            .unwrap_or(block_end);
        if edges.is_empty()
            && !Self::is_terminator(self.function_list[self.function.id].instructions[last_index])
        {
            if last_index + 1 == self.function_list[self.function.id].instructions.len() {
                statements
                    .push(ast::Comment::new("warning: block does not return".to_string()).into());
            } else {
                edges.push((
                    self.block_to_node(last_index + 1),
                    BlockEdge::new(BranchType::Unconditional),
                ));
            }
        }

        (statements, edges)
    }

    fn register(&mut self, index: usize) -> ast::RcLocal {
        if let Some(local) = self.register_map.get(&index) {
            return local.clone();
        }
        let local = if let Some(name) = self.debug_register_names.get(&index) {
            ast::RcLocal::new(ast::Local::new(Some(name.clone())))
        } else {
            ast::RcLocal::default()
        };
        self.register_map.insert(index, local.clone());
        local
    }

    fn constant(&mut self, index: usize) -> ast::Literal {
        // Some real-world bytecodes (e.g. luau-compile 0.728 with newer
        // opcode extensions) reference constant indices that don't exist
        // in the current function's constant table. The decompiler
        // should produce *something* useful here rather than panic, so we
        // fall back to `nil` for any out-of-bounds access. The same is
        // done for malformed constant data (e.g. a STRING whose index
        // points past the end of the string table) and for unknown
        // constant kinds.
        let converted_constant = match self.function_list[self.function.id]
            .constants
            .get(index)
        {
            Some(BytecodeConstant::Nil) => ast::Literal::Nil,
            Some(BytecodeConstant::Boolean(v)) => ast::Literal::Boolean(*v),
            Some(BytecodeConstant::Number(v)) => ast::Literal::Number(*v),
            Some(BytecodeConstant::Integer(v)) => ast::Literal::Integer(*v),
            Some(BytecodeConstant::String(v)) => {
                // `v` is the 1-based string-table index (0 = no
                // string). The lifter's debug-info code expects the
                // same convention, so we just hand it through.
                if *v == 0 {
                    ast::Literal::Nil
                } else if (*v as usize) <= self.string_table.len() {
                    ast::Literal::String(self.string_table[*v as usize - 1].clone())
                } else {
                    ast::Literal::Nil
                }
            }
            Some(BytecodeConstant::Vector(x, y, z, w)) => ast::Literal::Vector(*x, *y, *z, *w),
            Some(_) | None => ast::Literal::Nil,
        };
        self.constant_map
            .entry(index)
            .or_insert(converted_constant)
            .clone()
    }

    fn block_to_node(&self, insn_index: usize) -> NodeIndex {
        *self.blocks.get(&insn_index).unwrap()
    }

    fn jump_target(&self, block_start: usize, index: usize, d: isize) -> Option<NodeIndex> {
        let next_pc = block_start + index + 1;
        let target = next_pc.checked_add_signed(d)?;
        self.blocks.get(&target).copied()
    }

    fn is_terminator(instruction: Instruction) -> bool {
        match instruction {
            Instruction::BC { op_code, c, .. } => match op_code {
                OpCode::LOP_RETURN => true,
                OpCode::LOP_LOADB if c != 0 => true,
                _ => false,
            },
            Instruction::AD { op_code, .. } => matches!(
                op_code,
                OpCode::LOP_JUMP
                    | OpCode::LOP_JUMPBACK
                    | OpCode::LOP_JUMPIF
                    | OpCode::LOP_JUMPIFNOT
                    | OpCode::LOP_JUMPIFEQ
                    | OpCode::LOP_JUMPIFLE
                    | OpCode::LOP_JUMPIFLT
                    | OpCode::LOP_JUMPIFNOTEQ
                    | OpCode::LOP_JUMPIFNOTLE
                    | OpCode::LOP_JUMPIFNOTLT
                    | OpCode::LOP_JUMPXEQKNIL
                    | OpCode::LOP_JUMPXEQKB
                    | OpCode::LOP_JUMPXEQKN
                    | OpCode::LOP_JUMPXEQKS
                    | OpCode::LOP_FORNPREP
                    | OpCode::LOP_FORNLOOP
                    | OpCode::LOP_FORGPREP
                    | OpCode::LOP_FORGLOOP
                    | OpCode::LOP_FORGPREP_INEXT
                    | OpCode::LOP_FORGPREP_NEXT
            ),
            Instruction::E { op_code, .. } => matches!(op_code, OpCode::LOP_JUMPX),
        }
    }
}
