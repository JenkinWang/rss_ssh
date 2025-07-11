# rssh - Secure SSH Login Management Tool

`rssh` is a command-line tool built with Rust that simplifies managing and connecting to SSH servers. It securely stores your connection details and passwords/keys, and provides a simple interface for adding, listing, removing, and connecting to your servers. It also supports file transfers (upload/download) with a progress bar.

## ✨ Features

- **Add, List, Remove Connections**: Easily manage your SSH connection aliases.
- **Secure Password Storage**: Uses the system's native keychain (`keyring`) to securely store passwords.
- **Interactive & Non-Interactive Modes**: Connect via a simple command or select from a list of saved connections.
- **Password & Identity File Authentication**: Supports both password-based and public key-based authentication.
- **File Transfer**: Upload and download files securely over SFTP with a visual progress bar.

## 📦 Installation

1.  Ensure you have Rust and Cargo installed.
2.  Clone this repository:
    ```bash
    git clone https://github.com/JenkinWang/rss-ssh.git
    cd rss_ssh
    ```
3.  Build and install the binary:
    ```bash
    cargo install --path .
    ```
4.  Verify the installation by running:
    ```bash
    rssh --version
    ```

## 🚀 Usage

### Connection Management

-   **Add a new connection:**
    ```bash
    rssh add <alias> <user@host>
    ```
    *Example:* `rssh add webserver user@example.com`

-   **List all saved connections:**
    ```bash
    rssh list
    ```

-   **Remove a connection:**
    ```bash
    rssh remove <alias>
    ```
    *Example:* `rssh remove webserver`

### Connecting to a Server

-   **Connect using an alias:**
    ```bash
    rssh connect <alias> [--port <port>] [--identity /path/to/key]
    ```
    *Example (password):* `rssh connect webserver`
    *Example (identity file):* `rssh connect webserver --identity ~/.ssh/id_rsa`

-   **Interactive Mode (if no command is provided):**
    ```bash
    rssh
    ```
    This will present a list of saved connections to choose from.

### File Transfer

-   **Upload a file to a remote directory:**
    ```bash
    rssh upload <alias> <local-file-path> <remote-directory-path>
    ```
    The command will upload the specified local file into the remote directory, keeping the original filename.

    *Example:*
    ```bash
    # Uploads 'backup.zip' from the current directory to '/home/user/backups/' on the server
    rssh upload webserver ./backup.zip /home/user/backups/
    ```

-   **Download a file to a local directory:**
    ```bash
    rssh download <alias> <remote-file-path> <local-directory-path>
    ```
    The command will download the specified remote file into the local directory, keeping the original filename.

    *Example:*
    ```bash
    # Downloads '/var/log/app.log' from the server to the './logs' directory on your machine
    rssh download webserver /var/log/app.log ./logs
    ```

## 📝 License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
