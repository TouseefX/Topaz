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

        /// Use the ruau-bytecode crate as the deserializer backend
        /// instead of Topaz's built-in deserializer. This handles
        /// the luau-compile 0.728 typeinfo format correctly for
        /// bytecodes that the built-in deserializer can't parse.
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
