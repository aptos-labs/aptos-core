spec aptos_std::secp256r1 {
    spec ecdsa_raw_public_key_from_64_bytes(bytes: vector<u8>): ECDSARawPublicKey {
        aborts_if len(bytes) != RAW_PUBLIC_KEY_NUM_BYTES;
        ensures result == ECDSARawPublicKey { bytes };
    }
}
