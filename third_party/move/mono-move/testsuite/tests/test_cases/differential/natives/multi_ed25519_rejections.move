// Rejected multi_ed25519 inputs: off-curve or small-order sub-keys, threshold 0, signatures one
// byte short or long, a wrong bitmap, S plus L, and more than 32 generated keys. Each sits
// beside a control that differs only in that property. PK1, PK2 and SIG1 are from RFC 8032
// tests 1 and 2. `public_key_validate_internal` ignores a trailing threshold byte that the v2
// validator rejects.

// RUN: publish
module 0x1::multi_ed25519 {
    const BASE: vector<u8> = x"5866666666666666666666666666666666666666666666666666666666666666";
    const IDENTITY: vector<u8> = x"0100000000000000000000000000000000000000000000000000000000000000";
    const OFF_CURVE: vector<u8> = x"0200000000000000000000000000000000000000000000000000000000000000";
    const PK1: vector<u8> = x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
    const PK2: vector<u8> = x"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";
    const SIG1: vector<u8> = x"e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";
    // SIG1 with L added to S.
    const SIG1_S_PLUS_L: vector<u8> = x"e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901554c8c7872aa064e049dbb3013fbf29380d25bf5f0595bbe24655141438e7a101b";
    const SIGNER_0: vector<u8> = x"80000000";
    const SIGNERS_0_1: vector<u8> = x"c0000000";

    native fun public_key_validate_internal(bytes: vector<u8>): bool;
    native fun public_key_validate_v2_internal(bytes: vector<u8>): bool;
    native fun signature_verify_strict_internal(
        multisignature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>,
    ): bool;
    native fun generate_keys_internal(threshold: u8, num_keys: u8): (vector<u8>, vector<u8>);

    fun concat(parts: vector<vector<u8>>): vector<u8> {
        let out = vector[];
        parts.for_each(|part| out.append(part));
        out
    }

    fun verify(sig: vector<u8>, pk: vector<u8>): bool {
        signature_verify_strict_internal(sig, pk, b"")
    }

    public fun v1_trailing_threshold_ignored(): (bool, bool, bool, bool) {
        let zero = concat(vector[BASE, BASE, x"00"]);
        let above = concat(vector[BASE, BASE, x"03"]);
        (
            public_key_validate_internal(zero),
            public_key_validate_internal(above),
            public_key_validate_v2_internal(zero),
            public_key_validate_v2_internal(above),
        )
    }

    public fun v1_off_curve_subkey(): (bool, bool, bool) {
        (
            public_key_validate_internal(concat(vector[BASE, BASE])),
            public_key_validate_internal(concat(vector[BASE, OFF_CURVE])),
            public_key_validate_internal(concat(vector[OFF_CURVE, BASE])),
        )
    }

    public fun v1_valid_then_invalid(): (bool, bool, bool) {
        (
            public_key_validate_internal(concat(vector[BASE, BASE, BASE])),
            public_key_validate_internal(concat(vector[BASE, BASE, IDENTITY])),
            public_key_validate_internal(concat(vector[BASE, BASE, OFF_CURVE])),
        )
    }

    public fun verify_pk_malformed(): (bool, bool, bool) {
        let sig = concat(vector[SIG1, SIGNER_0]);
        (
            verify(sig, concat(vector[PK1, PK2, x"01"])),
            verify(sig, concat(vector[PK1, PK2, x"00"])),
            verify(sig, concat(vector[PK1, OFF_CURVE, x"01"])),
        )
    }

    public fun verify_signature_malformed(): (bool, bool, bool, bool, bool) {
        let pk = concat(vector[PK1, PK2, x"01"]);
        // The valid signature ends in a zero bitmap byte, so a padding or truncating decoder
        // would recover it from `short` or `long`.
        let short = concat(vector[SIG1, SIGNER_0]);
        short.pop_back();
        let long = concat(vector[SIG1, SIGNER_0, x"00"]);
        (
            verify(concat(vector[SIG1, SIGNER_0]), pk),
            verify(short, pk),
            verify(long, pk),
            verify(concat(vector[SIG1, SIGNERS_0_1]), pk),
            verify(concat(vector[SIG1_S_PLUS_L, SIGNER_0]), pk),
        )
    }

    public fun generate_32_keys(): (u64, u64) {
        let (sk, pk) = generate_keys_internal(1, 32);
        (sk.length(), pk.length())
    }

    public fun generate_33_keys(): u64 {
        let (sk, _) = generate_keys_internal(1, 33);
        sk.length()
    }
}

// RUN: execute 0x1::multi_ed25519::v1_trailing_threshold_ignored
// CHECK: results: true, true, false, false

// RUN: execute 0x1::multi_ed25519::v1_off_curve_subkey
// CHECK: results: true, false, false

// RUN: execute 0x1::multi_ed25519::v1_valid_then_invalid
// CHECK: results: true, false, false

// RUN: execute 0x1::multi_ed25519::verify_pk_malformed
// CHECK: results: true, false, false

// RUN: execute 0x1::multi_ed25519::verify_signature_malformed
// CHECK: results: true, false, false, false, false

// RUN: execute 0x1::multi_ed25519::generate_32_keys
// CHECK: results: 1025, 1025

// RUN: execute 0x1::multi_ed25519::generate_33_keys
// CHECK: aborted: code 655361 (MultiEd25519 secret key creation failed) in 0x1::multi_ed25519
