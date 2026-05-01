# nix-tree-rs

Interactively browse dependency graphs of Nix derivations. A Rust port of
[nix-tree].

## Why a port?

On large closures the original spends most of its startup computing closure
sizes. This implementation indexes the graph by integer id and walks it with
reusable buffers.

12 253-path NixOS system derivation, Apple M-series, warm store:

|                          | nix-tree 0.8.0 | nix-tree-rs |
| ------------------------ | -------------- | ----------- |
| load + compute all sizes | 2 m 44 s¹      | **1.6 s**   |
| runtime closure          | 99 MiB         | 45 MiB      |

¹ `nix-tree --dot`, same load path as the TUI.

## Install

```bash
nix run github:Mic92/nix-tree-rs
```

## Usage

```bash
nix-tree                              # current-system / ~/.nix-profile
nix-tree /nix/store/...-foo
nix-tree --derivation nixpkgs#hello   # build-time deps, no build needed
nix-tree --dot nixpkgs#hello | dot -Tsvg > deps.svg
nix-tree --diff /nix/var/nix/profiles/system-{41,42}-link
```

Press `?` inside the TUI for keybindings.

### `--diff`

Like `nix store diff-closures`, but sorted by size impact and with totals:

```console
$ nix-tree --diff /nix/var/nix/profiles/system-{41,42}-link
    -1.1 GiB  uutils-coreutils         (rebuilt)
   +24.2 MiB  gemini-cli               0.37.1 → 0.39.1
   +10.1 MiB  gettext                  0.26 → 1.0
    +7.6 MiB  pi-agent-deps            ∅ → 0.1.0
    -1.1 MiB  unixobcd                 2.3.12 → ∅
  +980.6 KiB  binutils                 2.44 → 2.46
  ...
722 paths → 728 paths, 9.6 GiB → 8.5 GiB (-1.0 GiB)
```

**Sizes:** *NAR* = the path itself · *closure* = path + all references ·
*added* = closure space lost if this path alone were removed from the parent.

## Hacking

```bash
nix develop -c cargo test
nix develop -c cargo bench --bench scroll -- /run/current-system   # frame timing
nix develop -c cargo run --example snapshot -- /run/current-system 'll'  # render to text
```

## License

BSD-3-Clause. Based on [nix-tree] by Utku Demir.

[nix-tree]: https://github.com/utdemir/nix-tree
