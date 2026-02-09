{ pkgs ? import <nixpkgs> {} }:

let
  androidComposition = pkgs.androidenv.composeAndroidPackages {
    buildToolsVersions = [ "34.0.0" ];
    platformVersions = [ "34" ];
    abiVersions = [ "x86_64" "arm64-v8a" ];
    includeEmulator = true;
    includeSystemImages = true;
    systemImageTypes = [ "google_apis" ];
    includeSources = false;
    includeNDK = false;
  };
  androidSdk = androidComposition.androidsdk;
  androidEmulator = androidComposition.emulator;

  # FHS environment to run Android build tools
  fhs = pkgs.buildFHSEnv {
    name = "android-fhs-env";
    targetPkgs = pkgs: with pkgs; [
      # Java
      jdk17

      # Android SDK
      androidSdk

      # Build tools
      gradle

      # Required for AAPT2 and other Android tools
      zlib
      ncurses5
      stdenv.cc.cc
      glibc

      # For emulator
      libGL
      libpulseaudio
      xorg.libX11
      xorg.libXext
      xorg.libXrandr
      xorg.libXi
      xorg.libXcursor
      xorg.libXfixes
      vulkan-loader
    ];

    profile = ''
      export JAVA_HOME="${pkgs.jdk17}"
      export ANDROID_HOME="${androidSdk}/libexec/android-sdk"
      export ANDROID_SDK_ROOT="${androidSdk}/libexec/android-sdk"
      export ANDROID_AVD_HOME="$HOME/.android/avd"
    '';

    runScript = "bash";
  };
in
pkgs.mkShell {
  buildInputs = [
    fhs
    pkgs.jdk17
    pkgs.gradle
  ];

  shellHook = ''
    echo "Gradesta Android development environment"
    echo ""
    echo "To build the Android app:"
    echo "  android-fhs-env -c './gradlew assembleDebug'"
    echo ""
    echo "To create an emulator AVD (first time only):"
    echo "  android-fhs-env -c 'avdmanager create avd -n gradesta_avd -k \"system-images;android-34;google_apis;x86_64\" --device \"pixel_5\"'"
    echo ""
    echo "To run the emulator:"
    echo "  android-fhs-env -c 'emulator -avd gradesta_avd'"
    echo ""
    echo "To install and run the app:"
    echo "  android-fhs-env -c 'adb install app/build/outputs/apk/debug/app-debug.apk'"
  '';
}
