
use std::path::PathBuf;

use fmp_client::Client;
use fmp_interface::command::ServerCommand;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, author, about)]
struct Cli {
    #[arg(short, long, global = true)]
    verbose: bool,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Play {
        path: PathBuf,
    },
    Clear,
    Exit,
    Quit,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    let mut client = Client::connect().await?;
    
    if let Some(e) = client.identify("test").await? { Err(e)? }
    
    match cli.command {
        Commands::Play { path } => client.command(ServerCommand::Play { path })?,
        Commands::Clear => client.command(ServerCommand::Clear)?,
        Commands::Exit | Commands::Quit => client.command(ServerCommand::Exit)?,
    }
    
    client.command(ServerCommand::Disconnect { client: client.name().unwrap().to_owned() })?;
    client.wait_commands().await?;
    
    Ok(())
}
