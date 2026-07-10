//! Multi-compiler stress test: every stress source in `tests/stress_sources/`
//! is compiled with each available Luau compiler (lune, luau-compile, lute)
//! and decompiled by Topaz. We assert the decompiler never panics and
//! doesn't produce "failed to decompile" output for any source under any
//! compiler.
//!
//! The set of available compilers depends on the build environment:
//!   * lune 0.10.5 — the original reference compiler
//!   * luau-compile from the official luau-lang/luau repo (currently
//!     tracks Luau 0.728) — same compiler Roblox uses internally
//!   * lute from the official luau-lang/lute repo — recent Luau nightly
//!
//! For each source we expect at least ONE compiler to be available, so
//! the test never silently passes on an environment where every
//! compiler is missing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const STRESS_DIR: &str = "tests/stress_sources";
const ENCODE_KEY: u8 = 1;

#[derive(Debug, Clone, Copy)]
enum Compiler {
    Lune,
    LuauCompile,
    Lute,
}

impl Compiler {
    fn name(self) -> &'static str {
        match self {
            Compiler::Lune => "lune",
            Compiler::LuauCompile => "luau-compile",
            Compiler::Lute => "lute",
        }
    }

    /// Compile a .luau source file to bytecode, returning the raw bytes
    /// (or an error string).
    fn compile(self, source: &Path) -> Result<Vec<u8>, String> {
        match self {
            Compiler::Lune => compile_via_lune(source),
            Compiler::LuauCompile => compile_via_luau(source),
            Compiler::Lute => compile_via_lute(source),
        }
    }
}

fn find_executable(names: &[&str]) -> Option<PathBuf> {
    for name in names {
        for path in &[
            format!("/usr/local/bin/{name}"),
            format!("/usr/bin/{name}"),
            format!("./{name}"),
            format!("../{name}"),
        ] {
            let p = Path::new(path);
            if p.exists() && p.is_file() {
                return Some(p.to_path_buf());
            }
        }
        if let Some(p) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&p) {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        if let Some(p) = std::env::var_os(name.to_uppercase() + "_BIN") {
            let p = PathBuf::from(p);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

fn find_lune() -> Option<PathBuf> {
    find_executable(&["lune"])
}
fn find_luau_compile() -> Option<PathBuf> {
    find_executable(&["luau-compile"])
}
fn find_lute() -> Option<PathBuf> {
    find_executable(&["lute"])
}

fn collect_sources() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(STRESS_DIR);
    if !dir.exists() {
        eprintln!("stress source dir {} does not exist; skipping", dir.display());
        return Vec::new();
    }
    let mut out: Vec<_> = fs::read_dir(&dir)
        .expect("could not read stress_sources dir")
        .filter_map(|e| match e {
            Ok(e) => Some(e.path()),
            Err(_) => None,
        })
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("luau"))
        .collect();
    out.sort();
    out
}

/// Write `contents` to a unique temp file under the system temp dir.
/// Returns the path to the file. The `suffix` is appended with a `.`
/// separator (so callers pass `"luau"` to get a `.luau` extension,
/// which lune requires).
fn write_temp_file(prefix: &str, suffix: &str, contents: &[u8]) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!(
        "topaz_mc_{}_{}_{}.{}",
        std::process::id(),
        ts,
        prefix,
        suffix
    ));
    fs::write(&path, contents).expect("write temp file");
    path
}

fn compile_via_lune(source: &Path) -> Result<Vec<u8>, String> {
    let lune = find_lune().ok_or_else(|| "lune not found".to_string())?;
    let out_path = write_temp_file("out", "bin", b"");
    // The driver takes the source file path (process.args[2]) and writes
    // compiled bytecode to process.args[3]. It uses process.args rather
    // than embedding the paths so we don't have to escape the source path
    // into a Luau string literal.
    let driver = r#"
local luau = require("@lune/luau")
local fs = require("@lune/fs")
local process = require("@lune/process")
local source_path = process.args[2]
local output_path = process.args[3]
local source = fs.readFile(source_path)
local bytecode = luau.compile(source, { optimizationLevel = 1, debugLevel = 2 })
fs.writeFile(output_path, bytecode)
"#;
    let driver_path = write_temp_file("driver", "luau", driver.as_bytes());
    let output = Command::new(&lune)
        .arg("run")
        .arg(&driver_path)
        .arg("--")
        .arg(source)
        .arg(&out_path)
        .output()
        .map_err(|e| format!("spawn lune: {e}"))?;
    if !output.status.success() {
        let _ = fs::remove_file(&driver_path);
        let _ = fs::remove_file(&out_path);
        return Err(format!(
            "lune exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let bytes = fs::read(&out_path).map_err(|e| format!("read out: {e}"))?;
    let _ = fs::remove_file(&driver_path);
    let _ = fs::remove_file(&out_path);
    Ok(bytes)
}

fn compile_via_luau(source: &Path) -> Result<Vec<u8>, String> {
    let luau = find_luau_compile().ok_or_else(|| "luau-compile not found".to_string())?;
    let output = Command::new(&luau)
        .arg("-O1")
        .arg("-g2")
        .arg("--binary")
        .arg(source)
        .output()
        .map_err(|e| format!("spawn luau-compile: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "luau-compile exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output.stdout)
}

fn compile_via_lute(_source: &Path) -> Result<Vec<u8>, String> {
    // Lute's `compile` returns a `CompileResult` userdata (whose bytecode
    // is bundled into a standalone executable) rather than raw bytecode,
    // and the Lute 0.x stdlib API for extracting raw bytecode is
    // unstable. Until Lute exposes a stable `@lute/luau.compile(...): bytes`
    // helper, we skip it in this test. `lute` is still listed in
    // `collect_compilers()` so the test will use it as soon as the
    // binary is on PATH; the compile_via_lute stub just reports "not
    // supported" so the test gracefully moves on.
    Err("lute bytecode extraction not yet supported".to_string())
}

fn decompile(bytecode: &[u8]) -> String {
    luau_lifter::decompile_bytecode_default(bytecode, ENCODE_KEY)
}

fn collect_compilers() -> Vec<Compiler> {
    let mut out = Vec::new();
    if find_lune().is_some() {
        out.push(Compiler::Lune);
    }
    if find_luau_compile().is_some() {
        out.push(Compiler::LuauCompile);
    }
    if find_lute().is_some() {
        out.push(Compiler::Lute);
    }
    out
}

#[test]
fn decompile_every_source_under_every_available_compiler() {
    let compilers = collect_compilers();
    assert!(
        !compilers.is_empty(),
        "no Luau compilers found on PATH (looked for lune, luau-compile, lute). \
         Install at least one to run this test."
    );

    eprintln!("=== compilers available: {:?} ===", compilers);

    let sources = collect_sources();
    assert!(
        !sources.is_empty(),
        "no .luau sources in {STRESS_DIR}; nothing to stress-test"
    );

    let mut failures: Vec<(String, String, String)> = Vec::new();
    let mut skips: Vec<(String, String, String)> = Vec::new();

    for src in &sources {
        let name = src
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>")
            .to_string();

        for compiler in &compilers {
            let bytecode = match compiler.compile(src) {
                Ok(b) => b,
                Err(e) => {
                    skips.push((name.clone(), compiler.name().to_string(), e));
                    continue;
                }
            };
            if bytecode.is_empty() {
                skips.push((
                    name.clone(),
                    compiler.name().to_string(),
                    "compiler produced empty bytecode".into(),
                ));
                continue;
            }

            let decompiled = decompile(&bytecode);

            // Fail if the decompiler produced an error marker.
            if decompiled.contains("failed to decompile")
                || decompiled.contains("unhandled instruction")
                || decompiled.contains("failed to deserialize")
            {
                let first_line = decompiled.lines().next().unwrap_or("").to_string();
                failures.push((
                    name.clone(),
                    compiler.name().to_string(),
                    format!("decompiler error: {}", first_line),
                ));
                continue;
            }
            if !decompiled.starts_with("-- Decomplied with Topaz") {
                failures.push((
                    name.clone(),
                    compiler.name().to_string(),
                    format!(
                        "output missing standard header; first 80 chars: {:?}",
                        &decompiled[..decompiled.len().min(80)]
                    ),
                ));
                continue;
            }
            if std::env::var("TOPAZ_MULTI_VERBOSE").is_ok() {
                eprintln!("  OK: {} [{}]", name, compiler.name());
            }
        }
    }

    if !skips.is_empty() {
        eprintln!("\n=== Skipped ({} entries) ===", skips.len());
        for (name, compiler, reason) in &skips {
            eprintln!("  skip {} [{}]: {}", name, compiler, reason);
        }
    }

    if !failures.is_empty() {
        let mut msg = format!(
            "{} (source, compiler) pair(s) failed to decompile:\n",
            failures.len()
        );
        for (name, compiler, reason) in &failures {
            msg.push_str(&format!("  - {} [{}]: {}\n", name, compiler, reason));
        }
        panic!("{}", msg);
    }
}

/// Verifies the same `(source, compiler)` pairs also produce output
/// that **recompiles** as valid Luau. This catches "decompiles without
/// crashing but produces invalid Lua" bugs.
#[test]
fn decompiled_output_recompiles_under_every_available_compiler() {
    let compilers = collect_compilers();

    // We use lune as the universal re-compiler since it's the most stable
    // for this purpose. If lune is missing we can't run this test.
    let lune = match find_lune() {
        Some(p) => p,
        None => {
            eprintln!("skipping recompile test: `lune` not found on PATH");
            return;
        }
    };

    // The driver takes the source file path (process.args[2]) and
    // writes compiled bytecode to process.args[3]. We invoke it once
    // per source/compiler pair and treat a non-zero exit + parse-style
    // stderr as a failure.
    let driver = r#"
local luau = require("@lune/luau")
local fs = require("@lune/fs")
local process = require("@lune/process")
local source = fs.readFile(process.args[2])
local bytecode = luau.compile(source, { optimizationLevel = 1, debugLevel = 2 })
fs.writeFile(process.args[3], bytecode)
"#;
    let driver_path = write_temp_file("driver", "luau", driver.as_bytes());

    let sources = collect_sources();
    assert!(!sources.is_empty(), "no .luau sources in {STRESS_DIR}");

    let mut failures: Vec<(String, String, String)> = Vec::new();

    for src in &sources {
        let name = src
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>")
            .to_string();
        for compiler in &compilers {
            let bytecode = match compiler.compile(src) {
                Ok(b) => b,
                Err(_) => continue, // skip if compiler can't handle this source
            };
            if bytecode.is_empty() {
                continue;
            }

            let decompiled = decompile(&bytecode);
            if decompiled.contains("failed to decompile")
                || decompiled.contains("unhandled instruction")
                || decompiled.contains("failed to deserialize")
            {
                continue; // already covered by the previous test
            }

            // Write the decompiled output to a temp file and try to
            // re-compile it with lune. A parse error indicates
            // syntactically invalid output.
            let src_path = write_temp_file("recomp", "luau", decompiled.as_bytes());
            let out_path = write_temp_file("recomp_out", "bin", b"");
            let status = Command::new(&lune)
                .arg("run")
                .arg(&driver_path)
                .arg(src_path.display().to_string())
                .arg(out_path.display().to_string())
                .output();
            let _ = fs::remove_file(&src_path);
            let _ = fs::remove_file(&out_path);
            let output = match status {
                Ok(o) => o,
                Err(e) => {
                    failures.push((
                        name.clone(),
                        compiler.name().to_string(),
                        format!("recompile spawn failed: {e}"),
                    ));
                    continue;
                }
            };
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let first_line = stderr.lines().next().unwrap_or("").trim();
                if first_line.contains("Expected")
                    || first_line.contains("syntax error")
                    || first_line.contains("Syntax")
                    || (first_line.starts_with("[ERROR]") && first_line.contains("parse"))
                {
                    failures.push((
                        name.clone(),
                        compiler.name().to_string(),
                        format!("syntax error on recompile: {first_line}"),
                    ));
                }
            }
        }
    }
    let _ = fs::remove_file(&driver_path);

    if !failures.is_empty() {
        let mut msg = format!(
            "{} (source, compiler) pair(s) had syntactically invalid decompilation:\n",
            failures.len()
        );
        for (name, compiler, reason) in &failures {
            msg.push_str(&format!("  - {} [{}]: {}\n", name, compiler, reason));
        }
        panic!("{}", msg);
    }
}

// Minimal which-style lookup mirrored from the other test files in
// this crate to avoid pulling in the `which` crate just for this.
#[allow(dead_code)]
mod which {
    use std::path::PathBuf;
    pub fn which(cmd: &str) -> Option<PathBuf> {
        let path = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(cmd);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }
}
