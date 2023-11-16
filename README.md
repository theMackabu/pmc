# Process Management Controller (PMC)

## Overview

PMC (Process Management Controller) is a simple process management tool written in Rust. It provides a command-line interface to start, stop, restart, and manage processes. PMC is designed to be extensible, allowing users to define and manage their processes efficiently.

## Features

- Start, stop, and restart processes.
- List all running processes with customizable output formats.
- Retrieve detailed information about a specific process.

## Installation

`cargo install pmc`

## Usage

### Start/Restart a Process

```bash
pmc start <id> or <script> [--name <name>]
```

### Stop/Kill a Process

```bash
pmc stop <id>
```

### Remove a Process

```bash
pmc remove <id>
```

### Get Information of a Process

```bash
pmc info <id>
```

### List All Processes

```bash
pmc list [--format <raw|json|default>]
```

### Get Logs from a Process

```bash
pmc logs <id> [--lines <num_lines>]
```

## Building from Source

If you want to build PMC from source, make sure you have Rust and Cargo installed on your system. Clone the repository, navigate to the project directory, and use the following commands:

```bash
cargo build --release
```

The compiled binary will be available in the `target/release` directory.
