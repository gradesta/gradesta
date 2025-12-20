{ pkgs ? import <nixpkgs> {} }:

pkgs.buildGoModule {
  pname = "blindbullet";
  version = "0.1.0";
  src = ./.;
  
  vendorHash = "sha256-GTEzktjQuXbp4wUU6SgRd7VPVj9xZwsQNLzjq83Z/0o=";
  
  nativeBuildInputs = with pkgs; [
    makeWrapper
    pkg-config
  ];
  
  buildInputs = with pkgs; [
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

  env.CGO_ENABLED = "1";
  
  preBuild = ''
    export PKG_CONFIG_PATH="${pkgs.glfw}/lib/pkgconfig:${pkgs.libGL}/lib/pkgconfig"
  '';

  postInstall = ''
    # Create a wrapper that sets up library paths
    wrapProgram $out/bin/blindbullet \
      --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath [
        pkgs.glfw
        pkgs.libGL
        pkgs.libGLU
        pkgs.xorg.libX11
        pkgs.xorg.libXcursor
        pkgs.xorg.libXrandr
        pkgs.xorg.libXi
        pkgs.xorg.libXinerama
        pkgs.xorg.libXxf86vm
        pkgs.xorg.libXext
      ]}"
  '';
}

