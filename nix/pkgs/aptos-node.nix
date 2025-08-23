{ lib
, rustPlatform
, openssl
, pkg-config
, cmake
, clang
, rocksdb
, protobuf
, ...
}:

rustPlatform.buildRustPackage rec {
  pname = "aptos-node";
  version = "main";

  # Use an explicit source path
  src = lib.cleanSource ../..;

  cargoLock = {
    lockFile = ../../Cargo.lock;
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
    rocksdb
  ];

  # Fix RocksDB compilation issues
  ROCKSDB_LIB_DIR = "${rocksdb}/lib";
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