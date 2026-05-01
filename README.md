# nix-tree-rs

A Rust port of [nix-tree](https://github.com/utdemir/nix-tree), providing an interactive visualization of Nix store dependencies.

## Overview

`nix-tree-rs` is a terminal user interface (TUI) that allows you to interactively browse and analyze the dependency tree of Nix store paths. It helps you understand why packages are in your Nix store and how much space they consume.

## Features

- **Interactive Navigation**: Three-pane interface showing referrers, current selection, and dependencies
- **Size Analysis**: View NAR size, closure size, and added size for each package
- **Search**: Find packages by name within the dependency tree
- **Why-Depends**: Discover all paths from GC roots to a specific package
- **Multiple Sort Orders**: Sort by name, closure size, or added size
- **Signature Verification**: See which packages are signed
- **Vim-like Keybindings**: Familiar navigation for vim users

## Why a Rust port?

The original [nix-tree] is great, but on large closures its startup is dominated
by computing closure sizes over string-keyed maps. `nix-tree-rs` builds a dense
integer-indexed graph and walks it with reusable buffers, so it stays responsive
on system-sized closures.

Measured on a 12 253-path NixOS system derivation closure (Apple M-series,
warm Nix store):

|                          | nix-tree 0.8.0 | nix-tree-rs |
| ------------------------ | -------------- | ----------- |
| load + compute all sizes | 2 m 44 s¹     | **1.6 s**   |
| runtime closure          | 99 MiB         | 45 MiB      |

¹ `nix-tree --dot`, which exercises the same load path as the TUI.

Feature parity is close but not complete; see [differences](#differences-from-nix-tree).

[nix-tree]: https://github.com/utdemir/nix-tree

## Installation

```bash
nix run github:Mic92/nix-tree-rs
```

## Usage

### Basic Usage

```bash
# Analyze current system profile (auto-detects from /run/current-system or ~/.nix-profile)
nix-tree

# Analyze specific store paths
nix-tree /nix/store/abc123-package-1.0 /nix/store/def456-package-2.0

# Analyze a derivation and its dependencies
nix-tree -d /nix/store/...firefox.drv

# Use with nix flakes
nix-tree nixpkgs#hello
```

### Keybindings

#### Navigation
- `j`/`↓` - Move down
- `k`/`↑` - Move up  
- `h`/`←` - Move to previous pane (go back)
- `l`/`→` - Move to next pane (explore dependencies)
- `Enter` - Select item
- `Page Up`/`Page Down` - Scroll quickly

#### Actions
- `/` - Search for packages
- `w` - Show why-depends (displays all paths from roots to selected package)
  - In why-depends view: use `h`/`l` to scroll horizontally
- `s` - Change sort order (cycles: closure size → added size → alphabetical)
- `?` - Toggle help
- `q`/`Esc` - Quit or close modal

### Understanding the Display

The interface shows three panes:
- **Left pane (Referrers)**: Packages that depend on the selected item
- **Middle pane (Current)**: The currently focused level of the tree
- **Right pane (Dependencies)**: Packages that the selected item depends on

For each package:
- `✓` indicates the package is signed
- Package name is shown with size in parentheses
- The status bar shows detailed information about the selected package

**Size Terminology**:
- **NAR Size**: The size of the package itself
- **Closure Size**: Total size including all dependencies
- **Added Size**: Additional space this package adds (excluding shared dependencies)

## Differences from nix-tree

Not yet supported:

- `--dot` graphviz output
- `--impure` flag forwarding
- `y` to yank the selected store path to the clipboard

Additions:

- `--option NAME VALUE` to forward arbitrary Nix options
- `g`/`G`/`PgUp`/`PgDn` navigation in the main list
- per-pane position counter and visible sort order

## Building from Source

```bash
# Clone the repository
git clone https://github.com/joerg/nix-tree-rs
cd nix-tree-rs

# Build with Cargo
cargo build --release

# Or use Nix
nix build
```

## Development

```bash
# Enter development shell
nix develop

# Run tests  
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

This is a Rust port of the original [nix-tree](https://github.com/utdemir/nix-tree) by Utku Demir.
