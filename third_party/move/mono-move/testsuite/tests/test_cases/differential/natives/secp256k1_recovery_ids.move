// secp256k1 recovery: s equal to the curve order n aborts while n - 1 recovers. For SMALL_R
// (r = 2, s = 1; both 2 and 2 + n are curve x-coordinates) ids 0, 2 and 3 recover keys (2 and
// 3 use R.x = r + n) and id 4 aborts; for SIG, whose r is above p - n, ids 2 and 3 fail. SIG
// and the message come from aptos_std's secp256k1 test; recovered keys were computed
// independently.

// RUN: publish
module 0x1::secp256k1 {
    const SIG: vector<u8> = x"f7ad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c777c423a1d2849baafd7ff6a9930814a43c3f80d59db56f";
    // The r half of SIG, then s = n.
    const S_IS_ORDER: vector<u8> = x"f7ad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141";
    // The r half of SIG, then s = n - 1.
    const S_BELOW_ORDER: vector<u8> = x"f7ad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364140";
    const SMALL_R: vector<u8> = x"00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000001";

    native fun ecdsa_recover_internal(
        message: vector<u8>,
        recovery_id: u8,
        signature: vector<u8>,
    ): (vector<u8>, bool);

    fun message(): vector<u8> {
        std::hash::sha2_256(b"test aptos secp256k1")
    }

    public fun s_is_order(): (vector<u8>, bool) {
        ecdsa_recover_internal(message(), 0, S_IS_ORDER)
    }

    public fun s_below_order(): (vector<u8>, bool) {
        ecdsa_recover_internal(message(), 0, S_BELOW_ORDER)
    }

    public fun small_r_id_zero(): (vector<u8>, bool) {
        ecdsa_recover_internal(message(), 0, SMALL_R)
    }

    public fun small_r_id_two(): (vector<u8>, bool) {
        ecdsa_recover_internal(message(), 2, SMALL_R)
    }

    public fun small_r_id_three(): (vector<u8>, bool) {
        ecdsa_recover_internal(message(), 3, SMALL_R)
    }

    public fun small_r_id_four(): (vector<u8>, bool) {
        ecdsa_recover_internal(message(), 4, SMALL_R)
    }

    public fun large_r_ids_two_three(): (vector<u8>, bool, vector<u8>, bool) {
        let (key2, ok2) = ecdsa_recover_internal(message(), 2, SIG);
        let (key3, ok3) = ecdsa_recover_internal(message(), 3, SIG);
        (key2, ok2, key3, ok3)
    }
}

// RUN: execute 0x1::secp256k1::s_is_order
// CHECK: aborted: code 65537 (Signature must be exactly 64 bytes) in 0x1::secp256k1

// RUN: execute 0x1::secp256k1::s_below_order
// CHECK: results: 0x5c12bc018385eba785989d184bba3c79d2aea00751d9377325f5db39ee22e4a7225439c21b011a714ce8f6b6737ebdadec5d28b8055aafd4320afa5641ad2bd2, true

// RUN: execute 0x1::secp256k1::small_r_id_zero
// CHECK: results: 0xaa5256480ff7edb32e454bbff43d2f3b2e8688cf3551e4d728f5e7771c850de1202948e3262d80c19f82287af086ccfe72dba3d9944f407000c15de8dc01d495, true

// RUN: execute 0x1::secp256k1::small_r_id_two
// CHECK: results: 0xa943964f6961532416757d6af94e385b88ce6cb449fe42e60368ea369acbb0606674e7fa9a7fc839e03fbe14e9cbf5a4cc275376078ea38047bd0880bf07ba23, true

// RUN: execute 0x1::secp256k1::small_r_id_three
// CHECK: results: 0x244b7811037127f74e1fa5d717146a28b91a53c601c2bd546bb5885e7f2b18890f5f1b6837de5c8a4075268944e7ba1a73cdb6a85f4ce7f01b268e1f0ff97d24, true

// RUN: execute 0x1::secp256k1::small_r_id_four
// CHECK: aborted: code 65537 (Recovery ID must be 0, 1, 2, or 3) in 0x1::secp256k1

// RUN: execute 0x1::secp256k1::large_r_ids_two_three
// CHECK: results: 0x, false, 0x, false
