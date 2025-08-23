{ lib
, rustPlatform
, openssl
, pkg-config
, cmake
, clang
, RocksDB
, protobuf
, ...
}:

rustPlatform.buildRustPackage rec {
  pname = "aptos-node";
  version = "main";

  src = ../..;

  cargoLock = {
    lockFile = ../../Cargo.lock;
    outputHashes = {
      "rocksdb-0.22.0" = "sha256-7c42i0S9X7f85C/1j7DlYQvZv5UQZ3n3z3n3z3n3z3n=";
    };
  };

  # Only build the aptos-node package, not the entire workspace
  cargoBuildType = "release";
  cargoBuildFlags = [ "-p" "aptos-node" ];

  nativeBuildInputs = [
    pkg-config
    cmake
    clang
    protobuf
  ];

  buildInputs = [
    openssl
    RocksDB
  ];

  # Fix RocksDB compilation issues
  ROCKSDB_LIB_DIR = "${RocksDB}/lib";
  ROCKSDB_STATIC = "true";

  # Additional environment variables for RocksDB
  OPENSSL_NO_VENDOR = "1";
  OPENSSL_DIR = "${openssl.dev}";
  OPENSSL_LIB_DIR = "${openssl.out}/lib";
  OPENSSL_INCLUDE_DIR = "${openssl.dev}/include";
  PKG_CONFIG_PATH = "${openssl.dev}/lib/pkgconfig";

  meta = with lib; {
    description = "Aptos Node - Layer 1 blockchain node for scalable, secure, and upgradeable web3 infrastructure";
    homepage = "https://aptoslabs.com";
    license = licenses.asl20;
    maintainers = with maintainers; [ ];
    platforms = platforms.all;
  };
}