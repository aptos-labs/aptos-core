// Rejected ed25519 inputs: keys off the curve or of small order, signatures one byte short or
// long or with S plus the group order L, a signature that holds for any message under the
// identity key, and a 31-byte secret key. All but the identity-key case sit beside a control
// that differs only in that property. SK, PK and SIG are RFC 8032 test 1.

// RUN: publish
module 0x1::ed25519 {
    const SK: vector<u8> = x"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
    const PK: vector<u8> = x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
    const SIG: vector<u8> = x"e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";
    const MSG_ZERO_TAIL: vector<u8> = b"message 47";
    const SIG_ZERO_TAIL: vector<u8> = x"0ceeb2238b8fb96feee436a1e723bf16f8b06356ecfb6ae9c2007b9193b8ec5b5c7485887c7d10d8c70e194a2a449d592f2ac0539cc70e82b1306673c75cfd00";
    const SIG_S_PLUS_L: vector<u8> = x"e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901554c8c7872aa064e049dbb3013fbf29380d25bf5f0595bbe24655141438e7a101b";
    const OFF_CURVE: vector<u8> = x"0200000000000000000000000000000000000000000000000000000000000000";
    const IDENTITY: vector<u8> = x"0100000000000000000000000000000000000000000000000000000000000000";
    const ORDER_8: vector<u8> = x"c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a";
    const PK_PLUS_ORDER_8: vector<u8> = x"9158312a9a8d6e3b34c891d6d61444f8b8211c5117ebad15bdb0bd68b07e0245";
    const BASE_POINT_S_ONE: vector<u8> = x"58666666666666666666666666666666666666666666666666666666666666660100000000000000000000000000000000000000000000000000000000000000";

    native fun public_key_validate_internal(bytes: vector<u8>): bool;
    native fun signature_verify_strict_internal(
        signature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>,
    ): bool;
    native fun sign_internal(sk: vector<u8>, msg: vector<u8>): vector<u8>;

    public fun validate_off_curve(): (bool, bool) {
        (public_key_validate_internal(PK), public_key_validate_internal(OFF_CURVE))
    }

    public fun validate_small_order(): (bool, bool, bool) {
        (
            public_key_validate_internal(PK_PLUS_ORDER_8),
            public_key_validate_internal(IDENTITY),
            public_key_validate_internal(ORDER_8),
        )
    }

    // SIG_ZERO_TAIL ends in a zero byte, so a padding or truncating decoder would recover it
    // from `short` or `long`.
    public fun verify_signature_wrong_length(): (bool, bool, bool) {
        let short = SIG_ZERO_TAIL;
        short.pop_back();
        let long = SIG_ZERO_TAIL;
        long.push_back(0);
        (
            signature_verify_strict_internal(SIG_ZERO_TAIL, PK, MSG_ZERO_TAIL),
            signature_verify_strict_internal(short, PK, MSG_ZERO_TAIL),
            signature_verify_strict_internal(long, PK, MSG_ZERO_TAIL),
        )
    }

    public fun verify_signature_s_not_reduced(): (bool, bool) {
        (
            signature_verify_strict_internal(SIG, PK, b""),
            signature_verify_strict_internal(SIG_S_PLUS_L, PK, b""),
        )
    }

    // R = base point and S = 1 satisfy the verification equation for any message under the
    // identity key, so only the key check rejects it.
    public fun verify_small_order_key(): bool {
        signature_verify_strict_internal(BASE_POINT_S_ONE, IDENTITY, b"any message")
    }

    public fun sign_valid_key(): vector<u8> {
        sign_internal(SK, b"")
    }

    public fun sign_short_key(): vector<u8> {
        let sk = SK;
        sk.pop_back();
        sign_internal(sk, b"")
    }
}

// RUN: execute 0x1::ed25519::validate_off_curve
// CHECK: results: true, false

// RUN: execute 0x1::ed25519::validate_small_order
// CHECK: results: true, false, false

// RUN: execute 0x1::ed25519::verify_signature_wrong_length
// CHECK: results: true, false, false

// RUN: execute 0x1::ed25519::verify_signature_s_not_reduced
// CHECK: results: true, false

// RUN: execute 0x1::ed25519::verify_small_order_key
// CHECK: results: false

// RUN: execute 0x1::ed25519::sign_valid_key
// CHECK: results: 0xe5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b

// RUN: execute 0x1::ed25519::sign_short_key
// CHECK: aborted: code 655361 (Ed25519 sign computation failed) in 0x1::ed25519
