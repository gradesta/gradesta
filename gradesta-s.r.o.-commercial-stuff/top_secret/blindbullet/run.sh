#!/usr/bin/env bash
# Wrapper script to run Blind Bullet with proper library paths on NixOS
# This sets up the LD_LIBRARY_PATH to find OpenGL and X11 libraries

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Set up library paths for OpenGL and X11
# On NixOS, OpenGL drivers are typically in /run/opengl-driver/lib
export LD_LIBRARY_PATH="${LD_LIBRARY_PATH}:/run/opengl-driver/lib"

# If we're in a nix-shell, add nix store library paths
if [ -n "$NIX_SHELL" ] || [ -n "$IN_NIX_SHELL" ]; then
    # Add common library paths from nix-shell
    for lib in libGL libGLU glfw xorg.libX11 xorg.libXcursor xorg.libXrandr xorg.libXi xorg.libXinerama xorg.libXxf86vm xorg.libXext; do
        var_name=$(echo "$lib" | tr '.' '_' | tr '[:lower:]' '[:upper:]')
        if [ -n "${!var_name}" ]; then
            export LD_LIBRARY_PATH="${LD_LIBRARY_PATH}:${!var_name}/lib"
        fi
    done
    
    # Also try direct variable names (lowercase)
    for var in libGL libGLU glfw; do
        if [ -n "${!var}" ]; then
            export LD_LIBRARY_PATH="${LD_LIBRARY_PATH}:${!var}/lib"
        fi
    done
fi

# Try to find libraries in common Nix store locations
# This helps when not in nix-shell
if [ -d "/nix/store" ]; then
    # Use find to locate libraries (this is a fallback)
    # In practice, you should use nix-build for a properly wrapped binary
    :
fi

# Run the executable
exec "${SCRIPT_DIR}/blindbullet" "$@"

