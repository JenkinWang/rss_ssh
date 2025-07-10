mod config;
mod credentials;

use crate::config::Config;
use crate::credentials::{get_password, set_password, delete_password};
use anyhow::{anyhow, Context, Result};
use std::net::TcpStream;
use ssh2::Session;
use std::io::{Read, Write, self};
use inquire::Select;
use crossterm::terminal;

use clap::{Parser, Subcommand};

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
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load()?;

    match cli.command {
        Some(Commands::Add { alias, connection_string }) => {
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
                delete_password(&alias)?; // 同时删除存储的密码
                println!("Connection '{}' removed.", alias);
            } else {
                return Err(anyhow!("Alias '{}' not found.", alias));
            }
        }
        Some(Commands::Connect { alias }) => {
            handle_connect(&config, &alias)?;
        }
        None => {
            // 交互式模式
            let aliases: Vec<String> = config.connections.keys().cloned().collect();
            if aliases.is_empty() {
                println!("No connections saved. Use 'add' command first.");
                return Ok(());
            }
            let choice = Select::new("Select a connection to open:", aliases).prompt()?;
            handle_connect(&config, &choice)?;
        }
    }

    Ok(())
}

// 连接处理函数
fn handle_connect(config: &Config, alias: &str) -> Result<()> {
    let conn_str = config.connections.get(alias)
        .context(format!("Alias '{}' not found.", alias))?;
    
    let parts: Vec<&str> = conn_str.split('@').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid connection string format. Use 'user@host'."));
    }
    let user = parts[0];
    let host = parts[1];

    // 尝试从 keychain 获取密码
    let password = match get_password(alias) {
        Ok(pass) => pass,
        Err(_) => {
            // 获取失败，提示用户输入并保存
            let pass = inquire::Password::new("Enter password:")
                .with_display_mode(inquire::PasswordDisplayMode::Masked)
                .prompt()?;
            set_password(alias, &pass)?;
            pass
        }
    };
    
    // 建立 SSH 连接
    let tcp = TcpStream::connect(format!("{}:22", host))
        .context(format!("Failed to connect to {}", host))?;
    let mut sess = Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;
    
    sess.userauth_password(user, &password)
        .context("Authentication failed. Please check your username/password.")?;

    println!("Successfully connected to {}!", conn_str);

    let mut channel = sess.channel_session()?;
    let (width, height) = terminal::size()?;
    channel.request_pty(
        "xterm-256color",
        None,
        Some((width as u32, height as u32, 0, 0)),
    )?;
    channel.shell()?;

    // 进入 raw 模式
    terminal::enable_raw_mode()?;
    
    // 非阻塞
    sess.set_blocking(false);

    let mut stdout = io::stdout();
    let mut channel_buf = [0; 1024];

    'main_loop: loop {
        // 从 stdin 读取并发送到 channel
        // 使用 crossterm::event::poll 来非阻塞地检查输入
        if crossterm::event::poll(std::time::Duration::from_millis(10))? {
            if let Ok(event) = crossterm::event::read() {
                match event {
                    crossterm::event::Event::Key(key_event) => {
                        if key_event.kind != crossterm::event::KeyEventKind::Press {
                            continue;
                        }
                        // 将 crossterm KeyCode 转换为字节序列
                        let mut key_bytes = Vec::new();
                        match key_event.code {
                            crossterm::event::KeyCode::Char(c) => {
                                if key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                                    // Handle Ctrl+C, Ctrl+D, etc.
                                    if ('a'..='z').contains(&c) {
                                        key_bytes.push((c as u8) - b'a' + 1);
                                    }
                                } else {
                                    key_bytes.push(c as u8);
                                }
                            }
                            crossterm::event::KeyCode::Enter => key_bytes.push(b'\r'),
                            crossterm::event::KeyCode::Backspace => key_bytes.push(8), // Backspace
                            crossterm::event::KeyCode::Left => key_bytes.extend_from_slice(b"\x1b[D"),
                            crossterm::event::KeyCode::Right => key_bytes.extend_from_slice(b"\x1b[C"),
                            crossterm::event::KeyCode::Up => key_bytes.extend_from_slice(b"\x1b[A"),
                            crossterm::event::KeyCode::Down => key_bytes.extend_from_slice(b"\x1b[B"),
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
                    _ => {} // Ignore other events
                }
            }
        }


        // 从 channel 读取并打印到 stdout
        loop {
            match channel.read(&mut channel_buf) {
                Ok(0) => {
                    // EOF
                    break 'main_loop;
                }
                Ok(n) => {
                    stdout.write_all(&channel_buf[..n])?;
                    stdout.flush()?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // 没有更多数据了，跳出内层循环
                    break;
                }
                Err(e) => {
                    // 发生错误
                    eprintln!("Channel read error: {}", e);
                    break 'main_loop;
                }
            }
        }
    }

    // 恢复终端
    terminal::disable_raw_mode()?;
    Ok(())
}
