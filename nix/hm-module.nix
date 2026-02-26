{ config, lib, pkgs, ... }:

let
  cfg = config.programs.grove;
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
  };

  config = lib.mkIf cfg.enable (lib.mkMerge [
    {
      home.packages = [ cfg.package ];
    }
    (lib.mkIf (cfg.environment != { }) {
      home.sessionVariables = cfg.environment;
    })
  ]);
}
