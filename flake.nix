{
  description = "A Rust implementation of nix-tree";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.treefmt-nix.flakeModule
      ];

      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      perSystem = { config, self', inputs', pkgs, system, ... }: {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "nix-tree";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          # TODO: we currently parese the hello derivation...
          doCheck = false;

          buildInputs = with pkgs; [
            # Add any system dependencies here if needed
          ];

          meta = with pkgs.lib; {
            description = "Interactive Nix dependency tree viewer";
            homepage = "https://github.com/utdemir/nix-tree";
            license = licenses.bsd3;
            maintainers = [ ];
          };
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ config.packages.default ];

          packages = with pkgs; [
            rustc
            cargo
            clippy
            rustfmt
            rust-analyzer

            # Dev tools
            cargo-watch
            cargo-edit
          ];

          RUST_BACKTRACE = 1;
        };

        treefmt = {
          projectRootFile = "flake.nix";
          programs = {
            nixpkgs-fmt.enable = true;
            rustfmt.enable = true;
          };
        };
      };
    };
}
