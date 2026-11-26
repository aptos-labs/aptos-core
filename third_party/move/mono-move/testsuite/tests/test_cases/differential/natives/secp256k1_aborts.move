// secp256k1 recovery input checks with their abort messages: a message that is not 32 bytes,
// recovery id 4, a 63-byte signature, and r equal to the curve order. Recovery id 1 on the
// same signature recovers a key.

// RUN: publish
module 0x1::secp256k1 {
    const SIG: vector<u8> = x"f7ad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c777c423a1d2849baafd7ff6a9930814a43c3f80d59db56f";
    // r = n, then the s half of SIG.
    const R_IS_ORDER: vector<u8> = x"fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141646bba0e1a37f9e7c777c423a1d2849baafd7ff6a9930814a43c3f80d59db56f";

    native fun ecdsa_recover_internal(
        message: vector<u8>,
        recovery_id: u8,
        signature: vector<u8>,
    ): (vector<u8>, bool);

    fun message(): vector<u8> {
        std::hash::sha2_256(b"test aptos secp256k1")
    }

    public fun bad_message(): (vector<u8>, bool) {
        ecdsa_recover_internal(b"too short", 0, SIG)
    }

    public fun bad_recovery_id(): (vector<u8>, bool) {
        ecdsa_recover_internal(message(), 4, SIG)
    }

    public fun short_signature(): (vector<u8>, bool) {
        let sig = SIG;
        sig.pop_back();
        ecdsa_recover_internal(message(), 0, sig)
    }

    public fun r_is_order(): (vector<u8>, bool) {
        ecdsa_recover_internal(message(), 0, R_IS_ORDER)
    }

    public fun recovery_id_one(): (vector<u8>, bool) {
        ecdsa_recover_internal(message(), 1, SIG)
    }
}

// RUN: execute 0x1::secp256k1::bad_message
// CHECK: aborted: code 65537 (Message must be exactly 32 bytes) in 0x1::secp256k1

// RUN: execute 0x1::secp256k1::bad_recovery_id
// CHECK: aborted: code 65537 (Recovery ID must be 0, 1, 2, or 3) in 0x1::secp256k1

// RUN: execute 0x1::secp256k1::short_signature
// CHECK: aborted: code 65537 (Signature must be exactly 64 bytes) in 0x1::secp256k1

// RUN: execute 0x1::secp256k1::r_is_order
// CHECK: aborted: code 65537 (Signature must be exactly 64 bytes) in 0x1::secp256k1

// RUN: execute 0x1::secp256k1::recovery_id_one
// CHECK: results: 0x69d9e026614af075416996e9c4f4680323fb7ab3bfdd0bf153c51f56c800ee2c7b47fd0b719006c5f4e9605dee17abcbd54b1518044932053148f73b288530f7, true
