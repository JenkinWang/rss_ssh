# rssh - A Secure SSH Login Management Tool

`rssh` is a command-line tool written in Rust to help you manage and connect to your SSH servers securely and efficiently. It supports connection aliasing, secure password storage in the system keychain, and authentication via both passwords and private keys.

## ✨ Features

- **Connection Aliasing**: Save your `user@host` connections with easy-to-remember aliases.
- **Secure Password Storage**: Automatically saves and retrieves passwords from the system's native keychain/credential store.
- **Multiple Authentication Methods**:
  - Connect using a securely stored password.
  - Connect using a private key file (`-i` flag), similar to `ssh -i`.
- **Interactive Mode**: An easy-to-use interactive menu to select and connect to a saved server if you run `rssh` without arguments.
- **Custom Port**: Specify a custom port for your SSH connection using the `-p` flag.
- **Cross-Platform**: Built with Rust, works on macOS, Linux, and Windows.

## 📦 Installation

1.  Ensure you have Rust and Cargo installed on your system. If not, get them from [rust-lang.org](https://www.rust-lang.org/).
2.  Clone this repository:
    ```bash
    git clone https://github.com/JenkinWang/rss_ssh.git
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

`rssh` provides several subcommands to manage your connections.

### 1. `add` - Add a New Connection

Save a new SSH server with a unique alias.

**Command:**
```bash
rssh add <ALIAS> <USER@HOST>
```

**Example:**
```bash
rssh add dev-server user@192.168.1.100
```

### 2. `list` - List Saved Connections

View all your saved connection aliases.

**Command:**
```bash
rssh list
```

**Example Output:**
```
Saved connections:
  dev-server -> user@192.168.1.100
  prod-server -> admin@my-vps.com
```

### 3. `remove` - Remove a Connection

Delete a saved connection alias and its corresponding password from the keychain.

**Command:**
```bash
rssh remove <ALIAS>
```

**Example:**
```bash
rssh remove dev-server
```

### 4. `connect` - Connect to a Server

Connect to a server using its alias. This is the most powerful command with several options.

**Command:**
```bash
rssh connect <ALIAS> [OPTIONS]
```

**Options:**
- `-p, --port <PORT>`: Specify a custom port (defaults to 22).
- `-i, --identity <PATH>`: Provide the path to a private key file for authentication.

**Connection Examples:**

- **Connect using a password** (it will be prompted on first use and saved securely for later):
  ```bash
  rssh connect dev-server
  ```

- **Connect to a server on a custom port**:
  ```bash
  rssh connect dev-server -p 2222
  ```

- **Connect using a private key**:
  ```bash
  rssh connect dev-server -i ~/.ssh/id_rsa
  ```

- **Connect using a private key on a custom port**:
  ```bash
  rssh connect dev-server -p 2222 -i ~/.ssh/id_rsa_custom
  ```

### 5. Interactive Mode

If you run `rssh` with no command, it enters a user-friendly interactive mode.

**Command:**
```bash
rssh
```

**Steps:**
1.  It will first display a list of your saved connections to choose from.
2.  Next, it will ask for the port number, with `22` as the default.
3.  Then, it will ask if you want to use an identity file (private key).
4.  If you choose yes, it will prompt for the path to your key file.

This mode guides you through the connection process step-by-step, making it easy to connect without remembering all the flags.
