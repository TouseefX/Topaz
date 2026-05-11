use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    time::Instant,
};

use clap::Parser;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    #[clap(short, long)]
    file: String,
}

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let args = Args::parse();
    let path = Path::new(&args.file);
    let mut input = File::open(path)?;
    let mut buffer = vec![0; input.metadata()?.len() as usize];
    input.read_exact(&mut buffer)?;

    let start = Instant::now();
    let res = lua51_lifter::decompile_bytecode(&buffer);
    let duration = start.elapsed();

    let mut out = File::create(path.with_extension("dec.51.lua").file_name().unwrap())?;
    writeln!(out, "-- decompiled by Topaz (took {:?})", duration)?;
    writeln!(out, "{}", res)?;

    Ok(())
}
