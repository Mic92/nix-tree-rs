#!/usr/bin/env nix
#!nix shell nixpkgs#vhs nixpkgs#ttyd nixpkgs#ffmpeg nixpkgs#bashInteractive --command bash
# shellcheck shell=bash
# bashInteractive: vhs spawns `bash` from PATH; the stdenv bash lacks readline,
# which makes PS1's \[ \] markers leak into the recording.
set -euo pipefail

cd "$(dirname "$0")/.."

# Put a fresh nix-tree on PATH so the tape has no `nix run` delay.
nix build .#
export PATH="$PWD/result/bin:$PATH"

exec vhs docs/demo.tape
