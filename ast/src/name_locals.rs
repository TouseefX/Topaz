use rustc_hash::{FxHashMap, FxHashSet};
use triomphe::Arc;

use crate::{
    type_inference_naming, Block, Call, Global, Literal, MethodCall, RValue, RcLocal, Select,
    Statement, Traverse, Upvalue,
};

struct Namer {
    rename: bool,
    counter: usize,
    upvalues: FxHashSet<RcLocal>,
    numeric_for_depth: usize,
    name_uses: FxHashMap<String, usize>,
    /// Usage-based type-inference hints (see `type_inference_naming`),
    /// consulted only as a fallback when a local has no name derivable
    /// from its own creating expression.
    type_hints: FxHashMap<RcLocal, &'static str>,
}

const FOR_LETTERS: &[&str] = &["i", "j", "k", "l", "m", "n"];
const SYNTHETIC_PREFIXES: &[&str] = &["v", "p", "t", "s", "n", "b", "k", "fn", "mod", "c"];

const LUA_KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
    // Luau type keywords (cannot be used as bare identifiers in type annotations)
    "cframe", "thread", "any", "never", "void",
];

const LUAU_BUILTINS: &[&str] = &[
    // Standard Lua / Luau global functions
    "wait", "warn", "print", "error", "assert", "type", "typeof", "pcall", "xpcall", "select",
    "require", "next", "pairs", "ipairs", "tostring", "tonumber", "unpack", "rawequal", "rawget",
    "rawset", "rawlen", "setmetatable", "getmetatable", "newproxy", "gcinfo", "collectgarbage",
    // Roblox / Luau standard libraries & globals
    "game", "workspace", "script", "math", "table", "string", "task", "coroutine", "bit32",
    "debug", "utf8", "os", "plugin", "shared", "_G",
    // Roblox standard data types & constructors
    "CFrame", "cframe", "Vector3", "Vector2", "UDim2", "UDim", "Color3", "ColorSequence",
    "NumberRange", "NumberSequence", "Ray", "TweenInfo", "Instance", "Enum", "BrickColor",
    "Rect", "Region3", "Region3int16", "Vector2int16", "Vector3int16", "Font", "PathWaypoint",
];

pub fn is_synthetic_name(name: &str) -> bool {
    if name.is_empty() || name == "_" {
        return true;
    }
    if name.len() == 1 && (name == "v" || name == "p") {
        return true;
    }
    for &prefix in SYNTHETIC_PREFIXES {
        if let Some(rest) = name.strip_prefix(prefix) {
            let rest = rest.strip_prefix("_u").unwrap_or(rest);
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                return true;
            }
        }
    }
    false
}

impl Namer {
    fn is_valid_identifier(name: &str) -> bool {
        if name.is_empty() {
            return false;
        }
        let mut chars = name.chars();
        let first = chars.next().unwrap();
        if !(first.is_ascii_alphabetic() || first == '_') {
            return false;
        }
        if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return false;
        }
        !LUA_KEYWORDS.contains(&name) && !LUAU_BUILTINS.contains(&name)
    }

    fn lower_first(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            Some(c) => c.to_ascii_lowercase().to_string() + chars.as_str(),
            None => String::new(),
        }
    }

    fn string_lit(rv: &RValue) -> Option<&str> {
        if let RValue::Literal(Literal::String(s)) = rv {
            std::str::from_utf8(s).ok()
        } else {
            None
        }
    }

    fn global_name(g: &Global) -> Option<&str> {
        std::str::from_utf8(&g.0).ok()
    }

    fn method_call_last(mc: &MethodCall) -> Option<String> {
        match mc.method.as_str() {
            "WaitForChild" | "FindFirstChild" | "FindFirstChildOfClass"
            | "FindFirstChildWhichIsA" | "FindFirstAncestor" | "GetService" => {
                mc.arguments.first().and_then(Self::string_lit).map(str::to_string)
            }
            _ => Some(mc.method.clone()),
        }
    }

    fn last_field(rv: &RValue) -> Option<String> {
        match rv {
            RValue::Index(idx) => Self::string_lit(&idx.right).map(str::to_string),
            RValue::MethodCall(mc) => Self::method_call_last(mc),
            RValue::Call(call) => Self::last_field(&call.value),
            RValue::Select(Select::Call(call)) => Self::last_field(&call.value),
            RValue::Select(Select::MethodCall(mc)) => Self::method_call_last(mc),
            RValue::Global(g) => Self::global_name(g).map(str::to_string),
            RValue::Local(_) => None,
            _ => None,
        }
    }

    fn first_string_arg(mc: &MethodCall) -> Option<&str> {
        mc.arguments.first().and_then(Self::string_lit)
    }

    fn name_from_method_call(mc: &MethodCall) -> Option<String> {
        match mc.method.as_str() {
            "GetService"
            | "WaitForChild"
            | "FindFirstChild"
            | "FindFirstChildOfClass"
            | "FindFirstChildWhichIsA"
            | "FindFirstAncestor"
            | "FindFirstAncestorOfClass"
            | "FindFirstAncestorWhichIsA" => {
                Self::first_string_arg(mc).map(Self::lower_first)
            }
            "GetChildren" => Some("children".to_string()),
            "GetDescendants" => Some("descendants".to_string()),
            "GetPlayers" => Some("players".to_string()),
            "GetMouse" => Some("mouse".to_string()),
            "GetPropertyChangedSignal" => Self::first_string_arg(mc)
                .map(|s| format!("{}Changed", Self::lower_first(s))),
            "Connect" | "ConnectParallel" | "Once" => Some("connection".to_string()),
            "Clone" => Self::last_field(&mc.value).map(|s| Self::lower_first(&s)),
            _ => None,
        }
    }

    fn name_from_call(call: &Call) -> Option<String> {
        match &*call.value {
            RValue::Global(g) => match Self::global_name(g)? {
                "require" => {
                    let arg = call.arguments.first()?;
                    Self::last_field(arg).map(|s| Self::lower_first(&s))
                }
                "tostring" => Some("str".to_string()),
                "tonumber" => Some("num".to_string()),
                "type" | "typeof" => Some("ty".to_string()),
                "newproxy" => Some("proxy".to_string()),
                "setmetatable" => call
                    .arguments
                    .first()
                    .and_then(Self::last_field)
                    .map(|s| Self::lower_first(&s)),
                _ => None,
            },
            RValue::Index(idx) => {
                let method = Self::string_lit(&idx.right)?;
                if method == "new" {
                    Self::last_field(&idx.left).map(|s| Self::lower_first(&s))
                } else if method == "fromName" || method == "named" {
                    call.arguments.first().and_then(Self::string_lit).map(Self::lower_first)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn derive_name(rv: &RValue) -> Option<String> {
        let raw = match rv {
            RValue::Call(call) => Self::name_from_call(call),
            RValue::MethodCall(mc) => Self::name_from_method_call(mc),
            RValue::Select(Select::Call(call)) => Self::name_from_call(call),
            RValue::Select(Select::MethodCall(mc)) => Self::name_from_method_call(mc),
            RValue::Index(idx) => Self::string_lit(&idx.right).map(Self::lower_first),
            RValue::Global(g) => Self::global_name(g).map(Self::lower_first),
            _ => None,
        }?;
        if Self::is_valid_identifier(&raw) {
            Some(raw)
        } else {
            None
        }
    }

    fn unique_name(&mut self, base: &str) -> String {
        let candidate = base.to_string();
        let count = self.name_uses.entry(candidate.clone()).or_insert(0);
        if *count == 0 {
            *count = 1;
            candidate
        } else {
            let n = *count;
            *count += 1;
            format!("{}{}", candidate, n + 1)
        }
    }

    fn name_local_smart(&mut self, hint: &str, rvalue: Option<&RValue>, local: &RcLocal) {
        let mut lock = local.0 .0.lock();
        if !self.rename {
            // lua51 path: keep whatever name is already there, but still
            // record it so later unique_name calls cannot collide.
            if let Some(ref existing) = lock.0 {
                self.name_uses.entry(existing.clone()).or_insert(1);
                return;
            }
        }
        if Arc::count(&local.0 .0) == 1 {
            lock.0 = Some("_".to_string());
            return;
        }
        // SSA copies the originating register's debug name onto every split
        // (`new_local_from`). Keeping that name without uniquifying made
        // distinct live ranges print as the same identifier (`insert = Bars`
        // then `insert = Debris`, `new = Color3.fromRGB` then `new = 255`)
        // and silently clobber each other in the emitted Luau.
        let preferred = lock.0.as_ref().and_then(|existing| {
            if !is_synthetic_name(existing) && Self::is_valid_identifier(existing) {
                Some(existing.clone())
            } else {
                None
            }
        });
        drop(lock);

        let name = if let Some(pref) = preferred {
            self.unique_name(&pref)
        } else if let Some(derived) = rvalue.and_then(Self::derive_name) {
            self.unique_name(&derived)
        } else if let Some(&hint) = self.type_hints.get(local) {
            self.unique_name(hint)
        } else {
            let suffix = self.counter;
            self.counter += 1;
            let upv = if self.upvalues.contains(local) { "_u" } else { "" };
            self.unique_name(&format!("{hint}{upv}{suffix}"))
        };
        local.0 .0.lock().0 = Some(name);
    }

    fn name_local_with_prefix(&mut self, prefix: &str, local: &RcLocal) {
        self.name_local_smart(prefix, None, local);
    }

    fn name_local_fixed(&mut self, fixed: &str, local: &RcLocal) {
        let mut lock = local.0 .0.lock();
        if lock.0.is_some() && !self.rename {
            return;
        }
        if Arc::count(&local.0 .0) == 1 {
            lock.0 = Some("_".to_string());
            return;
        }
        drop(lock);
        let name = self.unique_name(fixed);
        local.0 .0.lock().0 = Some(name);
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
            RValue::Literal(Literal::Number(_)) | RValue::Literal(Literal::Integer(_)) => "n",
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
        if crate::past_post_deadline() {
            return;
        }
        for statement in &mut block.0 {
            statement.post_traverse_values(&mut |value| -> Option<()> {
                if let itertools::Either::Right(RValue::Closure(closure)) = value {
                    let ptr = Arc::as_ptr(&closure.function.0) as *const ();
                    if !crate::walk_seen_insert(ptr) {
                        return None;
                    }
                    let Some(mut function) = closure.function.try_lock() else {
                        return None;
                    };
                    for param in &function.parameters {
                        self.name_local_with_prefix("p", param);
                    }
                    self.name_locals(&mut function.body);
                };
                None
            });
            match statement {
                Statement::Assign(assign) if assign.prefix => {
                    // Medal-improved: prefer the compiler debug name on a
                    // `local f = function ...` so CameraShaker.Start-style
                    // helpers keep their original identifier.
                    let named_from_debug = assign.left.len() == 1
                        && assign.right.len() == 1
                        && assign.left[0].as_local().is_some()
                        && assign.right[0].as_closure().is_some();
                    if named_from_debug {
                        let local = assign.left[0].as_local().unwrap();
                        let closure = assign.right[0].as_closure().unwrap();
                        if let Some(name) = closure.function.try_lock().and_then(|f| f.name.clone())
                        {
                            if Self::is_valid_identifier(&name) {
                                let mut lock = local.0 .0.lock();
                                if lock.0.is_none() || self.rename {
                                    lock.0 = Some(name);
                                }
                            }
                        }
                    }
                    for (i, lvalue) in assign.left.iter().enumerate() {
                        if named_from_debug && i == 0 {
                            if lvalue.as_local().unwrap().0 .0.try_lock().is_some_and(|g| g.0.is_some()) {
                                continue;
                            }
                        }
                        let rv = assign.right.get(i);
                        let hint = rv.map(Self::hint_for_rvalue).unwrap_or("v");
                        self.name_local_smart(hint, rv, lvalue.as_local().unwrap());
                    }
                }
                Statement::If(r#if) => {
                    crate::visit_shared_mut(&r#if.then_block, &mut |b| self.name_locals(b));
                    crate::visit_shared_mut(&r#if.else_block, &mut |b| self.name_locals(b));
                }
                Statement::While(r#while) => {
                    crate::visit_shared_mut(&r#while.block, &mut |b| self.name_locals(b));
                }
                Statement::Repeat(repeat) => {
                    crate::visit_shared_mut(&repeat.block, &mut |b| self.name_locals(b));
                }
                Statement::NumericFor(numeric_for) => {
                    let letter = self.for_letter();
                    self.name_local_fixed(letter, &numeric_for.counter);
                    self.numeric_for_depth += 1;
                    crate::visit_shared_mut(&numeric_for.block, &mut |b| self.name_locals(b));
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
                    crate::visit_shared_mut(&generic_for.block, &mut |b| self.name_locals(b));
                }
                _ => {}
            }
        }
    }

    fn find_upvalues(&mut self, block: &mut Block) {
        if crate::past_post_deadline() {
            return;
        }
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
                    let ptr = Arc::as_ptr(&closure.function.0) as *const ();
                    if crate::walk_seen_insert(ptr) {
                        if let Some(mut function) = closure.function.try_lock() {
                            self.find_upvalues(&mut function.body);
                        }
                    }
                };
                None
            });
            match statement {
                Statement::If(r#if) => {
                    crate::visit_shared_mut(&r#if.then_block, &mut |b| self.find_upvalues(b));
                    crate::visit_shared_mut(&r#if.else_block, &mut |b| self.find_upvalues(b));
                }
                Statement::While(r#while) => {
                    crate::visit_shared_mut(&r#while.block, &mut |b| self.find_upvalues(b));
                }
                Statement::Repeat(repeat) => {
                    crate::visit_shared_mut(&repeat.block, &mut |b| self.find_upvalues(b));
                }
                Statement::NumericFor(numeric_for) => {
                    crate::visit_shared_mut(&numeric_for.block, &mut |b| self.find_upvalues(b));
                }
                Statement::GenericFor(generic_for) => {
                    crate::visit_shared_mut(&generic_for.block, &mut |b| self.find_upvalues(b));
                }
                _ => {}
            }
        }
    }
}

pub fn name_locals(block: &mut Block, rename: bool) {
    if crate::past_post_deadline() {
        return;
    }
    crate::reset_walk_seen();
    let type_hints = if rename {
        // Usage-based type inference is only useful for the Luau path,
        // where `rename` enables the broader semantic-naming pipeline;
        // the lua51 path deliberately keeps names minimal/synthetic.
        type_inference_naming::collect_type_hints(block)
    } else {
        FxHashMap::default()
    };
    let mut namer = Namer {
        rename,
        counter: 1,
        upvalues: FxHashSet::default(),
        numeric_for_depth: 0,
        name_uses: FxHashMap::default(),
        type_hints,
    };
    crate::reset_walk_seen();
    namer.find_upvalues(block);
    crate::reset_walk_seen();
    namer.name_locals(block);
}

#[cfg(test)]
mod cycle_tests {
    use super::*;
    use crate::{share_block, If, Literal, Statement};

    #[test]
    fn cyclic_else_does_not_deadlock_name_or_format() {
        let shared = share_block(Block::default());
        let inner = If {
            condition: Literal::Boolean(true).into(),
            then_block: share_block(Block::default()),
            else_block: shared.clone(),
        };
        shared.lock().0.push(Statement::If(inner));
        let mut body = Block(vec![Statement::If(If {
            condition: Literal::Boolean(false).into(),
            then_block: share_block(Block::default()),
            else_block: shared,
        })]);
        name_locals(&mut body, true);
        let s = body.to_string();
        assert!(s.contains("if"), "formatter produced: {s:?}");
        assert!(s.contains("end"), "formatter produced: {s:?}");
        assert!(s.len() < 64 * 1024, "runaway format: {} bytes", s.len());
    }

    #[test]
    fn duplicate_debug_names_are_uniquified() {
        use crate::{Assign, Local, Literal, RcLocal};

        let a = RcLocal::new(Local::new(Some("insert".into())));
        let b = RcLocal::new(Local::new(Some("insert".into())));
        let _keep_a = a.clone();
        let _keep_b = b.clone();
        let mut a1 = Assign::new(vec![a.clone().into()], vec![Literal::Nil.into()]);
        a1.prefix = true;
        let mut a2 = Assign::new(vec![b.clone().into()], vec![Literal::Nil.into()]);
        a2.prefix = true;
        let mut body = Block(vec![a1.into(), a2.into()]);
        name_locals(&mut body, true);
        let na = a.0 .0.lock().0.clone().unwrap();
        let nb = b.0 .0.lock().0.clone().unwrap();
        assert_ne!(na, nb, "colliding debug names were not uniquified: {na} / {nb}");
        assert!(na == "insert" || nb == "insert", "{na} / {nb}");
    }
}
