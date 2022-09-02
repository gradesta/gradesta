with import <nixpkgs> {};
mkShell {
    buildInputs = [
        cargo
        rustc
        pkgconfig
        zeromq
        pkgs.llvmPackages.bintools
        cargo-watch
        cargo-tarpaulin
        python310
        python310Packages.pytest
        python310Packages.black
        python310Packages.pip
        python310Packages.virtualenv
    ];
}
