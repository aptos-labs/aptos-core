{ pkgs ? import <nixpkgs> { overlays = [ (import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz")) ]; } }:

let
  rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;
in

pkgs.mkShell {
  name = "aptos-core-dev";

  buildInputs = with pkgs; [
    rustToolchain
    rustfmt
    clippy
    rust-analyzer

    # System dependencies
    openssl
    pkg-config
    cmake
    clang
    rocksdb
    protobuf

    # Development tools
    git
    curl
    jq
    nodejs

    # Additional development tools
    cargo-watch
    cargo-audit
    cargo-expand
    cargo-nextest
    bacon
    typos
  ];

  # Environment variables
  RUST_BACKTRACE = 1;
  ROCKSDB_LIB_DIR = "${pkgs.rocksdb}/lib";
  ROCKSDB_STATIC = "true";
  OPENSSL_NO_VENDOR = "1";
  OPENSSL_DIR = "${pkgs.openssl.dev}";

  # Shell hooks
  shellHook = ''
    echo "Welcome to the Aptos Core development environment"
    echo "Run 'cargo build' to build the project"
  '';
}