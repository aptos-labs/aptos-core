// Differential test for the ristretto255 point natives and the point-handle store.
//
// Points never cross the Move/native boundary as bytes: a native returns a `u64`
// handle, wrapped into a `RistrettoPoint`. Most checks compute a point two ways
// and compare with `point_equals` (v1 is the oracle); a couple pin known
// compressed encodings.

// RUN: publish
module 0x1::ristretto255 {
    struct Scalar has copy, store, drop {
        data: vector<u8>
    }

    struct RistrettoPoint has drop {
        handle: u64
    }

    native fun scalar_from_u64_internal(num: u64): vector<u8>;

    native fun point_identity_internal(): u64;

    native fun point_is_canonical_internal(bytes: vector<u8>): bool;

    native fun point_decompress_internal(maybe_non_canonical_bytes: vector<u8>): (u64, bool);

    native fun point_clone_internal(point_handle: u64): u64;

    native fun point_compress_internal(point: &RistrettoPoint): vector<u8>;

    native fun point_mul_internal(point: &RistrettoPoint, a: vector<u8>, in_place: bool): u64;

    native fun point_equals(a: &RistrettoPoint, b: &RistrettoPoint): bool;

    native fun point_neg_internal(a: &RistrettoPoint, in_place: bool): u64;

    native fun point_add_internal(a: &RistrettoPoint, b: &RistrettoPoint, in_place: bool): u64;

    native fun point_sub_internal(a: &RistrettoPoint, b: &RistrettoPoint, in_place: bool): u64;

    native fun basepoint_mul_internal(a: vector<u8>): u64;

    native fun basepoint_double_mul_internal(
        a: vector<u8>, some_point: &RistrettoPoint, b: vector<u8>
    ): u64;

    native fun new_point_from_sha512_internal(sha2_512_input: vector<u8>): u64;

    native fun new_point_from_64_uniform_bytes_internal(bytes: vector<u8>): u64;

    native fun double_scalar_mul_internal(
        point1: u64, point2: u64, scalar1: vector<u8>, scalar2: vector<u8>
    ): u64;

    native fun multi_scalar_mul_internal<P, S>(points: &vector<P>, scalars: &vector<S>): u64;

    // `k * basepoint`, as a RistrettoPoint.
    fun basepoint_times(k: u64): RistrettoPoint {
        RistrettoPoint { handle: basepoint_mul_internal(scalar_from_u64_internal(k)) }
    }

    // point_identity compressed is the 32-byte zero encoding.
    public fun identity_compressed(): vector<u8> {
        let id = RistrettoPoint { handle: point_identity_internal() };
        point_compress_internal(&id)
    }

    // basepoint (1 * B) compressed is a fixed known encoding.
    public fun basepoint_compressed(): vector<u8> {
        let b = basepoint_times(1);
        point_compress_internal(&b)
    }

    // (5B + 3B) - 3B == 5B.
    public fun add_sub_roundtrip(): bool {
        let five_b = basepoint_times(5);
        let three_b = basepoint_times(3);
        let sum = RistrettoPoint { handle: point_add_internal(&five_b, &three_b, false) };
        let diff = RistrettoPoint { handle: point_sub_internal(&sum, &three_b, false) };
        let expected = basepoint_times(5);
        point_equals(&diff, &expected)
    }

    // In-place add/sub compute the same result as the out-of-place forms.
    public fun add_sub_in_place(): bool {
        let acc = basepoint_times(5);
        let three_b = basepoint_times(3);
        // acc = acc + 3B (== 8B), reusing acc's handle.
        acc = RistrettoPoint { handle: point_add_internal(&acc, &three_b, true) };
        // acc = acc - 3B (== 5B), reusing acc's handle.
        acc = RistrettoPoint { handle: point_sub_internal(&acc, &three_b, true) };
        let expected = basepoint_times(5);
        point_equals(&acc, &expected)
    }

    // P + (-P) == identity, out-of-place and in-place.
    public fun neg_roundtrip(): bool {
        let p = basepoint_times(7);
        let neg = RistrettoPoint { handle: point_neg_internal(&p, false) };
        let sum = RistrettoPoint { handle: point_add_internal(&p, &neg, false) };
        let id = RistrettoPoint { handle: point_identity_internal() };
        point_equals(&sum, &id)
    }

    public fun neg_in_place(): bool {
        let p = basepoint_times(7);
        let neg = RistrettoPoint { handle: point_neg_internal(&p, true) };
        let expected = RistrettoPoint {
            handle: point_neg_internal(&basepoint_times(7), false)
        };
        point_equals(&neg, &expected)
    }

    // point_mul(B, 5) == 5B, out-of-place and in-place.
    public fun mul_matches_basepoint(): bool {
        let b = basepoint_times(1);
        let p = RistrettoPoint { handle: point_mul_internal(&b, scalar_from_u64_internal(5), false) };
        let expected = basepoint_times(5);
        point_equals(&p, &expected)
    }

    public fun mul_in_place(): bool {
        let b = basepoint_times(1);
        let p = RistrettoPoint { handle: point_mul_internal(&b, scalar_from_u64_internal(5), true) };
        let expected = basepoint_times(5);
        point_equals(&p, &expected)
    }

    // double_scalar_mul(B, B, 2, 3) == 2B + 3B == 5B.
    public fun double_scalar_mul_matches(): bool {
        let p1 = basepoint_times(1);
        let p2 = basepoint_times(1);
        let h = double_scalar_mul_internal(
            p1.handle, p2.handle, scalar_from_u64_internal(2), scalar_from_u64_internal(3)
        );
        let result = RistrettoPoint { handle: h };
        let expected = basepoint_times(5);
        point_equals(&result, &expected)
    }

    // basepoint_double_mul(2, B, 3) == 2*B + 3*B == 5B.
    public fun basepoint_double_mul_matches(): bool {
        let b = basepoint_times(1);
        let h = basepoint_double_mul_internal(
            scalar_from_u64_internal(2), &b, scalar_from_u64_internal(3)
        );
        let result = RistrettoPoint { handle: h };
        let expected = basepoint_times(5);
        point_equals(&result, &expected)
    }

    // multi_scalar_mul([B, B], [2, 3]) == 2B + 3B == 5B.
    public fun multi_scalar_mul_matches(): bool {
        let points = vector[basepoint_times(1), basepoint_times(1)];
        let scalars = vector[
            Scalar { data: scalar_from_u64_internal(2) },
            Scalar { data: scalar_from_u64_internal(3) },
        ];
        let result = RistrettoPoint {
            handle: multi_scalar_mul_internal<RistrettoPoint, Scalar>(&points, &scalars)
        };
        let expected = basepoint_times(5);
        point_equals(&result, &expected)
    }

    // A cloned handle refers to an equal point.
    public fun clone_equal(): bool {
        let b = basepoint_times(3);
        let c = RistrettoPoint { handle: point_clone_internal(b.handle) };
        point_equals(&b, &c)
    }

    // decompress(compress(B)) == B, and reports success.
    public fun compress_decompress_roundtrip(): bool {
        let b = basepoint_times(1);
        let bytes = point_compress_internal(&b);
        let (handle, ok) = point_decompress_internal(bytes);
        let decompressed = RistrettoPoint { handle };
        ok && point_equals(&decompressed, &b)
    }

    // A valid compressed point is canonical.
    public fun is_canonical_true(): bool {
        let b = basepoint_times(1);
        point_is_canonical_internal(point_compress_internal(&b))
    }

    // All-ones 32 bytes is not a canonical point encoding.
    public fun is_canonical_false(): bool {
        point_is_canonical_internal(
            x"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        )
    }

    // Decompressing invalid bytes reports failure.
    public fun decompress_invalid_ok(): bool {
        let (_handle, ok) = point_decompress_internal(
            x"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        );
        ok
    }

    // Distinct points are not equal.
    public fun equals_false(): bool {
        let b = basepoint_times(1);
        let two_b = basepoint_times(2);
        point_equals(&b, &two_b)
    }

    // new_point_from_sha512 and new_point_from_64_uniform_bytes just need to run
    // and agree across VMs; compare their compressed encodings for equality of
    // outputs by round-tripping through decompress.
    public fun from_sha512_roundtrips(): bool {
        let p = RistrettoPoint { handle: new_point_from_sha512_internal(b"ristretto255 point test") };
        let bytes = point_compress_internal(&p);
        let (handle, ok) = point_decompress_internal(bytes);
        let q = RistrettoPoint { handle };
        ok && point_equals(&p, &q)
    }

    public fun from_64_uniform_roundtrips(): bool {
        let p = RistrettoPoint {
            handle: new_point_from_64_uniform_bytes_internal(
                x"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f",
            )
        };
        let bytes = point_compress_internal(&p);
        let (handle, ok) = point_decompress_internal(bytes);
        let q = RistrettoPoint { handle };
        ok && point_equals(&p, &q)
    }
}

// RUN: execute 0x1::ristretto255::identity_compressed
// CHECK: results: 0x0000000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::basepoint_compressed
// CHECK: results: 0xe2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76

// RUN: execute 0x1::ristretto255::add_sub_roundtrip
// CHECK: results: true

// RUN: execute 0x1::ristretto255::add_sub_in_place
// CHECK: results: true

// RUN: execute 0x1::ristretto255::neg_roundtrip
// CHECK: results: true

// RUN: execute 0x1::ristretto255::neg_in_place
// CHECK: results: true

// RUN: execute 0x1::ristretto255::mul_matches_basepoint
// CHECK: results: true

// RUN: execute 0x1::ristretto255::mul_in_place
// CHECK: results: true

// RUN: execute 0x1::ristretto255::double_scalar_mul_matches
// CHECK: results: true

// RUN: execute 0x1::ristretto255::basepoint_double_mul_matches
// CHECK: results: true

// RUN: execute 0x1::ristretto255::multi_scalar_mul_matches
// CHECK: results: true

// RUN: execute 0x1::ristretto255::clone_equal
// CHECK: results: true

// RUN: execute 0x1::ristretto255::compress_decompress_roundtrip
// CHECK: results: true

// RUN: execute 0x1::ristretto255::is_canonical_true
// CHECK: results: true

// RUN: execute 0x1::ristretto255::is_canonical_false
// CHECK: results: false

// RUN: execute 0x1::ristretto255::decompress_invalid_ok
// CHECK: results: false

// RUN: execute 0x1::ristretto255::equals_false
// CHECK: results: false

// RUN: execute 0x1::ristretto255::from_sha512_roundtrips
// CHECK: results: true

// RUN: execute 0x1::ristretto255::from_64_uniform_roundtrips
// CHECK: results: true
