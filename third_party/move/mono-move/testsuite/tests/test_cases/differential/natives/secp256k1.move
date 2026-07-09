// Differential test for the secp256k1 native.
//
// The aptos-framework is not pre-published here, so the native is re-declared
// inline (same pattern as aptos_hash.move). Test vectors come from
// aptos-stdlib/sources/cryptography/secp256k1.move's test_ecdsa_recover.

// RUN: publish
module 0x1::secp256k1 {
    native fun ecdsa_recover_internal(
        message: vector<u8>,
        recovery_id: u8,
        signature: vector<u8>,
    ): (vector<u8>, bool);

    // Valid signature: recovers the signer's 64-byte public key.
    public fun recover_valid(): (vector<u8>, bool) {
        ecdsa_recover_internal(
            std::hash::sha2_256(b"test aptos secp256k1"),
            0u8,
            x"f7ad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c777c423a1d2849baafd7ff6a9930814a43c3f80d59db56f",
        )
    }

    // Flipped bits in the signature still parse, but recovery fails: ([], false).
    public fun recover_invalid(): (vector<u8>, bool) {
        ecdsa_recover_internal(
            std::hash::sha2_256(b"test aptos secp256k1"),
            0u8,
            x"ffad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c7f7c423a1d2849baafd7ff6a9930814a43c3f80d59db56f",
        )
    }

    // Message that is not 32 bytes: native aborts with NFE_DESERIALIZE (0x01_0001).
    public fun recover_bad_message(): (vector<u8>, bool) {
        ecdsa_recover_internal(
            b"too short",
            0u8,
            x"f7ad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c777c423a1d2849baafd7ff6a9930814a43c3f80d59db56f",
        )
    }
}

// RUN: execute 0x1::secp256k1::recover_valid
// CHECK: results: 0x4646ae5047316b4230d0086c8acec687f00b1cd9d1dc634f6cb358ac0a9a8ffffe77b4dd0a4bfb95851f3b7355c781dd60f8418fc8a65d14907aff47c903a559, true

// RUN: execute 0x1::secp256k1::recover_invalid
// CHECK: results: 0x, false

// RUN: execute 0x1::secp256k1::recover_bad_message
// CHECK-SUBSTR: aborted: code 65537
