# Tiny synthetic dependency graph for integration tests.
#
#   v1: app-1.0 ──▶ liba-1.0 ──▶ libc-1.0
#                └▶ libb-1.0 ──▶ libc-1.0   (shared)
#
#   v2: app-2.0 ──▶ liba-1.0 ──▶ libc-1.0
#                └▶ libd-1.0                (libb dropped, libd added)
#
# Kept self-contained (no nixpkgs) so tests are hermetic and fast.
let
  drv = name: deps:
    derivation {
      inherit name;
      system = builtins.currentSystem;
      builder = "/bin/sh";
      args = [
        "-c"
        ''
          {
            echo ${name}
            ${builtins.concatStringsSep "\n" (map (d: "echo ${d}") deps)}
          } > $out
        ''
      ];
    };

  libc = drv "ntfx-libc-1.0" [ ];
  liba = drv "ntfx-liba-1.0" [ libc ];
  libb = drv "ntfx-libb-1.0" [ libc ];
  libd = drv "ntfx-libd-1.0" [ ];
in
{
  v1 = drv "ntfx-app-1.0" [ liba libb ];
  v2 = drv "ntfx-app-2.0" [ liba libd ];
}
