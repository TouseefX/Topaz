use ast::{
    inline_gotos::inline_short_gotos, local_declarations::LocalDeclarer,
    name_locals::name_locals, replace_locals::replace_locals, Traverse,
};
use by_address::ByAddress;
use cfg::ssa::{
    self,
    structuring::{structure_conditionals, structure_jumps, structure_method_calls},
};
use indexmap::IndexMap;
use lifter::Lifter;
use parking_lot::Mutex;
use petgraph::algo::dominators::simple_fast;
use rustc_hash::{FxHashMap, FxHashSet};
use triomphe::Arc;

use lua51_deserializer::chunk::Chunk;

mod lifter;

pub fn dump_cfgs(bytecode: &[u8]) -> Vec<cfg::CfgSnapshot> {
    let chunk = match Chunk::parse(bytecode) {
        Ok((_, c)) => c,
        Err(_) => return Vec::new(),
    };
    let mut lifted = Vec::new();
    let (top_function, _) = Lifter::lift(&chunk.function, &mut lifted);
    let mut out = vec![cfg::CfgSnapshot::from_function(&top_function, "main")];
    for (i, (_, function, _)) in lifted.into_iter().enumerate() {
        let name = format!("closure #{i}");
        out.push(cfg::CfgSnapshot::from_function(&function, name));
    }
    out
}

pub fn decompile_bytecode(bytecode: &[u8]) -> String {
    let chunk = Chunk::parse(bytecode).unwrap().1;
    let mut lifted = Vec::new();
    let (function, upvalues) = Lifter::lift(&chunk.function, &mut lifted);
    lifted.push((Arc::<Mutex<_>>::default(), function, upvalues));
    lifted.reverse();

    let (main, ..) = lifted.first().unwrap().clone();
    let mut upvalues = lifted
        .into_iter()
        .map(|(ast_function, mut function, upvalues_in)| {
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
                    || structure_method_calls(&mut function)
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
                ast_function.parameters = params;
                ast_function.is_variadic = is_variadic;
                ast_function.line = func_line;
            }
            (ByAddress(ast_function), upvalues_in)
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
    name_locals(&mut body, false);

    body.to_string()
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

fn link_upvalues(
    body: &mut ast::Block,
    upvalues: &mut FxHashMap<ByAddress<Arc<Mutex<ast::Function>>>, Vec<ast::RcLocal>>,
) {
    for stat in &mut body.0 {
        stat.traverse_rvalues(&mut |rvalue| {
            if let ast::RValue::Closure(closure) = rvalue {
                let old_upvalues = upvalues.remove(&closure.function).unwrap();
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
