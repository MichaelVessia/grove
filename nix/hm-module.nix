{ config, lib, pkgs, ... }:

let
  cfg = config.programs.grove;
  tomlFormat = pkgs.formats.toml { };

  configAttrs = lib.filterAttrs (_: v: v != null) {
    inherit (cfg.settings) sidebar-width-pct theme launch-skip-permissions;
  };

  hasSettings = configAttrs != { };
in
{
  options.programs.grove = {
    enable = lib.mkEnableOption "grove, a minimal workspace manager for AI coding agents";

    package = lib.mkPackageOption pkgs "grove" { };

    environment = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = { };
      example = lib.literalExpression ''
        {
          GROVE_CLAUDE_CMD = lib.getExe pkgs.claude-code;
          GROVE_LAZYGIT_CMD = lib.getExe pkgs.lazygit;
        }
      '';
      description = ''
        Environment variables to set for Grove.
        Grove supports the following overrides:
        - GROVE_CLAUDE_CMD
        - GROVE_CODEX_CMD
        - GROVE_OPENCODE_CMD
        - GROVE_LAZYGIT_CMD
      '';
    };

    settings = {
      sidebar-width-pct = lib.mkOption {
        type = lib.types.nullOr (lib.types.ints.between 10 90);
        default = null;
        description = "Sidebar width as a percentage (10-90). Grove default: 33.";
      };

      theme = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "Color theme name. Grove default: catppuccin-mocha.";
      };

      launch-skip-permissions = lib.mkOption {
        type = lib.types.nullOr lib.types.bool;
        default = null;
        description = "Skip permission prompts on workspace launch. Grove default: false.";
      };
    };
  };

  config = lib.mkIf cfg.enable (lib.mkMerge [
    {
      home.packages = [ cfg.package ];
    }
    (lib.mkIf (cfg.environment != { }) {
      home.sessionVariables = cfg.environment;
    })
    (lib.mkIf hasSettings {
      xdg.configFile."grove/config.toml".source =
        tomlFormat.generate "grove-config" configAttrs;
    })
  ]);
}
