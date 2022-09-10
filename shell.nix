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
    ];
}
