{
  description = "Rebi - A Keyboard and Mouse remapping tool";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default-linux";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, systems, crane, rust-overlay }: let
    eachSystem = nixpkgs.lib.genAttrs (import systems);
  in {
    nixosModules.default = { pkgs, lib, ... }: {
      imports = [ ./nixos-module.nix ];
      programs.rebi.guiPackage = lib.mkDefault self.packages.${pkgs.system}.rebi-gui;
      programs.rebi.helperPackage = lib.mkDefault self.packages.${pkgs.system}.rebi-helper;
    };

    packages = eachSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };

      rustToolchain = pkgs.rust-bin.stable.latest.default;
      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
      # Filter source to include only what's needed for the build
      # We include 'polkit', 'assets' and the desktop file explicitly.
      src = let
        customFilter = path: type:
          (pkgs.lib.hasInfix "/polkit/" path) ||
          (pkgs.lib.hasInfix "/assets/" path) ||
          (pkgs.lib.hasSuffix "rebi.desktop" path) ||
          (craneLib.filterCargoSources path type);
      in pkgs.lib.cleanSourceWith {
        src = craneLib.path ./.;
        filter = customFilter;
      };

      commonArgs = {
        inherit src;
        strictDeps = true;
        pname = "rebi-workspace";
        version = "0.1.0";
        nativeBuildInputs = with pkgs; [
          pkg-config
        ];
      };
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      # 2. Build the helper
      rebi-helper = let
        metadata = craneLib.crateNameFromCargoToml { cargoToml = ./helper/Cargo.toml; };
      in craneLib.buildPackage (commonArgs // {
        inherit (metadata) version;
        inherit cargoArtifacts;
        pname = "rebi-helper";
        cargoExtraArgs = "-p rebi-helper";

        postInstall = ''
          install -Dm444 polkit/org.rebi.policy -t $out/share/polkit-1/actions
          install -Dm444 polkit/org.rebi.rules -t $out/share/polkit-1/rules.d

          # Patch the policy for NixOS to point to the system-wide binary
          substituteInPlace $out/share/polkit-1/actions/org.rebi.policy \
            --replace "/usr/bin/rebi-helper" "/run/current-system/sw/bin/rebi-helper"
        '';
      });

      # 3. Build the GUI
      rebi-gui = let
        metadata = craneLib.crateNameFromCargoToml { cargoToml = ./gui/Cargo.toml; };
        runtimeDeps = [ rebi-helper ];
        runtimeLibs = with pkgs; [
          wayland
          libxkbcommon
          libGL
          gtk3
        ];
      in craneLib.buildPackage (commonArgs // {
        inherit (metadata) version;
        inherit cargoArtifacts;
        pname = "rebi-gui";
        cargoExtraArgs = "-p rebi-gui";

        nativeBuildInputs = (commonArgs.nativeBuildInputs or []) ++ [ pkgs.makeWrapper ];
        buildInputs = (commonArgs.buildInputs or []) ++ runtimeLibs;

        postInstall = ''
          wrapProgram $out/bin/rebi-gui \
            --prefix PATH : ${pkgs.lib.makeBinPath runtimeDeps} \
            --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath runtimeLibs}
          
          install -Dm644 rebi.desktop -t $out/share/applications
          install -Dm644 assets/logo-dark.png $out/share/icons/hicolor/scalable/apps/rebi.png
          install -Dm644 assets/logo-light.png $out/share/icons/hicolor/scalable/apps/rebi-light.png
        '';
      });
    in { 
        inherit rebi-helper rebi-gui;
        default = rebi-gui;
      }
    );
  };
}
