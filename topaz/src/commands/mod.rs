use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod decompile;
pub use decompile::*;
mod serve;
pub use serve::*;

#[derive(Debug, Subcommand)]
pub enum Commands {
    
    Decompile {

        #[arg(short, long)]
        input: PathBuf,


        #[arg(short, long)]
        output: PathBuf,


        #[arg(short, long, default_value_t = default_encode_key())]
        encode_key: u8,


        #[arg(short, long, default_value_t = false)]
        lua51: bool,

        /// Force the ruau-bytecode crate as the deserializer backend.
        /// By default Topaz now prefers ruau for plain Luau bytecode
        /// and falls back to the built-in deserializer for shuffled /
        /// custom-key inputs that still need the legacy path.
        #[arg(long, default_value_t = false)]
        ruau: bool,
    },

    
    Serve {
        
        #[arg(short, long, default_value_t = 3000)]
        port: u16,

        
        #[arg(short, long, default_value_t = true)]
        luau: bool,

        
        #[arg(short, long, default_value_t = false)]
        lua51: bool,
    },
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
