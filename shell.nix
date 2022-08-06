with import <nixpkgs> {};
mkShell {
    buildInputs = [
        pkgconfig
        zeromq
        pkgs.llvmPackages.bintools
        cargo-watch
        cargo-tarpaulin
    ];
}
