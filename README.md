# nix-tree-rs

A Rust implementation of [nix-tree](https://github.com/utdemir/nix-tree), an interactive Nix dependency tree viewer.

## Features

- 🌳 Interactive three-pane navigation (referrers, current, dependencies)
- 📊 Multiple sorting options (alphabetical, closure size, added size)
- 🔍 Real-time search functionality
- 📏 Size calculations (NAR size, closure size, added size)
- ✓ Signature verification display
- ⌨️ Vim-like keybindings

## Installation

```bash
nix run github:joerg/nix-tree-rs
```

## Usage

```bash
# View dependencies of current system
nix-tree

# View specific store path
nix-tree /nix/store/...

# View derivation dependencies
nix-tree -d /nix/store/...drv
```

## Keybindings

- `j`/`↓` - Move down
- `k`/`↑` - Move up  
- `h`/`←` - Move to previous pane
- `l`/`→` - Move to next pane
- `/` - Search
- `s` - Change sort order
- `?` - Show help
- `q`/`Esc` - Quit

## Architecture

This implementation follows the architecture of [nix-melt](https://github.com/nix-community/nix-melt) with:

- Clean module separation
- Strong type safety with custom error types
- Async operations with tokio
- TUI built with ratatui and crossterm

## Development

```bash
# Enter development shell
nix develop

# Build
cargo build

# Run tests  
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy
```