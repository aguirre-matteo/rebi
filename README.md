<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="assets/logo-light.png">
    <img alt="Rebi Logo" src="assets/logo-dark.png" width="300">
  </picture>
</p>

# Rebi

Rebi is a modern frontend for keyd, a powerful keyboard remapping daemon for Linux. It is built with Rust and the Iced GUI framework, Rebi provides an intuitive interface to manage complex keyboard and mouse remapping profiles without manual configuration file editing.

## Features

- Profile Management: Create and switch between multiple remapping profiles for different workflows.
- Device Targeting: Specifically target individual keyboards or mice by their hardware IDs.
- Layer Support: Define multiple layers with specific modifiers and activation rules.
- Action Types: Support for advanced keyd features including Overload, Timeout, Macros, and Command execution.
- Key Recording: Easily record key combinations and mouse buttons directly through the interface.

## Prerequisites

Rebi requires the keyd daemon to be installed and running on your system.

## Installation

### Arch Linux

You can build the package from source using the provided PKGBUILD:

```bash
makepkg -si
```

### Debian / Ubuntu

Build and install the .deb package:

```bash
cargo build --release
cargo deb -p rebi-gui
sudo dpkg -i target/debian/rebi_0.1.0_amd64.deb
```

### Fedora

Build and install the RPM package:

```bash
cargo build --release
cargo generate-rpm -p rebi-gui
sudo dnf install target/generate-rpm/rebi-0.1.0-1.x86_64.rpm
```

### NixOS

Add this repository to your flake: 

```nix
{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rebi = {
      url = "github:aguirre-matteo/rebi";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
}
```

Import the NixOS module and enable Rebi on your config:

```nix
{ inputs, pkgs, ... }:

{
  imports = [ inputs.rebi.nixosModules.default ];
  programs.rebi.enable = true;
}
```

Rebuild your system:

```bash
nixos-rebuild switch
```

## Usage

1. Launch Rebi from your application menu or run `rebi-gui` from the terminal.
2. Create a new profile or select an existing one.
3. Define your target devices in the ID section.
4. Add layers and mapping rules as needed.
5. Click "Apply ALL to System" to sync the configuration with keyd. You will be prompted for authentication.

## License

This project is licensed under the GPL-2.0 License.
