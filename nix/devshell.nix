{ inputs, ... }:
{
  perSystem = { config, self', inputs', pkgs, system, ... }:
    let
      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" "rust-analyzer" ];
      };
    in
    {
      _module.args.pkgs = import inputs.nixpkgs {
        inherit system;
        overlays = [ inputs.rust-overlay.overlays.default ];
      };

      devShells.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          # Rust toolchain (replaces bare rustc/cargo/clippy/rustfmt/rust-analyzer)
          rustToolchain
          pkg-config
          openssl

          # System tools
          bash
          coreutils
          findutils
          git
          gnumake
          gnugrep
          gnused
          jq
          ripgrep
          tmux

          # Dev tools
          cargo-watch
          lefthook
          lazygit
        ];

        RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

        shellHook = ''
          if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
            git config --local core.hooksPath .githooks
          fi
        '';
      };
    };
}
