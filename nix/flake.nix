{
  description = "Aptos Core - Layer 1 blockchain";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;

      in
      {
        packages = {
          aptos-core = pkgs.callPackage ./nix/pkgs/aptos-core.nix {
            rustPlatform = pkgs.rustPlatform.override {
              rustc = rustToolchain;
              cargo = rustToolchain;
            };
          };

          aptos-core-docker = pkgs.callPackage ./nix/pkgs/aptos-core-docker.nix {
            aptos-core = self.packages.${system}.aptos-core;
          };

          default = self.packages.${system}.aptos-core;
        };

        devShells = {
          default = pkgs.mkShell {
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
            ];

            # Environment variables
            RUST_BACKTRACE = 1;
            ROCKSDB_LIB_DIR = "${pkgs.rocksdb}/lib";

            # Shell hooks
            shellHook = ''
              echo "Welcome to the Aptos Core development environment"
              echo "Run 'cargo build' to build the project"
            '';
          };
        };

        apps = {
          aptos-node = {
            type = "app";
            program = "${self.packages.${system}.aptos-core}/bin/aptos-node";
          };
        };
      });
}