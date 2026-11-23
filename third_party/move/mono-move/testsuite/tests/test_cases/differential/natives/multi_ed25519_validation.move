// multi_ed25519 key validation on keys built from the ed25519 base point: 32 and 33
// sub-keys, the empty key, a v2 key without its threshold byte, thresholds of 0 and above n,
// and a v2 key with a small-order sub-key. Also key generation with threshold 0 or above n,
// and signing with a malformed secret key.

// RUN: publish
module 0x1::multi_ed25519 {
    const BASE: vector<u8> = x"5866666666666666666666666666666666666666666666666666666666666666";
    const IDENTITY: vector<u8> = x"0100000000000000000000000000000000000000000000000000000000000000";

    native fun public_key_validate_internal(bytes: vector<u8>): bool;
    native fun public_key_validate_v2_internal(bytes: vector<u8>): bool;
    native fun generate_keys_internal(threshold: u8, count: u8): (vector<u8>, vector<u8>);
    native fun sign_internal(sk: vector<u8>, message: vector<u8>): vector<u8>;

    fun keys(count: u64): vector<u8> {
        let out = vector[];
        let index = 0;
        while (index < count) {
            out.append(BASE);
            index += 1;
        };
        out
    }

    fun with_threshold(count: u64, threshold: u8): vector<u8> {
        let out = keys(count);
        out.push_back(threshold);
        out
    }

    public fun v1_cases(): (bool, bool, bool, bool) {
        (
            public_key_validate_internal(keys(32)),
            public_key_validate_internal(keys(33)),
            public_key_validate_internal(vector[]),
            public_key_validate_internal(with_threshold(2, 1)),
        )
    }

    public fun v2_cases(): (bool, bool, bool, bool, bool) {
        let small_order = BASE;
        small_order.append(IDENTITY);
        small_order.push_back(1);
        (
            public_key_validate_v2_internal(with_threshold(2, 2)),
            public_key_validate_v2_internal(keys(2)),
            public_key_validate_v2_internal(with_threshold(2, 0)),
            public_key_validate_v2_internal(with_threshold(2, 3)),
            public_key_validate_v2_internal(small_order),
        )
    }

    public fun generate_threshold_zero(): u64 {
        let (sk, _) = generate_keys_internal(0, 3);
        sk.length()
    }

    public fun generate_threshold_above_n(): u64 {
        let (sk, _) = generate_keys_internal(4, 3);
        sk.length()
    }

    public fun sign_bad_sk(): vector<u8> {
        sign_internal(x"00", b"m")
    }
}

// RUN: execute 0x1::multi_ed25519::v1_cases
// CHECK: results: true, false, true, true

// RUN: execute 0x1::multi_ed25519::v2_cases
// CHECK: results: true, false, false, false, false

// RUN: execute 0x1::multi_ed25519::generate_threshold_zero
// CHECK: aborted: code 655361 (MultiEd25519 secret key creation failed) in 0x1::multi_ed25519

// RUN: execute 0x1::multi_ed25519::generate_threshold_above_n
// CHECK: aborted: code 655361 (MultiEd25519 secret key creation failed) in 0x1::multi_ed25519

// RUN: execute 0x1::multi_ed25519::sign_bad_sk
// CHECK: aborted: code 655363 (MultiEd25519 secret key deserialization failed) in 0x1::multi_ed25519
