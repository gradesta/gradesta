{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust toolchain for building the elf
    rustc
    cargo
    pkg-config
    openssl

    # Node.js for Claude Code CLI
    nodejs_20
    nodePackages.npm

    # Git for Claude Code operations
    git

    # Common tools Claude might use
    ripgrep
    fd
    jq
    curl
    wget
  ];

  shellHook = ''
    # Set up npm global directory in user space
    export NPM_CONFIG_PREFIX="$HOME/.npm-global"
    export PATH="$NPM_CONFIG_PREFIX/bin:$PATH"

    # Create the npm global directory if it doesn't exist
    mkdir -p "$NPM_CONFIG_PREFIX"

    # Check if claude is installed, if not, provide instructions
    if ! command -v claude &> /dev/null; then
      echo "Claude Code CLI not found. Install it with:"
      echo "  npm install -g @anthropic-ai/claude-code"
      echo ""
      echo "After installation, authenticate with:"
      echo "  claude auth"
    else
      echo "Claude Code CLI is available"
      claude --version 2>/dev/null || true
    fi

    echo ""
    echo "Claude Code Elf development shell ready"
    echo "Run 'cargo run' to start the elf server"
  '';

  # Environment variables
  RUST_BACKTRACE = "1";
}
