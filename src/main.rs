//! Command-line interface for [`apiproxy`](https://crates.io/crates/apiproxy).
//!
//! Parses the CLI and dispatches each subcommand to the appropriate module.

use clap::{Parser, Subcommand};

use crate::error::Result;

mod config;
mod error;
mod hosts;
mod serve;

#[derive(Parser)]
#[command(name = "apiproxy", version, about = "Store HTTP endpoints and proxy them")]
struct Cli {
    #[arg(long, global = true, default_value = "apiproxy.json")]
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
    Ls,
    /// Start the proxy
    Serve,
    /// Show or change config
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Set the hostname (updates /etc/hosts)
    Hostname { value: String },
    /// Set the bind IP (updates /etc/hosts)
    BindIp { value: String },
    /// Set the port
    Port { value: u16 },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Add { url, key } => config::add(&cli.config, &url, &key)?,
        Command::Rm { key } => config::remove(&cli.config, &key)?,
        Command::Ls => config::list(&cli.config)?,
        Command::Serve => serve::run(&cli.config).await?,
        Command::Config { action } => match action {
            None => config::show(&cli.config)?,
            Some(ConfigAction::Hostname { value }) => config::set_hostname(&cli.config, &value)?,
            Some(ConfigAction::BindIp { value }) => config::set_bind_ip(&cli.config, &value)?,
            Some(ConfigAction::Port { value }) => config::set_port(&cli.config, value)?,
        },
    }
    Ok(())
}
