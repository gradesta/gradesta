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
    ];
}
