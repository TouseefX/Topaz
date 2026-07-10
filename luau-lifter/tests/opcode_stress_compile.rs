//! Compile-and-decompile stress test.
//!
//! This is the more thorough cousin of `opcode_stress.rs`: instead of
//! relying on pre-compiled `.luau.bin` fixtures (which only cover
//! whatever the original author happened to write), this test walks a
//! `tests/stress_sources/` directory of `.luau` source files,
//! *compiles* each one to bytecode using an external `lune` binary,
//! decompiles the bytecode with the lifter, and asserts the decompiler
//! never panics or silently drops a handled instruction.
//!
//! The 15 sources currently in `stress_sources/` cover:
//! - arithmetic (`+ - * / % // ^`)
//! - bitwise (via `bit32` library, since lune's parser is version-sensitive)
//! - string concatenation, length, indexing, library calls
//! - table constructors, indexed access, iteration, library calls
//! - `if/elseif/else`, `while`, `repeat`, numeric `for`, generic `for`,
//!   `break`, `continue`, multi-return
//! - closures, upvalue capture, varargs
//! - logical `and`/`or`, short-circuit chains, nil-coalescing patterns
//! - `pcall`, `error`, `xpcall` (basic)
//! - method calls, `self`, `setmetatable`
//! - `string.*`, `math.*`, `type`, `typeof`, `select`, `raw*`
//! - compound assignments (`+=`, `-=`, `*=`, `/=`, `%=`, `//=`, `^=`)
//! - complex multi-function scripts
//!
//! To run this test, install `lune` from
//! <https://lune-org.github.io> and ensure `lune` is on `PATH` (or
//! available at `/usr/local/bin/lune`, `/usr/bin/lune`, `./lune`, or
//! `../lune`).
//!
//! If `lune` is not available, the test is silently skipped (with a
//! printed warning) so it doesn't break CI in environments where
//! lune isn't installed. Set `TOPAZ_STRESS_REQUIRE_LUNE=1` to make
//! the test fail-fast instead of skipping when lune is missing.
//!
//! To add a new stress source, drop a `.luau` file into
//! `tests/stress_sources/` and the test will pick it up automatically.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const STRESS_DIR: &str = "tests/stress_sources";
const ENCODE_KEY: u8 = 203;

/// Compile a Luau source file to bytecode by invoking the lune
/// binary with a small driver script. The driver reads the source,
/// compiles it via `@lune/luau`, and writes the bytecode to a known
/// temp file. We then read that file from Rust. We can't write the
/// bytecode to lune's stdout (lune has no `io` library; `print`
/// would work but adds a trailing newline), so we use a temp file
/// instead.
fn compile_via_lune(lune: &Path, source: &Path) -> Result<Vec<u8>, String> {
    let driver = r#"
local luau = require("@lune/luau")
local fs = require("@lune/fs")
local process = require("@lune/process")

local source_path = process.args[2]
local output_path = process.args[3]

local source = fs.readFile(source_path)
local bytecode = luau.compile(source, {
    optimizationLevel = 1,
    debugLevel = 2,
})
fs.writeFile(output_path, bytecode)
"#;
    // Unique-per-call temp file path so parallel tests don't clobber each other's driver script.
    let driver_path = {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        std::env::temp_dir().join(format!("topaz_compile_driver_{}_{}_{}.luau", std::process::id(), ts, "src"))
    };
    let output_path = {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        std::env::temp_dir().join(format!("topaz_compile_output_{}_{}_{}.bin", std::process::id(), ts, "src"))
    };
    fs::write(&driver_path, driver).map_err(|e| format!("write driver: {e}"))?;

    let output = Command::new(lune)
        .arg("run")
        .arg(&driver_path)
        .arg("--")
        .arg(source)
        .arg(&output_path)
        .output()
        .map_err(|e| format!("failed to spawn lune: {e}"))?;
    let _ = fs::remove_file(&driver_path);

    if !output.status.success() {
        let _ = fs::remove_file(&output_path);
        return Err(format!(
            "lune exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let bytecode = fs::read(&output_path).map_err(|e| format!("read bytecode: {e}"))?;
    let _ = fs::remove_file(&output_path);
    Ok(bytecode)
}

/// Recompile a decompiled Luau source by running it under lune
/// (this catches *syntax* errors but accepts runtime errors, since
/// the decompiled output may reference names that aren't in the
/// original script's scope).
fn check_recompiles_cleanly(lune: &Path, source: &str) -> Result<(), String> {
    let path = {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        std::env::temp_dir().join(format!("topaz_decompiled_recompile_{}_{}_{}.luau", std::process::id(), ts, "recomp"))
    };
    fs::write(&path, source).map_err(|e| format!("write temp: {e}"))?;
    let output = Command::new(lune)
        .arg("run")
        .arg(&path)
        .output()
        .map_err(|e| format!("spawn lune: {e}"))?;
    let _ = fs::remove_file(&path);

    // Distinguish parse errors (bad) from runtime errors (acceptable
    // — the decompiler may produce semantically different code that
    // happens to error at runtime, e.g. calling `print` when the
    // original script had it in scope but the decompiler didn't
    // preserve the scope).
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let first_line = stderr.lines().next().unwrap_or("").trim();
    if first_line.contains("Expected")
        || first_line.contains("syntax error")
        || first_line.contains("Syntax")
        || first_line.starts_with("[ERROR]") && first_line.contains("parse")
    {
        return Err(format!("syntax error: {first_line}"));
    }
    // Otherwise treat as runtime error — acceptable.
    Ok(())
}

fn find_lune() -> Option<PathBuf> {
    let candidates = [
        "/usr/local/bin/lune",
        "/usr/bin/lune",
        "./lune",
        "../lune",
        "./target/lune",
    ];
    for c in candidates {
        let p = Path::new(c);
        if p.exists() && p.is_file() {
            return Some(p.to_path_buf());
        }
    }
    if let Some(p) = std::env::var_os("LUNE_BIN") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    which::which("lune")
}

/// Try to find `lune`; if not present, either skip (default) or panic
/// (when `TOPAZ_STRESS_REQUIRE_LUNE=1`). Returns the path to use.
fn require_lune() -> Option<PathBuf> {
    match find_lune() {
        Some(p) => Some(p),
        None => {
            let force = std::env::var("TOPAZ_STRESS_REQUIRE_LUNE")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false);
            if force {
                panic!(
                    "TOPAZ_STRESS_REQUIRE_LUNE=1 but `lune` was not found on PATH.\n\
                     Install it from https://lune-org.github.io and re-run, or unset\n\
                     TOPAZ_STRESS_REQUIRE_LUNE to allow the test to skip."
                );
            }
            eprintln!(
                "skipping compile-and-decompile stress test: `lune` not found on PATH.\n\
                 Install lune (https://lune-org.github.io) and ensure it's on PATH\n\
                 to run this test, or set TOPAZ_STRESS_REQUIRE_LUNE=1 to make it\n\
                 fail-fast instead of skipping."
            );
            None
        }
    }
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

#[test]
fn compile_and_decompile_every_stress_source() {
    let Some(lune) = require_lune() else { return };

    let sources = collect_sources();
    assert!(
        !sources.is_empty(),
        "no .luau sources in {STRESS_DIR}; nothing to stress-test"
    );

    let mut failures: Vec<(String, String)> = Vec::new();
    for src in &sources {
        let name = src
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>")
            .to_string();

        let bytecode = match compile_via_lune(&lune, src) {
            Ok(b) => b,
            Err(e) => {
                failures.push((name, format!("lune compile failed: {e}")));
                continue;
            }
        };

        if bytecode.is_empty() {
            failures.push((name, "lune produced empty bytecode".into()));
            continue;
        }

        let output = luau_lifter::decompile_bytecode(&bytecode, ENCODE_KEY);
        if output.contains("failed to decompile") {
            failures.push((name, "decompiler reported 'failed to decompile'".into()));
            continue;
        }
        if output.contains("unhandled instruction") {
            failures.push((name, "decompiler dropped a handled instruction".into()));
            continue;
        }
        if !output.starts_with("-- Decomplied with Topaz") {
            failures.push((
                name,
                format!("output missing standard header; first 80 chars: {:?}", &output[..output.len().min(80)])
            ));
            continue;
        }
        if std::env::var("TOPAZ_STRESS_VERBOSE").is_ok() {
            eprintln!("  OK: {name}");
        }
    }

    if !failures.is_empty() {
        let mut msg = format!("{} stress source(s) failed:\n", failures.len());
        for (name, reason) in &failures {
            msg.push_str(&format!("  - {name}: {reason}\n"));
        }
        panic!("{msg}");
    }
}

/// Same as above, but additionally verifies the decompiled output
/// parses as a syntactically valid Luau program (when re-compiled by
/// lune). This catches "decompiles without crashing but produces
/// invalid Lua" bugs that the other test misses.
#[test]
fn decompiled_output_recompiles_cleanly() {
    let Some(lune) = require_lune() else { return };

    let sources = collect_sources();
    assert!(
        !sources.is_empty(),
        "no .luau sources in {STRESS_DIR}; nothing to stress-test"
    );

    let mut failures: Vec<(String, String)> = Vec::new();
    for src in &sources {
        let name = src
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>")
            .to_string();

        let bytecode = match compile_via_lune(&lune, src) {
            Ok(b) => b,
            Err(e) => {
                failures.push((name, format!("lune compile failed: {e}")));
                continue;
            }
        };

        let decompiled = luau_lifter::decompile_bytecode(&bytecode, ENCODE_KEY);

        if let Err(e) = check_recompiles_cleanly(&lune, &decompiled) {
            failures.push((name.clone(), e));
        }
        if std::env::var("TOPAZ_STRESS_VERBOSE").is_ok() {
            eprintln!("  RE-OK: {name}");
        }
    }

    if !failures.is_empty() {
        let mut msg = format!(
            "{} stress source(s) had syntactically invalid decompilation:\n",
            failures.len()
        );
        for (name, reason) in &failures {
            msg.push_str(&format!("  - {name}: {reason}\n"));
        }
        panic!("{msg}");
    }
}

// Minimal `which` implementation since we can't pull in the `which` crate
// just for this. Mirrors the behavior of the `which` crate on Linux.
mod which {
    use std::path::PathBuf;

    pub fn which(cmd: &str) -> Option<PathBuf> {
        let path_var = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(cmd);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }
}
