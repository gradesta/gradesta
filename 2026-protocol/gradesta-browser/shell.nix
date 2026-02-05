{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    pkg-config
    rustc
    cargo
    cmake
    gcc
    llvmPackages.clang
  ];

  buildInputs = with pkgs; [
    wayland
    wayland-protocols
    libxkbcommon
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
    xorg.libXrender
    vulkan-loader
    openssl
    libGL
    alsa-lib
    # For whisper-rs-sys bindgen
    llvmPackages.libclang
  ];

  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
    wayland
    libxkbcommon
    vulkan-loader
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
    xorg.libXrender
    libGL
    alsa-lib
  ]);

  # For whisper-rs-sys build (bindgen needs libclang)
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

  shellHook = ''
    echo "Gradesta Browser development shell"
    echo "Run 'cargo build' to build the browser"
    echo "Run 'cargo run' to run the browser"
  '';
}
