{
  description = "Grove development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);
    in
    {
      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              bash
              cargo
              clippy
              coreutils
              findutils
              git
              gnumake
              gnugrep
              gnused
              jq
              lefthook
              lazygit
              openssl
              pkg-config
              ripgrep
              rustc
              rust-analyzer
              rustfmt
              tmux
              zellij
            ];
          };
        });
    };
}
