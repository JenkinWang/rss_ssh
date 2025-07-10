mod config;
mod credentials;

use crate::config::Config;
use crate::credentials::{delete_password, get_password, set_password};
use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use crossterm::terminal;
use inquire::{Confirm, Password, Select, Text};
use ssh2::Session;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "rssh", version = "1.0", about = "A secure SSH login management tool")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
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
}

fn main() -> anyhow::Result<()> {
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
            handle_connect(&config, &alias, port, identity.as_deref())?;
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

            let use_identity =
                Confirm::new("Use identity file (private key)?")
                    .with_default(false)
                    .prompt()?;
            let identity_path = if use_identity {
                let path_str = Text::new("Enter path to private key:").prompt()?;
                Some(PathBuf::from(path_str))
            } else {
                None
            };

            handle_connect(&config, &choice, port, identity_path.as_deref())?;
        }
    }

    Ok(())
}

fn handle_connect(
    config: &Config,
    alias: &str,
    port: u16,
    identity_path: Option<&Path>,
) -> Result<()> {
    let conn_str = config
        .connections
        .get(alias)
        .context(format!("Alias '{}' not found.", alias))?;

    let parts: Vec<&str> = conn_str.split('@').collect();
    if parts.len() != 2 {
        return Err(anyhow!(
            "Invalid connection string format. Use 'user@host'."
        ));
    }
    let user = parts[0];
    let host = parts[1];

    let tcp = TcpStream::connect(format!("{}:{}", host, port))
        .context(format!("Failed to connect to {}:{}", host, port))?;
    let mut sess = Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;

    if let Some(private_key_path) = identity_path {
        let attempts = 0;
        loop {
            let auth_result =
                sess.userauth_pubkey_file(user, None, private_key_path, None);

            match auth_result {
                Ok(_) => break,
                Err(e) => {
                    if e.to_string().contains("passphrase") && attempts < 1 {
                        let passphrase = Password::new("Enter passphrase for key:")
                            .with_display_mode(inquire::PasswordDisplayMode::Masked)
                            .prompt()?;
                        if sess
                            .userauth_pubkey_file(user, None, private_key_path, Some(&passphrase))
                            .is_ok()
                        {
                            break;
                        }
                    }
                    return Err(anyhow!("Authentication failed with key: {}", e));
                }
            }
        }
    } else {
        let password = match get_password(alias) {
            Ok(pass) => pass,
            Err(_) => {
                let pass = Password::new("Enter password:")
                    .with_display_mode(inquire::PasswordDisplayMode::Masked)
                    .prompt()?;
                set_password(alias, &pass)?;
                pass
            }
        };
        sess.userauth_password(user, &password)
            .context("Authentication failed. Please check your username/password.")?;
    }

    println!("Successfully connected to {}!", conn_str);

    let mut channel = sess.channel_session()?;
    let (width, height) = terminal::size()?;
    channel.request_pty(
        "xterm-256color",
        None,
        Some((width as u32, height as u32, 0, 0)),
    )?;
    channel.shell()?;

    terminal::enable_raw_mode()?;
    sess.set_blocking(false);

    let mut stdout = io::stdout();
    let mut channel_buf = [0; 1024];

    'main_loop: loop {
        if crossterm::event::poll(std::time::Duration::from_millis(10))? {
            if let Ok(event) = crossterm::event::read() {
                match event {
                    crossterm::event::Event::Key(key_event) => {
                        if key_event.kind != crossterm::event::KeyEventKind::Press {
                            continue;
                        }
                        let mut key_bytes = Vec::new();
                        match key_event.code {
                            crossterm::event::KeyCode::Char(c) => {
                                if key_event
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    if ('a'..='z').contains(&c) {
                                        key_bytes.push((c as u8) - b'a' + 1);
                                    }
                                } else {
                                    key_bytes.push(c as u8);
                                }
                            }
                            crossterm::event::KeyCode::Enter => key_bytes.push(b'\r'),
                            crossterm::event::KeyCode::Backspace => key_bytes.push(8),
                            crossterm::event::KeyCode::Left => {
                                key_bytes.extend_from_slice(b"\x1b[D")
                            }
                            crossterm::event::KeyCode::Right => {
                                key_bytes.extend_from_slice(b"\x1b[C")
                            }
                            crossterm::event::KeyCode::Up => key_bytes.extend_from_slice(b"\x1b[A"),
                            crossterm::event::KeyCode::Down => {
                                key_bytes.extend_from_slice(b"\x1b[B")
                            }
                            crossterm::event::KeyCode::Tab => key_bytes.push(b'\t'),
                            crossterm::event::KeyCode::Esc => key_bytes.push(0x1b),
                            _ => {}
                        }
                        if !key_bytes.is_empty() {
                            channel.write_all(&key_bytes)?;
                            channel.flush()?;
                        }
                    }
                    crossterm::event::Event::Resize(width, height) => {
                        channel.request_pty_size(width as u32, height as u32, None, None)?;
                    }
                    _ => {}
                }
            }
        }

        loop {
            match channel.read(&mut channel_buf) {
                Ok(0) => break 'main_loop,
                Ok(n) => {
                    stdout.write_all(&channel_buf[..n])?;
                    stdout.flush()?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    eprintln!("Channel read error: {}", e);
                    break 'main_loop;
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
    Ok(())
}