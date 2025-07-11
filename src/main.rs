mod cli;
mod config;
mod credentials;
mod ssh;

use crate::cli::{Cli, Commands};
use crate::config::Config;
use crate::credentials::delete_password;
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use inquire::{Confirm, Select, Text};
use std::path::PathBuf;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load()?;

    match cli.command {
        Some(Commands::Add {
            alias,
            connection_string,
        }) => {
            config.connections.insert(alias.clone(), connection_string);
            config.save()?;
            println!("Connection '{}' added.", alias);
        }
        Some(Commands::List) => {
            if config.connections.is_empty() {
                println!("No connections saved. Use 'rssh add <alias> <user@host>' to add one.");
            } else {
                println!("Saved connections:");
                for (alias, conn) in &config.connections {
                    println!("  {} -> {}", alias, conn);
                }
            }
        }
        Some(Commands::Remove { alias }) => {
            if config.connections.remove(&alias).is_some() {
                config.save()?;
                delete_password(&alias)?;
                println!("Connection '{}' removed.", alias);
            } else {
                return Err(anyhow!("Alias '{}' not found.", alias));
            }
        }
        Some(Commands::Connect {
            alias,
            port,
            identity,
        }) => {
            let sess = ssh::create_session(&config, &alias, port, identity.as_deref())?;
            ssh::handle_interactive_shell(sess)?;
        }
        Some(Commands::Upload {
            alias,
            local_path,
            remote_path,
            port,
            identity,
        }) => {
            let sess = ssh::create_session(&config, &alias, port, identity.as_deref())?;
            ssh::handle_upload(sess, &local_path, &remote_path)?;
        }
        Some(Commands::Download {
            alias,
            remote_path,
            local_path,
            port,
            identity,
        }) => {
            let sess = ssh::create_session(&config, &alias, port, identity.as_deref())?;
            ssh::handle_download(sess, &remote_path, &local_path)?;
        }
        None => {
            // Interactive mode
            let aliases: Vec<String> = config.connections.keys().cloned().collect();
            if aliases.is_empty() {
                println!("No connections saved. Use 'add' command first.");
                return Ok(());
            }
            let choice = Select::new("Select a connection to open:", aliases).prompt()?;
            let port_str = Text::new("Enter port:").with_default("22").prompt()?;
            let port = port_str.parse::<u16>().context("Invalid port number")?;

            let use_identity = Confirm::new("Use identity file (private key)?")
                .with_default(false)
                .prompt()?;
            let identity_path = if use_identity {
                let path_str = Text::new("Enter path to private key:").prompt()?;
                Some(PathBuf::from(path_str))
            } else {
                None
            };

            let sess = ssh::create_session(&config, &choice, port, identity_path.as_deref())?;
            ssh::handle_interactive_shell(sess)?;
        }
    }

    Ok(())
}