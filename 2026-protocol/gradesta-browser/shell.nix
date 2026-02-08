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
    # External video player for video content
    mpv
    # Text-to-speech (Speech Dispatcher)
    speechd
    # Vulkan SDK for GPU-accelerated whisper (optional: build with --features gpu-vulkan)
    vulkan-headers
    shaderc        # provides glslc shader compiler
    vulkan-validation-layers
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

  # Help cmake find Vulkan SDK components
  VULKAN_SDK = "${pkgs.vulkan-headers}";
  VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";

  shellHook = ''
    echo "Gradesta Browser development shell"
    echo "Run 'cargo build' to build the browser"
    echo "Run 'cargo run' to run the browser"
  '';
}
