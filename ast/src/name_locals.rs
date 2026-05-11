use rustc_hash::FxHashSet;
use triomphe::Arc;

use crate::{Block, Global, Literal, RValue, RcLocal, Statement, Traverse, Upvalue};

struct Namer {
    rename: bool,
    counter: usize,
    upvalues: FxHashSet<RcLocal>,
    numeric_for_depth: usize,
}

const FOR_LETTERS: &[&str] = &["i", "j", "k", "l", "m", "n"];

impl Namer {
    fn is_synthetic_name(name: &str) -> bool {
        if name.is_empty() || name == "_" {
            return true;
        }
        if name.len() == 1 && (name == "v" || name == "p") {
            return true;
        }
        if (name.starts_with('v') || name.starts_with('p')) && name.len() > 1 {
            return name[1..].chars().all(|c| c.is_ascii_digit());
        }
        false
    }

    fn name_local_with_prefix(&mut self, prefix: &str, local: &RcLocal) {
        let mut lock = local.0 .0.lock();
        if lock.0.is_some() && !self.rename {
            return;
        }
        if lock.0.is_some() && self.rename {
            if let Some(ref name) = lock.0 {
                if !Self::is_synthetic_name(name) {
                    return;
                }
            }
        }
        if Arc::count(&local.0 .0) == 1 {
            lock.0 = Some("_".to_string());
            return;
        }
        let suffix = self.counter;
        self.counter += 1;
        let upv = if self.upvalues.contains(local) { "_u" } else { "" };
        lock.0 = Some(format!("{prefix}{upv}{suffix}"));
    }

    fn name_local_fixed(&mut self, fixed: &str, local: &RcLocal) {
        let mut lock = local.0 .0.lock();
        if lock.0.is_some() && !self.rename {
            return;
        }
        if lock.0.is_some() && self.rename {
            if let Some(ref name) = lock.0 {
                if !Self::is_synthetic_name(name) {
                    return;
                }
            }
        }
        if Arc::count(&local.0 .0) == 1 {
            lock.0 = Some("_".to_string());
            return;
        }
        lock.0 = Some(fixed.to_string());
    }

    fn for_letter(&self) -> &'static str {
        FOR_LETTERS[self.numeric_for_depth.min(FOR_LETTERS.len() - 1)]
    }

    fn gen_for_convention(right: &[RValue]) -> Option<(&'static str, &'static str)> {
        let first = right.first()?;
        let global_name = |g: &Global| std::str::from_utf8(&g.0).ok().map(|s| s.to_string());
        let name = match first {
            RValue::Call(call) => match &*call.value {
                RValue::Global(g) => global_name(g),
                _ => None,
            },
            RValue::Global(g) => global_name(g),
            _ => None,
        }?;
        match name.as_str() {
            "pairs" => Some(("k", "v")),
            "ipairs" => Some(("i", "v")),
            "next" => Some(("k", "v")),
            _ => None,
        }
    }

    fn hint_for_rvalue(rv: &RValue) -> &'static str {
        match rv {
            RValue::Literal(Literal::String(_)) => "s",
            RValue::Literal(Literal::Number(_)) => "n",
            RValue::Literal(Literal::Boolean(_)) => "b",
            RValue::Table(_) => "t",
            RValue::Closure(_) => "fn",
            RValue::Call(call) => {
                if let RValue::Global(g) = &*call.value {
                    if std::str::from_utf8(&g.0).ok() == Some("require") {
                        return "mod";
                    }
                }
                "v"
            }
            _ => "v",
        }
    }

    fn name_locals(&mut self, block: &mut Block) {
        for statement in &mut block.0 {
            statement.post_traverse_values(&mut |value| -> Option<()> {
                if let itertools::Either::Right(RValue::Closure(closure)) = value {
                    let mut function = closure.function.lock();
                    for param in &function.parameters {
                        self.name_local_with_prefix("p", param);
                    }
                    self.name_locals(&mut function.body);
                };
                None
            });
            match statement {
                Statement::Assign(assign) if assign.prefix => {
                    for (i, lvalue) in assign.left.iter().enumerate() {
                        let hint = assign
                            .right
                            .get(i)
                            .map(Self::hint_for_rvalue)
                            .unwrap_or("v");
                        self.name_local_with_prefix(hint, lvalue.as_local().unwrap());
                    }
                }
                Statement::If(r#if) => {
                    self.name_locals(&mut r#if.then_block.lock());
                    self.name_locals(&mut r#if.else_block.lock());
                }
                Statement::While(r#while) => {
                    self.name_locals(&mut r#while.block.lock());
                }
                Statement::Repeat(repeat) => {
                    self.name_locals(&mut repeat.block.lock());
                }
                Statement::NumericFor(numeric_for) => {
                    let letter = self.for_letter();
                    self.name_local_fixed(letter, &numeric_for.counter);
                    self.numeric_for_depth += 1;
                    self.name_locals(&mut numeric_for.block.lock());
                    self.numeric_for_depth -= 1;
                }
                Statement::GenericFor(generic_for) => {
                    let convention = Self::gen_for_convention(&generic_for.right);
                    if let Some((k_name, v_name)) = convention {
                        if generic_for.res_locals.len() == 1 {
                            self.name_local_fixed(v_name, &generic_for.res_locals[0]);
                        } else {
                            self.name_local_fixed(k_name, &generic_for.res_locals[0]);
                            self.name_local_fixed(v_name, &generic_for.res_locals[1]);
                            for res_local in &generic_for.res_locals[2..] {
                                self.name_local_with_prefix("v", res_local);
                            }
                        }
                    } else {
                        for res_local in &generic_for.res_locals {
                            self.name_local_with_prefix("v", res_local);
                        }
                    }
                    self.name_locals(&mut generic_for.block.lock());
                }
                _ => {}
            }
        }
    }

    fn find_upvalues(&mut self, block: &mut Block) {
        for statement in &mut block.0 {
            statement.post_traverse_values(&mut |value| -> Option<()> {
                if let itertools::Either::Right(RValue::Closure(closure)) = value {
                    self.upvalues.extend(
                        closure
                            .upvalues
                            .iter()
                            .map(|u| match u {
                                Upvalue::Copy(l) | Upvalue::Ref(l) => l,
                            })
                            .cloned(),
                    );
                    self.find_upvalues(&mut closure.function.lock().body);
                };
                None
            });
            match statement {
                Statement::If(r#if) => {
                    self.find_upvalues(&mut r#if.then_block.lock());
                    self.find_upvalues(&mut r#if.else_block.lock());
                }
                Statement::While(r#while) => {
                    self.find_upvalues(&mut r#while.block.lock());
                }
                Statement::Repeat(repeat) => {
                    self.find_upvalues(&mut repeat.block.lock());
                }
                Statement::NumericFor(numeric_for) => {
                    self.find_upvalues(&mut numeric_for.block.lock());
                }
                Statement::GenericFor(generic_for) => {
                    self.find_upvalues(&mut generic_for.block.lock());
                }
                _ => {}
            }
        }
    }
}

pub fn name_locals(block: &mut Block, rename: bool) {
    let mut namer = Namer {
        rename,
        counter: 1,
        upvalues: FxHashSet::default(),
        numeric_for_depth: 0,
    };
    namer.find_upvalues(block);
    namer.name_locals(block);
}
