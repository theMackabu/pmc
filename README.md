# Process Management Controller (PMC)

## Overview

PMC (Process Management Controller) is a simple PM2 alternative written in Rust. It provides a command-line interface to start, stop, restart, and manage fork processes

## Features

- Start, stop, and restart processes.
- List all running processes with customizable output formats.
- Retrieve detailed information about a specific process.

## Usage

```bash
# Start/Restart a process
pmc start <id> or <script> [--name <name>]

# Stop/Kill a process
pmc stop <id>

# Remove a process
pmc remove <id>

# Get process info
pmc info <id>

# List all processes
pmc list [--format <raw|json|default>]

# Get process logs
pmc logs <id> [--lines <num_lines>]
```

For more commands, check out `pmc --help`

### Installation

Pre-built binaries for Linux, MacOS, and Windows can be found on the [releases](releases) page.
Install from crates.io using `cargo install pmc`

#### Building

- Clone the project
- Open a terminal in the project folder
- Check if you have cargo (Rust's package manager) installed, just type in `cargo`
- If cargo is installed, run `cargo build --release`
- Put the executable into one of your PATH entries
  - Linux: usually /bin/ or /usr/bin/
  - Windows: C:\Windows\System32 is good for it but don't use windows
