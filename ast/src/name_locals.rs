use rustc_hash::FxHashSet;
use triomphe::Arc;

use crate::{Block, Literal, RValue, RcLocal, Statement, Traverse, Upvalue};

struct Namer {
    rename: bool,
    counter: usize,
    upvalues: FxHashSet<RcLocal>,
}

impl Namer {
    fn extract_name_hint(rvalue: &RValue) -> Option<String> {
        match rvalue {
            RValue::Closure(closure) => {
                let function = closure.function.lock();
                if let Some(name) = &function.name {
                    if !name.is_empty() {
                        return Some(Self::sanitize_name(name));
                    }
                }
                None
            }
            RValue::MethodCall(method_call) => {
                if let Some(RValue::Literal(Literal::String(arg))) = method_call.arguments.first() {
                    if let Ok(s) = std::str::from_utf8(arg) {
                        return Some(Self::sanitize_name(s));
                    }
                }
                None
            }
            RValue::Index(index) => match &*index.right {
                RValue::Literal(Literal::String(string)) => {
                    if let Ok(s) = std::str::from_utf8(string) {
                        return Some(Self::sanitize_name(s));
                    }
                    None
                }
                _ => None,
            },
            RValue::Global(global) => Some(Self::sanitize_name(&global.to_string())),
            _ => None,
        }
    }

    fn sanitize_name(name: &str) -> String {
        let sanitized: String = name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect();

        if sanitized.chars().next().map_or(false, |c| c.is_numeric()) {
            format!("_{}", sanitized)
        } else if sanitized.is_empty() {
            "var".to_string()
        } else {
            sanitized
        }
    }

    fn get_prefix_from_rvalue(rvalue: &RValue) -> &'static str {
        match rvalue {
            RValue::Table(_) => "t",           
            RValue::Closure(_) => "f",        
            _ => "v",                          
        }
    }

    fn name_local(&mut self, prefix: &str, local: &RcLocal, hint: Option<&RValue>) {
        let mut lock = local.0 .0.lock();
        if self.rename || lock.0.is_none() {
            if Arc::strong_count(&local.0 .0) == 1 {
                lock.0 = Some("_".to_string());
            } else {
                let type_prefix = if let Some(rvalue) = hint {
                    Self::get_prefix_from_rvalue(rvalue)
                } else {
                    prefix
                };

                let name = if let Some(rvalue) = hint {
                    if let Some(hint_name) = Self::extract_name_hint(rvalue) {
                        format!("{}_{}_{}", type_prefix, hint_name, self.counter)
                    } else {
                        format!("{}{}", type_prefix, self.counter)
                    }
                } else {
                    format!("{}{}", prefix, self.counter)
                };

                lock.0 = Some(name);
                self.counter += 1;
            }
        }
    }

    fn name_locals(&mut self, block: &mut Block) {
        for statement in &mut block.0 {
            statement.post_traverse_values(&mut |value| -> Option<()> {
                if let itertools::Either::Right(RValue::Closure(closure)) = value {
                    let mut function = closure.function.lock();
                    for param in &function.parameters {
                        self.name_local("a", param, None);  
                    }
                    self.name_locals(&mut function.body);
                };
                None
            });
            match statement {
                Statement::Assign(assign) if assign.prefix => {
                    for (i, lvalue) in assign.left.iter().enumerate() {
                        if let Some(local) = lvalue.as_local() {
                            let hint = assign.right.get(i);
                            self.name_local("l", local, hint);
                        }
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
                    self.name_local("i", &numeric_for.counter, None);  
                    self.name_locals(&mut numeric_for.block.lock());
                }
                Statement::GenericFor(generic_for) => {
                    for res_local in &generic_for.res_locals {
                        self.name_local("v", res_local, None);  
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
    };
    namer.find_upvalues(block);
    namer.name_locals(block);
}