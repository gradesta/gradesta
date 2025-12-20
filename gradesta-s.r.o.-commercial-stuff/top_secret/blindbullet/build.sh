#!/usr/bin/env bash
# Build script for Blind Bullet
# On NixOS, run: nix-shell --run "./build.sh"
#
# Note: Fully static linking is not practical on NixOS for X11/OpenGL applications.
# The binary will be dynamically linked but can be made portable using the Nix derivation.

set -e

export CGO_ENABLED=1

echo "Building Blind Bullet (dynamic linking)..."
echo "Note: On NixOS, static linking X11/OpenGL libraries is not feasible."
echo "      The binary will work with ./run.sh or when built via 'nix-build'"
echo ""

go build -o blindbullet .

echo ""
echo "Build successful!"
echo "Run with: ./run.sh"
echo "Or build a wrapped version with: nix-build default.nix"

