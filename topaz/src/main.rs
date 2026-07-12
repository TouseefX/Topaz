use clap::Parser;

use crate::commands::{Cli, decompile, serve};

mod commands;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default().with_max_level(log::LevelFilter::Info),
        );
        log::info!("Topaz CLI starting on Android");
    }

    // Tracing subscriber for desktop and android (falls back to log on android)
    #[cfg(not(target_os = "android"))]
    {
        let subscriber = tracing_subscriber::fmt()
            .compact()
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_target(false)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("failed to set global tracing subscriber");
    }
    #[cfg(target_os = "android")]
    {
        // On Android we still set tracing subscriber but also log via android_logger
        let subscriber = tracing_subscriber::fmt()
            .compact()
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_target(false)
            .with_writer(std::io::stderr)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    }

    let cli = Cli::parse();
    match cli.command {
        commands::Commands::Decompile {
            input,
            output,
            encode_key,
            lua51,
            luaur,
        } => decompile(&input, &output, encode_key, lua51, luaur)?,
        commands::Commands::Serve { port, luau, lua51 } => serve(port, luau, lua51).await?,
    }

    Ok(())
}
