let pkgs = import <nixpkgs> { overlays = [ (import ./cypress-overlay.nix) ]; };

in pkgs.mkShell {
    buildInputs = with pkgs; [
        cypress
        ffmpeg
        cargo
        rustc
        pkgconfig
        zeromq
        pkgs.llvmPackages.bintools
        cargo-watch
        cargo-tarpaulin
        python310
        python310Packages.setuptools
        python310Packages.pip
        python310Packages.virtualenv
        python310Packages.wheel
    ];

  shellHook = ''
    export CYPRESS_INSTALL_BINARY=0
    export CYPRESS_RUN_BINARY=${pkgs.cypress}/bin/Cypress
  '';

}

