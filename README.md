# Process Management Controller (PMC)

## Overview

PMC (Process Management Controller) is a simple PM2 alternative written in Rust. It provides a command-line/api interface to start, stop, restart, and manage fork processes

## Features

- Start, stop, and restart processes.
- List all running processes with customizable output formats.
- Retrieve detailed information about a specific process.
- Use HTTP/rust api to control processes.

## Usage

```bash
# Start/Restart a process
pmc start <id/name> or <script> [--name <name>]

# Stop/Kill a process
pmc stop <id/name>

# Remove a process
pmc remove <id/name>

# Get process info
pmc info <id/name>

# Get process env
pmc env <id/name>

# Save all processes to dumpfile
pmc save

# Restore all processes
pmc restore

# List all processes
pmc list [--format <raw|json|default>]

# Get process logs
pmc logs <id/name> [--lines <num_lines>]

# Flush process logs
pmc flush <id/name>

# Reset process index
pmc daemon reset

# Stop daemon
pmc daemon stop

# Start/Restart daemon
pmc daemon start

# Check daemon health
pmc daemon health

# Add new Ssrver
pmc server new

# List servers
pmc server list [--format <format>]

# Remove server
pmc server remove <name>

# Set default server
pmc server default [<name>]
```

For more command information, check out `pmc --help`

### Installation way
#### 1. Run bash script
```
curl -fsSL https://raw.githubusercontent.com/theMackabu/pmc/master/scripts/install.sh | sh
```

#### 2. Pre-built binary download
Pre-built binaries for Linux, MacOS, and WSL can be found on the [releases](releases) page.
There is no windows support yet. Install from crates.io using `cargo install pmc` (requires clang++)

#### Building

- Clone the project
- Open a terminal in the project folder
- Check if you have cargo (Rust's package manager) installed, just type in `cargo`
- If cargo is installed, run `cargo build --release`
- Put the executable into one of your PATH entries, usually `/bin/` or `/usr/bin/`
