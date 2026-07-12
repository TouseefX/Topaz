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

        /// Force the luaur-compatible plain-opcode path (encode key 1).
        /// Default already prefers that for LBC versions 3..=11, then falls
        /// back to the native deserializer with encode-key detection for
        /// Roblox client dumps (key 203).
        #[arg(long, default_value_t = false, alias = "ruau")]
        luaur: bool,
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
