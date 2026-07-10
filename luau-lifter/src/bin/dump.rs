use std::env;
use std::fs;
use std::process::ExitCode;

use luau_lifter::deserializer;
use luau_lifter::instruction::Instruction;
use luau_lifter::op_code::OpCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: dump <input.luau.bin>");
        return ExitCode::from(1);
    }
    let path = &args[1];
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("read failed: {e}");
            return ExitCode::from(1);
        }
    };
    // Brute-force the encode key by picking the key that produces the most
    // non-NOP instructions (proxy for "decoded correctly").
    let mut best: Option<(u8, deserializer::bytecode::Bytecode)> = None;
    let mut best_n = 0;
    for k in 0u8..=255 {
        match deserializer::deserialize(&bytes, k) {
            Ok(c) => {
                let chunk = match &c {
                    deserializer::bytecode::Bytecode::Chunk(c) => c,
                    _ => continue,
                };
                let mut n = 0;
                for f in &chunk.functions {
                    for ins in &f.instructions {
                        let op = match ins {
                            Instruction::BC { op_code, .. }
                            | Instruction::AD { op_code, .. }
                            | Instruction::E { op_code, .. } => op_code,
                        };
                        if *op != OpCode::LOP_NOP {
                            n += 1;
                        }
                    }
                }
                if n > best_n {
                    best_n = n;
                    best = Some((k, c));
                }
            }
            Err(e) => {
                if k == 0 {
                    eprintln!("[k=0] error: {}", e);
                }
            }
        }
    }
    let (key, c) = match best {
        Some(b) => b,
        None => {
            eprintln!("no encode key produced non-NOP instructions");
            return ExitCode::from(1);
        }
    };
    eprintln!("(encode_key={}, non-NOP instructions: {})", key, best_n);
    let chunk = match c {
        deserializer::bytecode::Bytecode::Chunk(c) => c,
        _ => return ExitCode::from(1),
    };
    for func_id in 0..chunk.functions.len() {
        let func = &chunk.functions[func_id];
        let name = if func_id as u32 == chunk.main {
            "main".to_string()
        } else {
            format!("func#{}", func_id)
        };
        println!("\n=== {} ({} insns) ===", name, func.instructions.len());
        for (pc, ins) in func.instructions.iter().enumerate() {
            println!("  pc={:3}  {:?}", pc, ins);
        }
    }
    ExitCode::from(0)
}
