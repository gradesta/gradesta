{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    pkg-config
    rustc
    cargo
    rustfmt
    clippy
  ];

  buildInputs = with pkgs; [
    openssl
  ];

  shellHook = ''
    echo "Gradesta Nextcloud Connector development shell"
    echo "Run 'cargo build' to build the connector"
    echo "Run 'cargo run' to run the connector"
  '';
}
