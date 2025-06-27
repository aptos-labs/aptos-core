{
  description = "Aptos Core Nix Dev Environment";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-compat.url = "github:edolstra/flake-compat/v1.1.0";
    systems.url = "github:nix-systems/default";
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };
  };
  outputs = { self, nixpkgs, flake-utils, ... }:
  let
  in flake-utils.lib.eachDefaultSystem (system:
  let
    pkgs = import nixpkgs {
      inherit system;
    };
  in {
    devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          # Rust toolchain
          cargo rustfmt rustc clippy rust-analyzer

          # Cargo extensions
          cargo-outdated

          # Build tools
          llvmPackages_latest.clang
          llvmPackages_latest.bintools
          gcc13

          # System libraries
          openssl
          pkg-config
          libudev-zero
          elfutils
        ];
        RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
        LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ pkgs.llvmPackages_latest.libclang.lib ];
    };
  });
}
