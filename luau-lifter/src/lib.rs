mod deserializer;
mod instruction;
mod lifter;
mod op_code;

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
use rustc_hash::FxHashMap;
use triomphe::Arc;
use walkdir::WalkDir;

use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    time::Instant,
};

use deserializer::bytecode::Bytecode;

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

pub fn dump_cfgs(bytecode: &[u8], encode_key: u8) -> Vec<cfg::CfgSnapshot> {
    let encode_key = detect_encode_key(bytecode, encode_key);
    let chunk = match deserializer::deserialize(bytecode, encode_key) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let chunk = match chunk {
        Bytecode::Chunk(c) => c,
        Bytecode::Error(_) => return Vec::new(),
    };

    let mut out = Vec::new();
    let mut visited = rustc_hash::FxHashSet::default();
    let mut stack = vec![chunk.main];
    while let Some(func_id) = stack.pop() {
        if !visited.insert(func_id) {
            continue;
        }
        let (function, _upvalues, child_functions) =
            Lifter::lift(&chunk.functions, &chunk.string_table, func_id);
        let name = if func_id == chunk.main {
            "main".to_string()
        } else {
            format!("function #{func_id}")
        };
        out.push(cfg::CfgSnapshot::from_function(&function, name));
        stack.extend(child_functions.into_iter().map(|(_, f)| f));
    }
    out
}

pub fn decompile_bytecode(bytecode: &[u8], encode_key: u8) -> String {
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
                    Lifter::lift(&chunk.functions, &chunk.string_table, func_id);
                lifted.push((ast_func, function, upvalues));
                stack.extend(child_functions.into_iter().map(|(a, f)| (a.0, f)));
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
            name_locals(&mut body, true);
            body.to_string()
        }
    }
}

fn propagate_names(body: &mut ast::Block) {
    propagate_names_block(body);
}

fn propagate_names_block(block: &mut ast::Block) {
    for _ in 0..2 {
        for stat in block.0.iter() {
            if let ast::Statement::Assign(assign) = stat {
                if assign.left.len() == 1 && assign.right.len() == 1 {
                    if let Some(lhs) = assign.left[0].as_local() {
                        let lhs_name = lhs.0 .0.lock().0.clone();
                        if let Some(lhs_name) = lhs_name {
                            if let ast::RValue::Local(rhs) = &assign.right[0] {
                                let mut rhs_lock = rhs.0 .0.lock();
                                if rhs_lock.0.is_none() {
                                    rhs_lock.0 = Some(lhs_name);
                                }
                            }
                        } else {
                            if let ast::RValue::Local(rhs) = &assign.right[0] {
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
                propagate_names_block(&mut closure.function.lock().body);
            }
        });
        match stat {
            ast::Statement::If(r#if) => {
                propagate_names_block(&mut r#if.then_block.lock());
                propagate_names_block(&mut r#if.else_block.lock());
            }
            ast::Statement::While(r#while) => {
                propagate_names_block(&mut r#while.block.lock());
            }
            ast::Statement::Repeat(repeat) => {
                propagate_names_block(&mut repeat.block.lock());
            }
            ast::Statement::NumericFor(numeric_for) => {
                propagate_names_block(&mut numeric_for.block.lock());
            }
            ast::Statement::GenericFor(generic_for) => {
                propagate_names_block(&mut generic_for.block.lock());
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
