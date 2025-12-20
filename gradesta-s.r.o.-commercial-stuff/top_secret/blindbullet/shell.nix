{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    go
    gcc
    pkg-config
    glfw
    libGL
    libGLU
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    xorg.libXinerama
    xorg.libXxf86vm
    xorg.libXext
  ];

  shellHook = ''
    export CGO_ENABLED=1
    export PKG_CONFIG_PATH="${pkgs.glfw}/lib/pkgconfig:${pkgs.libGL}/lib/pkgconfig"
    # For static linking
    export CC=gcc
  '';
}

