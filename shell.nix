with import <nixpkgs> {};
mkShell {
    buildInputs = [
        ffmpeg
        cargo
        rustc
        pkgconfig
        zeromq
        pkgs.llvmPackages.bintools
        cargo-watch
        cargo-tarpaulin
        python310
        python310Packages.black
        python310Packages.pytest
        python310Packages.pytest-cov
        python310Packages.setuptools
        python310Packages.pip
        python310Packages.virtualenv
    ];
}
