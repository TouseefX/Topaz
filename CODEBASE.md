# Topaz codebase map

Working notes for this repo so later edits match **this** tree, not Medal
upstream or a guessed API. Read this before touching SSA, AST sharing, or
the formatter.

Topaz is a **Luau (and Lua 5.1) bytecode decompiler** forked from Medal.
Studio / Roblox 2026 dumps are the truth; `luau-lang/luau` bytecode is
**not** the same as Roblox client bytecode.

## Toolchain

| Item | Value |
| --- | --- |
| Workspace edition | **2024** (`Cargo.toml`) |
| Channel | **nightly** (`rust-toolchain.toml`) |
| Extra targets | `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android` |
| Resolver | 3 |
| Panic | `unwind` (catch_unwind is load-bearing) |
| Mutex | `parking_lot::Mutex` — **never** `ReentrantMutex` |
| Arc | `triomphe::Arc` (not `std::sync::Arc`) |
| Graphs | petgraph fork `jujhar16/petgraph` branch `ensure_len_resize_with` |

`luau-lifter` is still `edition = "2021"` in its own `Cargo.toml`;
everything else inherits workspace 2024.

### Integer inference (E0689)

Edition 2024 / current nightly will **not** pick a type for an unsuffixed
literal that only has a method called on it:

```rust
// BAD — E0689: can't call method `saturating_add` on `{integer}`
Statement::If(i) => 1.saturating_add(shared_weight(...))

// GOOD — sibling arms already did this
Statement::If(i) => 1usize.saturating_add(shared_weight(...))
Statement::While(w) => 1usize.saturating_add(...)
_ => 1,  // OK: match return type is usize, no method on the literal
```

Same class of bug: `0.saturating_sub`, `1.checked_add`, `2.pow`. Suffix
the literal (`1usize`, `0u32`) or bind `let n: usize = 1`.

### Other compile traps seen here

- `traverse_rvalues` needs `&mut self` on the statement — iterating
  `&block.0` then calling it is E0596.
- `Statement::clone()` / `If::clone()` is **shallow** (`Arc` clone of
  `then_block` / `else_block`). Use `deep_clone_statement` /
  `Block::deep_clone` when splicing a copy that must not alias.
- `RcLocal` is `Hash` by creation-order **id**, `Eq` by `ByAddress`.
  Do not mix with pointer-hash maps unless you mean identity.
- `local.0 .0.lock()` is the name cell (`ByAddress<Arc<Mutex<Local>>>`).
- Nightly `let_chains` are used in `restructure` / `cfg` (`if … && let`).
- `#![feature(box_patterns)]` is **gone** — do not re-add it.

## Crates

```
topaz            CLI (`decompile`, `serve`). version 1.3.1-beta
topaz-gui        egui desktop / Android UI + CFG viewer
luau-lifter      Roblox/Luau decode → CFG lift → SSA → AST → string
lua51-lifter     Lua 5.1 path (separate deserializer)
lua51-deserializer
cfg              CFG graph, SSA construct/destruct, jump structuring
ast              IR + formatter + post passes
restructure      CFG → structured `if`/`while`/`for` (`restructure::lift`)
luau-worker      Cloudflare worker wrapper (not the decompile core)
```

Dependency direction (do not invert):

```
topaz / topaz-gui
    → luau-lifter / lua51-lifter
        → restructure → cfg → ast
```

`ast` must not depend on `cfg`. `cfg` depends on `ast` (blocks hold
`ast::Block`).

## End-to-end Luau pipeline

Entry: `luau_lifter::decompile_bytecode` / `_default` / `_via_luaur`.

1. **`begin_decompile_deadline`** — wall clock **before** decode.
   Default 180s, CLI `--time-budget`, env `TOPAZ_TIME_BUDGET_SECS`.
   Thread-local: `cfg::set_decompile_deadline`. **Rayon / spawned
   threads do not inherit it** — copy the `Instant` onto every worker
   (`decompile_one`, post thread, fallback format on main).
2. **Decode** `deserializer::loadsafe_ir::decode_chunk`.
   - Plain opcodes: encode key **1** (`luau-compile`, luaur).
   - Roblox client: key **203** (op-byte only: `op' = op * key`).
   - `detect_encode_key` tries preferred, 203, 1, 227.
   Version range luaur-aligned **3..=11**. Roblox custom bytecode ≠
   luau-lang/luau.
3. **Lift** `Lifter::lift` (`luau-lifter/src/lifter.rs`, ~2k lines) →
   `cfg::function::Function` (petgraph of `ast::Block` + `BlockEdge`).
   Runs on a **64 MB** stack thread (`topaz-lift`).
4. **Per function** `decompile_function`:
   - `cfg::ssa::construct` (phi / params / upvalues)
   - loop: `structure_jumps` → `ssa::inline` → `structure_conditionals`
     → `remove_unnecessary_params` (cap 1–4 iters by CFG size)
   - `ssa::Destructor::destruct`
   - on panic: `flatten_cfg` + comment, **do not** restructure a
     half-destructed graph (parking_lot deadlock)
   - `restructure::lift` → structured `ast::Block`
   - `flatten_string_dispatch` then `fold_copy_locals_with` (skip
     upvalues; only fold `a = b` when `b` has **exactly one** assign
     in this function)
   - `LocalDeclarer::declare_locals` (ignore params+upvalues; skip
     for-binders; skip **never-written** SSA temps)
   - `post_process::apply_all_with_upvalues` (small bodies)
5. **Waves** of nested closures (children before parents, rayon, 64 MB
   stacks).
6. **Post-wave** thread `topaz-post` (also 64 MB):
   `link_upvalues` → `apply_context_naming` → `propagate_names` →
   `inline_short_gotos` → `apply_guard_clauses` → `name_locals` →
   `render_complete`.
   Snapshots (deep clone) after naming / goto so a hang can still emit.
7. **Render** `Block` `Display` → `formatter::Formatter`.
   Caps: 32 MB complete / 8 MB fallback. Step cap **1_500_000** visits.
   Stops on `past_post_deadline`. Luau has **no goto**: leftovers become
   comments via `sanitize_for_luau`.
8. CLI writes the file **always**. If the text contains
   `TOPAZ_INCOMPLETE` / unstructured IR / formatter-stopped / skipped
   budget → **exit non-zero**. Parse ≠ compile ≠ correct function.

## Key types

### AST (`ast`)

| Type | Notes |
| --- | --- |
| `RValue` | Local, Global, Call, Index, Unary, Binary, Closure, Literal, Table, Select |
| `LValue` | Local, Global, Index |
| `Statement` | Assign, If, While, Repeat, NumericFor, GenericFor, Return, Goto, Label, … plus SSA leftovers `NumForInit/Next`, `GenericForInit/Next`, `SetList`, `Close` |
| `Block(Vec<Statement>)` | Owned statement list |
| `SharedBlock = Arc<Mutex<Block>>` | `If.then_block` / `else_block`, loop bodies |
| `SharedFunction` | Nested closures |
| `If` | `condition: RValue`, `then_block`/`else_block: SharedBlock`. `If::new` wraps with `share_block` |
| `Assign` | `prefix` ⇒ `local x = …`; `parallel`; `compound_op` |
| `RcLocal` | Identity local. Debug name in `local.0.0.lock().0` |
| `Function` (ast) | name, line, parameters, variadic, `body: Block` |

`If` / `While` / `For` **do not** store owned `Block`. They store
`SharedBlock`. That is why shallow `clone()` aliases control flow.

### CFG (`cfg`)

- `Function`: `StableDiGraph<ast::Block, BlockEdge>`
- `BlockEdge`: `BranchType::{Unconditional, Then, Else}` + SSA `arguments`
  (`Vec<(RcLocal, RValue)>` phi args)
- Unique-pred jump merge (`structure_jumps`, `restructure::jump::match_jump`)
  **appends the successor block into the predecessor**. Do not skip all
  merges into then/else arms (`is_branch_arm` did that, v12 hang). Empty
  edges that drop a real join swallow the rest of a dispatcher into one
  `else` (v11 IOInvis).

### Deadlines (thread-local, two of them)

| TLS | Module | Used by |
| --- | --- | --- |
| `cfg::DECOMPILE_DEADLINE` | `cfg/src/lib.rs` | SSA, lift, collapse |
| `ast::POST_DEADLINE` | `ast/src/lib.rs` | goto inline, guard clauses, name_locals, **formatter** |

Main-thread fallback format **must** call `ast::set_post_deadline` (and
`cfg::set_decompile_deadline`) or `Display` runs unbounded. v12 0-byte
kill was this.

## Walk / lock rules

- `parking_lot::Mutex` is **not** reentrant. Nested `lock()` on the same
  `SharedBlock` deadlocks at 0% CPU.
- Always `try_lock`. On failure, skip / assume cycle.
- `visit_shared` / `visit_shared_mut` / `walk_seen_insert` skip an Arc
  already seen (DAG + cycle).
- Formatter `FORMAT_PATH` skips only **path** cycles so DAG-shared
  then-bodies still print twice. That reprint is exponential on a fat
  else-DAG → `MAX_FORMAT_STEPS` + deadline.
- Never hold `then_block` and `else_block` locks together (`format_if`).
- Drop a guard before recursing into a child that might alias it.

## Clone rules

| API | What it does |
| --- | --- |
| `Statement::clone` / `If::clone` | Arc clone. **Aliases** then/else |
| `deep_clone_statement` | New mutexes; cycle-safe via `CLONE_BLOCKS` |
| `Block::deep_clone` | Same, plus nested closures (`CLONE_FUNCS`) |

Goto-tail inlining **must** deep-clone. Nested weight of a tail (not
top-level statement count) is capped at 64 so a 50k-line `else` is not
a “short tail”.

## Post passes (order matters)

Per-function (small): `flatten_string_dispatch` → `fold_copy_locals_with`
→ `LocalDeclarer` → `post_process` (cond_expr, table_cleanup, copy_fold,
compound_assign, unused_vars, …).

Whole-script post-wave: naming → **inline_short_gotos** → guard clauses
→ name_locals → format.

`fold_copy_locals_with`:

- Do not fold into **upvalues**.
- Do not fold `a = b` when `b` has `src_w != 1` (never assigned, or
  assigned more than once).
- CameraShaker `Update` broke when both pos/rot folded into the module
  `Vector3.zero`.

`LocalDeclarer`:

- `binder_depth` skips for-binders (v7 `local i, v` shadow).
- `written` skips never-assigned temps (v12 `local v28, v29` inside
  `for _, v28 in …`).

## Files (one line)

### `ast/src`

| File | Role |
| --- | --- |
| `lib.rs` | `RValue`/`Statement`/`Block`/`SharedBlock`, deadlines, deep_clone, sanitize |
| `formatter.rs` | Luau printer; path cycle + step cap + deadline |
| `inline_gotos.rs` | Join-goto / skip-rest / back-edge while / short-tail inline |
| `local_declarations.rs` | Insert `local` at dominance frontier |
| `name_locals.rs` | `vN` / `v_uN` / k,v for-binders; uniquify debug names |
| `fold_copies.rs` | SSA copy fold (the one used from luau-lifter) |
| `copy_fold.rs` | Older/smaller fold used from `post_process` |
| `flatten_dispatch.rs` | Collapse inverted `x ~= "K"` wrappers (IOInvis-shaped) |
| `guard_clauses.rs` | Tail `if` → early-exit / invert |
| `local.rs` | `RcLocal` identity + deterministic Hash id |
| `if.rs` / `while.rs` / `for.rs` | Structured control nodes |
| `assign.rs` | Assign + compound ops |
| `post_process.rs` | Quality bundle after destruct |
| `traverse.rs` | `Traverse` / `visit_shared*` |

### `cfg/src`

| File | Role |
| --- | --- |
| `function.rs` | CFG container |
| `block.rs` | `BlockEdge` / `BranchType` |
| `ssa/construct.rs` | SSA construction, param removal, local map |
| `ssa/destruct.rs` | SSA destruction (liveness in `destruct/liveness.rs`) |
| `ssa/inline.rs` | Copy/constant inline in CFG |
| `ssa/structuring.rs` | `structure_jumps` / `structure_conditionals` |
| `ssa/upvalues.rs` | Upvalue grouping |
| `lib.rs` | Budget TLS |
| `dominators.rs` / `dom_index.rs` | O(1) dominates |

### `restructure/src`

| File | Role |
| --- | --- |
| `lib.rs` | `lift(Function) -> Block` driver |
| `jump.rs` | Unique-pred merge, unnecessary condition |
| `conditional.rs` | Diamond / triangle → `If` |
| `loop.rs` | while / numeric for / generic for |

Do **not** import Medal `loop.rs` wholesale. Terminal-for is a small
function only. Medal `simple_fast` is quadratic — Topaz has O(n) DomIndex.

### `luau-lifter/src`

| File | Role |
| --- | --- |
| `lib.rs` | Orchestration, post-wave, render, encode-key, incomplete marker |
| `lifter.rs` | Bytecode instructions → CFG |
| `deserializer/` | Chunk / function / list / leb128 / **loadsafe_ir** |
| `op_code.rs` | Opcode enum (keyed) |
| `instruction.rs` | Decoded instruction |
| `builtins.rs` | FASTCALL names |

### `topaz/src`

CLI clap: `decompile` (file in/out, `--encode-key`, `--lua51`, `--luaur`,
`--time-budget`) and `serve` (axum). Incomplete output is a failed
`decompile` (`ErrorKind::Other`) **after** the file is written.

## Comparators / fixtures in the repo root

| Path | Use |
| --- | --- |
| `ClientRenderer.luau` | Cleaned comparator (does compile). **Not** a substitute for a decompile dump |
| `ClientRenderer_LocalScript_*_Bytecode.txt.txt` | The large Roblox dump (~22 MB text) |
| `CameraShaker_ModuleScript_*` | LunaUX-era bytecode; readable ref is LunaUX V1.4.6 |
| `sample.luau.bin` / `sample.luac` | Small runtime-ok sample |
| `v7-ClientRenderer.luau` | Historical dump |
| `scripts/request_path_gates.py` | Grep `request2 ==` paths in a dump |

Large scripts: **60k+ lines**, bytecode **1–4 MB**, decoded IR ~16 MB.
Every new walk must be O(n) / O(1) dominates, not O(n²).

## Standing product rules

- Decompile **every** function. Do not skip SSA.
- Never emit `goto` / `::label::` in the final file (comment them).
- Timeout / panic must **not** throw away the decompile: incomplete
  file + nonzero exit.
- Exit 0 without `TOPAZ_INCOMPLETE` is **not** automatically valid.
  String coverage ≠ completeness. Do not ship `CLEANED-complete.luau`
  as if it were the decompiler output.
- Do not execute / distribute ClientRenderer as working.
- CameraShaker harness (when you have a binary): mocked `Start` binds
  once + `_running=true`; `Update` twice **equal** angles; event
  handler uses its argument. Two **local** pos/rot inits (v10 Y
  `0.017453292519943295` twice), not module zero + nil.
- Do not `keep_unsplit` every incoming upvalue.
- Do not re-pin stable; Android rustup targets stay.
- Do not re-enable `try_coalesce_copy_by_sharing` for n>2000.
- Do not treat parse-0 as success.

## Open quality (do not claim fixed without a dump)

| Item | Last known |
| --- | --- |
| ClientRenderer full `luau-compile` | Fail (`cFrame652`, 200-local / `v_u8020` history) |
| `request2 ==` not only under IOInvis else | v11 dump nested 851 later tests; v12 had **no file** |
| CameraShaker `Update` | Unassigned accum + module zero (v11/v12) |
| CameraShaker `StopSustained` | Binder `v28` shadowed by `local v28, v29`; empty `if fadeOutDuration ~= 0` |
| Sample | Runtime match v11 — keep it green |

v12 hang (0 bytes, exit 137, CYCLE else→else depth 19) is the reason
for: deep-clone tails, tail weight, formatter step cap, main-thread
fallback deadline, cyclic AST → `render_incomplete`, revert of
`is_branch_arm`.

## How to build / run

```bash
cargo test -p ast --lib
cargo build --release --bin topaz

# Guarded ClientRenderer (do not omit the outer timeout)
TOPAZ_DEBUG_CYCLES=1 timeout -s KILL 240 \
  ./target/release/topaz decompile \
  --time-budget 120 \
  ClientRenderer_LocalScript_88615311464962_Bytecode.txt.txt \
  out.luau
```

Env: `TOPAZ_DEBUG_CYCLES=1` (pass timings + cycle report), `=2` (per-round
goto). `TOPAZ_TIME_BUDGET_SECS` if CLI flag omitted.

Do **not** run that LBC unbounded.

## Medal

Upstream: Babyhamsta/medal-decompiler (Lua 5.1 / IronBrew). Improved
fork is readability-only. Do not replace Roblox 1–14 deserializer or
whole `loop.rs`. Medal AST uses LocalRefs/smallvec; Topaz does not.
Medal `control_flow_cleanup` overlaps `guard_clauses` — skip a blind port.
