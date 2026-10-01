// RUN: publish
module 0x1::bls12381_algebra {
    struct Fr {}
    struct Fq12 {}
    struct G1 {}
    struct G2 {}
    struct Gt {}
    struct FormatFrLsb {}
    struct FormatG1Compr {}

    // Not a marker any resolver knows.
    struct Unknown {}
}

// A generic marker: the canonical type tag of `Fr<u8>` carries `<u8>`, so it
// never matches a marker name.
module 0x1::bn254_algebra {
    struct Fr<phantom T> {}
}

module 0x1::crypto_algebra {
    use 0x1::bls12381_algebra::{
        Fq12,
        FormatFrLsb,
        FormatG1Compr,
        Fr,
        G1,
        G2,
        Gt,
        Unknown
    };
    use 0x1::bn254_algebra;

    /// 1 MB / 144 bytes per `G1Projective`.
    const G1_NUM_MAX: u64 = 1048576 / 144;

    native fun double_internal<G>(element_handle: u64): u64;
    native fun downcast_internal<L, S>(handle: u64): (bool, u64);
    native fun from_u64_internal<S>(value: u64): u64;
    native fun multi_pairing_internal<G1, G2, Gt>(
        g1_handles: vector<u64>, g2_handles: vector<u64>
    ): u64;
    native fun multi_scalar_mul_internal<G, S>(
        element_handles: vector<u64>, scalar_handles: vector<u64>
    ): u64;
    native fun one_internal<S>(): u64;
    native fun order_internal<G>(): vector<u8>;
    native fun serialize_internal<S, F>(handle: u64): vector<u8>;
    native fun sqr_internal<G>(handle: u64): u64;
    native fun upcast_internal<S, L>(handle: u64): u64;
    native fun zero_internal<S>(): u64;

    public fun multi_scalar_mul_size_mismatch(): u64 {
        multi_scalar_mul_internal<G1, Fr>(
            vector[one_internal<G1>()],
            vector[from_u64_internal<Fr>(1), from_u64_internal<Fr>(2)]
        )
    }

    public fun multi_pairing_size_mismatch(): u64 {
        multi_pairing_internal<G1, G2, Gt>(
            vector[one_internal<G1>()],
            vector[one_internal<G2>(), one_internal<G2>()]
        )
    }

    // `multi_scalar_mul` has no `Gt` arm even though `scalar_mul` does.
    public fun multi_scalar_mul_on_gt(): u64 {
        multi_scalar_mul_internal<Gt, Fr>(
            vector[one_internal<Gt>()], vector[from_u64_internal<Fr>(1)]
        )
    }

    // The structure check runs first, so this is "not implemented" rather than
    // a size mismatch.
    public fun multi_scalar_mul_on_gt_size_mismatch(): u64 {
        multi_scalar_mul_internal<Gt, Fr>(
            vector[one_internal<Gt>()],
            vector[from_u64_internal<Fr>(1), from_u64_internal<Fr>(2)]
        )
    }

    // `double` covers the groups and `Gt`, not the fields.
    public fun double_on_a_field(): u64 {
        double_internal<Fr>(from_u64_internal<Fr>(7))
    }

    // `sqr` covers the fields, not the groups.
    public fun sqr_on_a_group(): u64 {
        sqr_internal<G1>(one_internal<G1>())
    }

    // A `G1` element has no `Fr` serialization format.
    public fun serialize_with_a_foreign_format(): vector<u8> {
        serialize_internal<G1, FormatFrLsb>(one_internal<G1>())
    }

    // `Fq12` upcasts from `Gt`, not the other way round.
    public fun upcast_in_the_wrong_direction(): u64 {
        upcast_internal<Fq12, Gt>(from_u64_internal<Fq12>(7))
    }

    public fun downcast_between_unrelated_structures(): (bool, u64) {
        downcast_internal<Fr, G1>(from_u64_internal<Fr>(7))
    }

    public fun unknown_marker(): u64 {
        zero_internal<Unknown>()
    }

    // `order` has an arm for every structure, so only an unknown marker can
    // reach its fallback.
    public fun unknown_marker_order(): vector<u8> {
        order_internal<Unknown>()
    }

    // An instantiated marker resolves to nothing, even though `Fr` alone would.
    public fun instantiated_marker(): u64 {
        zero_internal<bn254_algebra::Fr<u8>>()
    }

    // One element short of the limit, so the final `one_internal` still fits.
    public fun element_store_within_limit(): vector<u8> {
        let remaining = G1_NUM_MAX - 1;
        while (remaining > 0) {
            zero_internal<G1>();
            remaining -= 1;
        };
        serialize_internal<G1, FormatG1Compr>(one_internal<G1>())
    }

    public fun element_store_over_limit(): u64 {
        let remaining = G1_NUM_MAX + 1;
        let last = 0;
        while (remaining > 0) {
            last = zero_internal<G1>();
            remaining -= 1;
        };
        last
    }
}

// RUN: execute 0x1::crypto_algebra::multi_scalar_mul_size_mismatch
// CHECK: aborted: code 65538 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::multi_pairing_size_mismatch
// CHECK: aborted: code 65538 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::multi_scalar_mul_on_gt
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::multi_scalar_mul_on_gt_size_mismatch
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::double_on_a_field
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::sqr_on_a_group
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::serialize_with_a_foreign_format
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::upcast_in_the_wrong_direction
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::downcast_between_unrelated_structures
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::unknown_marker
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::unknown_marker_order
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::instantiated_marker
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::element_store_within_limit
// CHECK: results: 0x97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb

// RUN: execute 0x1::crypto_algebra::element_store_over_limit
// CHECK: aborted: code 589827 (Algebra context memory 1048576-byte limit exceeded: currently using 1048464 bytes; was asked for 1048608 bytes) in 0x1::crypto_algebra
