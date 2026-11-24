// Input checks of the test-only provers on 32-byte scalars, and their order. The single
// prover checks the bit width, then that the value fits in 64 bits (2^64 - 1 passes, 2^64
// aborts with no message). The batch prover checks the bit width, the number of values, equal
// value and blinder counts, then that every value fits in 64 bits. Each abort has a passing
// control differing only in the rejected input; steps with two invalid inputs show which
// check comes first.

// RUN: publish
module 0x1::ristretto255 {
    struct RistrettoPoint has drop {
        handle: u64
    }

    native fun point_decompress_internal(maybe_non_canonical_bytes: vector<u8>): (u64, bool);

    native fun scalar_from_u64_internal(num: u64): vector<u8>;

    native fun scalar_from_u128_internal(num: u128): vector<u8>;

    public fun basepoint(): RistrettoPoint {
        let (handle, _) = point_decompress_internal(
            x"e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76"
        );
        RistrettoPoint { handle }
    }

    public fun hash_base(): RistrettoPoint {
        let (handle, _) = point_decompress_internal(
            x"8c9240b456a9e6dc65c377a1048d745f94a08cdb7f44cbcd7b46f34048871134"
        );
        RistrettoPoint { handle }
    }

    public fun scalar(value: u64): vector<u8> {
        scalar_from_u64_internal(value)
    }

    public fun wide_scalar(value: u128): vector<u8> {
        scalar_from_u128_internal(value)
    }
}

module 0x1::ristretto255_bulletproofs {
    use 0x1::ristretto255::{Self, RistrettoPoint};

    const DST: vector<u8> = b"AptosBulletproofs";
    const BLINDER: vector<u8> = x"e7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";

    native fun prove_range_internal(
        val: vector<u8>,
        blinder: vector<u8>,
        num_bits: u64,
        dst: vector<u8>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint): (vector<u8>, vector<u8>);

    native fun prove_batch_range_internal(
        vals: vector<vector<u8>>,
        rs: vector<vector<u8>>,
        num_bits: u64,
        dst: vector<u8>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint): (vector<u8>, vector<vector<u8>>);

    // Returns the proof and commitment lengths.
    public fun prove(value: u128, num_bits: u64): (u64, u64) {
        let (proof, comm) = prove_range_internal(
            ristretto255::wide_scalar(value), BLINDER, num_bits, DST,
            &ristretto255::basepoint(), &ristretto255::hash_base()
        );
        (proof.length(), comm.length())
    }

    // Values 1..=num_vals, except `wide_value` at `wide_index` (when in range),
    // and `num_rs` blinders. Returns the proof length and commitment count.
    public fun prove_batch(
        num_vals: u64, num_rs: u64, wide_index: u64, wide_value: u128, num_bits: u64
    ): (u64, u64) {
        let vals = vector[];
        let index = 0;
        while (index < num_vals) {
            if (index == wide_index) {
                vals.push_back(ristretto255::wide_scalar(wide_value));
            } else {
                vals.push_back(ristretto255::scalar(index + 1));
            };
            index = index + 1;
        };
        let rs = vector[];
        index = 0;
        while (index < num_rs) {
            rs.push_back(ristretto255::scalar(1000003 * (index + 1)));
            index = index + 1;
        };
        let (proof, comms) = prove_batch_range_internal(
            vals, rs, num_bits, DST, &ristretto255::basepoint(), &ristretto255::hash_base()
        );
        (proof.length(), comms.length())
    }
}

// RUN: execute 0x1::ristretto255_bulletproofs::prove --args 18446744073709551615, 64
// CHECK: results: 672, 32

// RUN: execute 0x1::ristretto255_bulletproofs::prove --args 18446744073709551615, 10
// CHECK: aborted: code 65539 (Range of 10 bits is not supported (must be 8, 16, 32, or 64)) in 0x1::ristretto255_bulletproofs

// RUN: execute 0x1::ristretto255_bulletproofs::prove --args 18446744073709551616, 64
// CHECK: aborted: code 65538 in 0x1::ristretto255_bulletproofs

// RUN: execute 0x1::ristretto255_bulletproofs::prove --args 18446744073709551616, 10
// CHECK: aborted: code 65539 (Range of 10 bits is not supported (must be 8, 16, 32, or 64)) in 0x1::ristretto255_bulletproofs

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch --args 2, 2, 1, 18446744073709551615, 64
// CHECK: results: 736, 2

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch --args 2, 2, 1, 18446744073709551615, 10
// CHECK: aborted: code 65539 (Range of 10 bits is not supported (must be 8, 16, 32, or 64)) in 0x1::ristretto255_bulletproofs

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch --args 3, 3, 9, 0, 64
// CHECK: aborted: code 65540 (Batch size 3 is not supported (must be 1, 2, 4, 8, or 16)) in 0x1::ristretto255_bulletproofs

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch --args 0, 0, 9, 0, 64
// CHECK: aborted: code 65540 (Batch size 0 is not supported (must be 1, 2, 4, 8, or 16)) in 0x1::ristretto255_bulletproofs

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch --args 2, 1, 9, 0, 64
// CHECK: aborted: code 65541 (Number of committed values (2) must equal number of blinding factors (1)) in 0x1::ristretto255_bulletproofs

// Three blinders is not a supported batch size, but only the value count is checked
// against the supported sizes.
// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch --args 2, 3, 9, 0, 64
// CHECK: aborted: code 65541 (Number of committed values (2) must equal number of blinding factors (3)) in 0x1::ristretto255_bulletproofs

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch --args 2, 2, 1, 18446744073709551616, 64
// CHECK: aborted: code 65538 in 0x1::ristretto255_bulletproofs

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch --args 3, 3, 9, 0, 10
// CHECK: aborted: code 65539 (Range of 10 bits is not supported (must be 8, 16, 32, or 64)) in 0x1::ristretto255_bulletproofs

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch --args 3, 2, 9, 0, 64
// CHECK: aborted: code 65540 (Batch size 3 is not supported (must be 1, 2, 4, 8, or 16)) in 0x1::ristretto255_bulletproofs

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch --args 2, 1, 0, 18446744073709551616, 64
// CHECK: aborted: code 65541 (Number of committed values (2) must equal number of blinding factors (1)) in 0x1::ristretto255_bulletproofs
