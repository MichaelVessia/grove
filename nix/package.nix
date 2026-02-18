{ inputs, ... }:
{
  flake = {
    overlays.default = final: prev: {
      grove = inputs.self.packages.${final.stdenv.hostPlatform.system}.default;
    };
  };

  perSystem = { config, self', inputs', pkgs, system, lib, ... }:
    let
      rustToolchain = pkgs.rust-bin.stable.latest.default;
      craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;

      src = lib.cleanSourceWith {
        src = inputs.self;
        filter = path: type:
          (craneLib.filterCargoSources path type);
      };

      commonArgs = {
        inherit src;
        strictDeps = true;
        pname = "grove";

        meta = with lib; {
          description = "A minimal workspace manager for AI coding agents";
          homepage = "https://github.com/jordangarrison/grove";
          license = licenses.mit;
          mainProgram = "grove";
        };

        nativeBuildInputs = with pkgs; [
          pkg-config
        ];

        buildInputs = with pkgs; [
          openssl
        ];
      };

      cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
        pname = "grove-deps";
      });

      grove = craneLib.buildPackage (commonArgs // {
        inherit cargoArtifacts;
        doCheck = false;
      });

      wrappedGrove = {
        withTmux ? true,
        withGit ? true,
        withLazygit ? true,
      }:
        let
          runtimeDeps = lib.flatten [
            (lib.optional withTmux pkgs.tmux)
            (lib.optional withGit pkgs.git)
            (lib.optional withLazygit pkgs.lazygit)
          ];
        in
        if runtimeDeps == [] then grove
        else pkgs.symlinkJoin {
          name = "grove-${grove.version or "0.1.0"}";
          paths = [ grove ];
          nativeBuildInputs = [ pkgs.makeWrapper ];
          postBuild = ''
            wrapProgram $out/bin/grove \
              --prefix PATH : ${lib.makeBinPath runtimeDeps}
          '';
        };

      defaultGrove = lib.makeOverridable wrappedGrove {};
    in
    {
      packages = {
        default = defaultGrove;
        grove = defaultGrove;
        grove-unwrapped = grove;
      };

      apps.default = {
        type = "app";
        program = "${defaultGrove}/bin/grove";
      };
    };
}
