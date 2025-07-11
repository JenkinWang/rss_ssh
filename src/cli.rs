use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rssh", version = "1.0", about = "A secure SSH login management tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new SSH connection
    Add {
        #[arg(help = "A unique alias for the connection")]
        alias: String,
        #[arg(help = "Connection string in user@host format")]
        connection_string: String,
    },
    /// List all saved SSH connections
    List,
    /// Remove a saved SSH connection
    Remove {
        #[arg(help = "The alias of the connection to remove")]
        alias: String,
    },
    /// Connect to a server using a saved alias
    Connect {
        #[arg(help = "The alias of the connection to use")]
        alias: String,
        #[arg(short, long, help = "The port to connect to", default_value_t = 22)]
        port: u16,
        #[arg(short, long, help = "Path to the private key file")]
        identity: Option<PathBuf>,
    },
    /// Upload a file to a remote directory
    Upload {
        #[arg(help = "The alias of the connection to use")]
        alias: String,
        #[arg(help = "Local file to upload")]
        local_path: PathBuf,
        #[arg(help = "Remote directory to save the file in")]
        remote_path: PathBuf,
        #[arg(short, long, help = "The port to connect to", default_value_t = 22)]
        port: u16,
        #[arg(short, long, help = "Path to the private key file")]
        identity: Option<PathBuf>,
    },
    /// Download a file to a local directory
    Download {
        #[arg(help = "The alias of the connection to use")]
        alias: String,
        #[arg(help = "Remote file to download")]
        remote_path: PathBuf,
        #[arg(help = "Local directory to save the file in")]
        local_path: PathBuf,
        #[arg(short, long, help = "The port to connect to", default_value_t = 22)]
        port: u16,
        #[arg(short, long, help = "Path to the private key file")]
        identity: Option<PathBuf>,
    },
}
