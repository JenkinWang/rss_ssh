use crate::config::Config;
use crate::credentials::{get_password, set_password};
use anyhow::{anyhow, Context, Result};
use crossterm::terminal;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{Confirm, Password};
use ssh2::Session;
use std::fs;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::Path;

pub fn create_session(
    config: &Config,
    alias: &str,
    port: u16,
    identity_path: Option<&Path>,
) -> Result<Session> {
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

    println!("Connecting to {}@{}:{}", user, host, port);

    let tcp = TcpStream::connect(format!("{}:{}", host, port))
        .context(format!("Failed to connect to {}:{}", host, port))?;
    let mut sess = Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;

    if let Some(private_key_path) = identity_path {
        let mut attempts = 0;
        loop {
            let auth_result = sess.userauth_pubkey_file(user, None, private_key_path, None);

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
                        attempts += 1;
                    } else {
                        return Err(anyhow!("Authentication failed with key: {}", e));
                    }
                }
            }
        }
    } else {
        let password = match get_password(alias) {
            Ok(pass) => pass,
            Err(_) => {
                let pass = Password::new(&format!("Enter password for {}:", conn_str))
                    .with_display_mode(inquire::PasswordDisplayMode::Masked)
                    .prompt()?;
                if Confirm::new("Save password to keychain?")
                    .with_default(true)
                    .prompt()? {
                    set_password(alias, &pass)?;
                }
                pass
            }
        };
        sess.userauth_password(user, &password)
            .context("Authentication failed. Please check your username/password.")?;
    }

    println!("Successfully connected!");
    Ok(sess)
}

pub fn handle_interactive_shell(sess: Session) -> Result<()> {
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

pub fn handle_upload(sess: Session, local_path: &Path, remote_dir: &Path) -> Result<()> {
    if !local_path.is_file() {
        return Err(anyhow!(
            "Local path {:?} is not a file. Please provide a path to a file to upload.",
            local_path
        ));
    }

    let file_name = local_path.file_name().unwrap(); // Safe due to is_file check
    let remote_path = remote_dir.join(file_name);

    let mut local_file = fs::File::open(local_path)
        .context(format!("Failed to open local file: {:?}", local_path))?;
    let file_size = local_file.metadata()?.len();

    println!("Uploading {:?} to {:?}...", local_path, remote_path);

    let pb = ProgressBar::new(file_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})")
        .unwrap()
        .progress_chars("#>-"));

    let sftp = sess.sftp().context("Failed to create SFTP session")?;
    let mut remote_file = sftp.create(&remote_path)
        .context(format!("Failed to create remote file: {:?}", remote_path))?;

    let mut reader = pb.wrap_read(&mut local_file);
    io::copy(&mut reader, &mut remote_file)?;

    pb.finish_with_message("Upload complete");
    Ok(())
}

pub fn handle_download(sess: Session, remote_path: &Path, local_dir: &Path) -> Result<()> {
    let file_name = remote_path.file_name().ok_or_else(|| {
        anyhow!(
            "Remote path {:?} is a directory or invalid. Please provide a path to a file to download.",
            remote_path
        )
    })?;

    if local_dir.is_file() {
        return Err(anyhow!(
            "Local destination {:?} is a file. Please provide a directory path.",
            local_dir
        ));
    }
    fs::create_dir_all(local_dir)
        .context(format!("Failed to create local directory {:?}", local_dir))?;

    let local_path = local_dir.join(file_name);

    println!("Downloading {:?} to {:?}...", remote_path, local_path);

    let sftp = sess.sftp().context("Failed to create SFTP session")?;
    let mut remote_file = sftp.open(remote_path)
        .context(format!("Failed to open remote file: {:?}", remote_path))?;
    
    let stat = remote_file.stat()?;
    let file_size = stat.size.unwrap_or(0);

    let pb = ProgressBar::new(file_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})")
        .unwrap()
        .progress_chars("#>-"));

    let mut local_file = fs::File::create(&local_path)
        .context(format!("Failed to create local file: {:?}", local_path))?;

    let mut reader = pb.wrap_read(&mut remote_file);
    io::copy(&mut reader, &mut local_file)?;

    pb.finish_with_message("Download complete");
    Ok(())
}