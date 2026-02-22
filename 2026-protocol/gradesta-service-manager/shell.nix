{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust toolchain
    rustc
    cargo
    pkg-config
    openssl

    # Docker for service management
    docker
    docker-compose
  ];

  shellHook = ''
    echo "Gradesta Service Manager development shell"
    echo "Run 'cargo run -- --help' to see available commands"
  '';

  RUST_BACKTRACE = "1";
}
