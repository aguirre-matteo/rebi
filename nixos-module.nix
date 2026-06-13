{ config, pkgs, lib, ... }:

let
  cfg = config.programs.rebi;
in {
  options.programs.rebi = {
    enable = lib.mkEnableOption "Rebi - A Keyboard and Mouse remapping tool (Frontend for keyd)";
    
    guiPackage = lib.mkOption {
      type = lib.types.package;
      description = "The Rebi GUI package to install.";
    };

    helperPackage = lib.mkOption {
      type = lib.types.package;
      description = "The Rebi helper package to install. Must be installed system-wide for Polkit to work.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ 
      cfg.guiPackage 
      cfg.helperPackage
      pkgs.keyd
    ];

    services.keyd.enable = true;
    security.polkit.enable = true;
    security.polkit.extraConfig = ''
      polkit.addRule(function(action, subject) {
          if (action.id == "org.rebi.pkexec.run-helper" &&
              subject.isInGroup("wheel")) {
              return polkit.Result.AUTH_ADMIN_KEEP;
          }
      });
    '';
  };
}
