{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "rebi-dev-shell";

  # Dependencias que necesitas para compilar y ejecutar en tiempo de desarrollo
  buildInputs = with pkgs; [
    cargo
    rustc
    rust-analyzer
    clippy
    
    # Librerías necesarias para Winit / Iced bajo Wayland (Hyprland)
    wayland
    libxkbcommon
    libGL
    
    # Dependencias para los diálogos de archivos (RFD)
    glib
    gtk3
  ];

  # Variables de entorno críticas para que Winit encuentre las librerías dinámicas
  shellHook = ''
    export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${
      with pkgs;
      lib.makeLibraryPath [
        wayland
        libxkbcommon
        libGL
        gtk3
      ]
    }"
    
    # Aseguramos que use Wayland nativo en Hyprland
    export WINIT_UNIX_BACKEND=wayland
  '';
}
