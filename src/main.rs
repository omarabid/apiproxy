use clap::{Parser, Subcommand};

use crate::error::Result;

mod error;
mod routes;
mod serve;

#[derive(Parser)]
#[command(name = "apiproxy", version, about = "Store HTTP endpoints and proxy them")]
struct Cli {
    #[arg(long, global = true, default_value = "apiproxy.jsonl")]
    config: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Register an API URL under a key
    Add { url: String, key: String },
    /// Remove a key
    Rm { key: String },
    /// List keys
    Ls {
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
    /// Start the proxy
    Serve {
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Add { url, key } => routes::add(&cli.config, &url, &key)?,
        Command::Rm { key } => routes::remove(&cli.config, &key)?,
        Command::Ls { port } => routes::list(&cli.config, port)?,
        Command::Serve { port } => serve::run(&cli.config, port).await?,
    }
    Ok(())
}
