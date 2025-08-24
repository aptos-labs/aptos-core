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

        # Create a custom rustPlatform with the specified toolchain
        customRustPlatform = pkgs.makeRustPlatform {
          rustc = rustToolchain;
          cargo = rustToolchain;
        };

      in
      {
        packages = {
          # Packages removed - using development shell approach instead
          # Use 'just build' or 'just aptos-node' for building
        };

        devShells = {
          default = pkgs.mkShell {
            name = "aptos-node-dev";

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
              libclang
              llvm
              lld  # Add lld linker for Rust builds
              elfutils
              elfutils.dev  # Add elfutils dev package for headers
              elfutils.out  # Add elfutils main package for libdw.so.1
              zlib  # Added zlib library
              pkgs.udev
              jemalloc  # Added jemalloc for jemalloc-sys override

              # Development tools
              git
              curl
              jq
              nodejs
            ];

            # Environment variables for library paths
            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
            BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.libclang.dev}/include";
            
            # Add PKG_CONFIG_PATH to help find libraries
            PKG_CONFIG_PATH = "${pkgs.elfutils.dev}/lib/pkgconfig:${pkgs.zlib}/lib/pkgconfig:$PKG_CONFIG_PATH";
            
            # Additional library paths
            LD_LIBRARY_PATH = "${pkgs.libclang.lib}/lib:${pkgs.llvm.lib}/lib:${pkgs.elfutils}/lib:${pkgs.elfutils.out}/lib:${pkgs.zlib}/lib:${pkgs.jemalloc}/lib:${pkgs.rocksdb}/lib:${pkgs.stdenv.cc.cc.lib}/lib:${pkgs.openssl.out}/lib:$LD_LIBRARY_PATH";

            # Environment variables for build configuration
            CARGO_BUILD_RUSTFLAGS = "-C target-feature=+sse4.2 -C opt-level=3";
            RUST_BACKTRACE = 1;
            ROCKSDB_LIB_DIR = "${pkgs.rocksdb}/lib";
            
            # Fix jemalloc-sys build issues with strerror_r
            JEMALLOC_SYS_WITH_MALLOC_CONF = "";
            # Override jemalloc-sys to use system jemalloc instead of building from source
            JEMALLOC_OVERRIDE = "${pkgs.jemalloc}/lib/libjemalloc.so";

            # Shell hooks to ensure correct toolchain is used
            shellHook = ''
              echo "Welcome to the Aptos Node development environment"
              echo "Run 'cargo build' to build the project"
              
              # Ensure Nix-provided Clang and LLVM are used
              export PATH="${pkgs.clang}/bin:${pkgs.llvm}/bin:$PATH"
            '';
          };
        };

        apps = {
          aptos-node = {
            type = "app";
            program = "${self.packages.${system}.aptos-node}/bin/aptos-node";
          };
        };
      });
}