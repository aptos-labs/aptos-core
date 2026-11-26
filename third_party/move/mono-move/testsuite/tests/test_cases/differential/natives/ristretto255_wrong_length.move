// Byte arguments of the wrong length to ristretto255 natives that take a 32-byte scalar (each
// scalar position of the scalar and point natives), 32 bytes or 64 bytes. Each function takes
// the length; the input is 6 followed by zero bytes, so the right length is a passing control
// and one byte fewer or more (more would pass a truncating reader) fails. V1 raises
// INTERNAL_TYPE_ERROR (substring check), MonoMove a native invariant violation.

// RUN: publish
module 0x1::ristretto255 {
    struct Scalar has copy, store, drop {
        data: vector<u8>
    }

    struct RistrettoPoint has drop {
        handle: u64
    }

    native fun scalar_mul_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>;

    native fun scalar_add_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>;

    native fun scalar_sub_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>;

    native fun scalar_neg_internal(a_bytes: vector<u8>): vector<u8>;

    native fun scalar_from_u64_internal(num: u64): vector<u8>;

    native fun scalar_reduced_from_32_bytes_internal(bytes: vector<u8>): vector<u8>;

    native fun scalar_uniform_from_64_bytes_internal(bytes: vector<u8>): vector<u8>;

    native fun point_compress_internal(point: &RistrettoPoint): vector<u8>;

    native fun point_mul_internal(point: &RistrettoPoint, scalar: vector<u8>, in_place: bool): u64;

    native fun basepoint_mul_internal(scalar: vector<u8>): u64;

    native fun basepoint_double_mul_internal(
        point_scalar: vector<u8>, some_point: &RistrettoPoint, basepoint_scalar: vector<u8>
    ): u64;

    native fun new_point_from_64_uniform_bytes_internal(bytes: vector<u8>): u64;

    native fun double_scalar_mul_internal(
        point1: u64, point2: u64, scalar1: vector<u8>, scalar2: vector<u8>
    ): u64;

    native fun multi_scalar_mul_internal<P, S>(points: &vector<P>, scalars: &vector<S>): u64;

    // The byte 6 followed by zeros, `len` bytes in total.
    fun six(len: u64): vector<u8> {
        let bytes = vector[6u8];
        while (bytes.length() < len) {
            bytes.push_back(0);
        };
        bytes
    }

    fun scalar(value: u64): vector<u8> {
        scalar_from_u64_internal(value)
    }

    fun times_b(multiple: u64): RistrettoPoint {
        RistrettoPoint { handle: basepoint_mul_internal(scalar(multiple)) }
    }

    fun compress(handle: u64): vector<u8> {
        point_compress_internal(&RistrettoPoint { handle })
    }

    public fun mul_first(len: u64): vector<u8> {
        scalar_mul_internal(six(len), scalar(7))
    }

    public fun mul_second(len: u64): vector<u8> {
        scalar_mul_internal(scalar(7), six(len))
    }

    public fun add_first(len: u64): vector<u8> {
        scalar_add_internal(six(len), scalar(7))
    }

    public fun add_second(len: u64): vector<u8> {
        scalar_add_internal(scalar(7), six(len))
    }

    public fun sub_first(len: u64): vector<u8> {
        scalar_sub_internal(six(len), scalar(7))
    }

    public fun sub_second(len: u64): vector<u8> {
        scalar_sub_internal(scalar(7), six(len))
    }

    public fun neg(len: u64): vector<u8> {
        scalar_neg_internal(six(len))
    }

    public fun point_mul(len: u64): vector<u8> {
        compress(point_mul_internal(&times_b(1), six(len), false))
    }

    public fun basepoint_mul(len: u64): vector<u8> {
        compress(basepoint_mul_internal(six(len)))
    }

    // a * (2B) + b * B.
    public fun basepoint_double_mul_first(len: u64): vector<u8> {
        compress(basepoint_double_mul_internal(six(len), &times_b(2), scalar(1)))
    }

    public fun basepoint_double_mul_second(len: u64): vector<u8> {
        compress(basepoint_double_mul_internal(scalar(1), &times_b(2), six(len)))
    }

    // scalar1 * B + scalar2 * (2B).
    public fun double_scalar_mul_first(len: u64): vector<u8> {
        let one_b = times_b(1);
        let two_b = times_b(2);
        compress(double_scalar_mul_internal(one_b.handle, two_b.handle, six(len), scalar(1)))
    }

    public fun double_scalar_mul_second(len: u64): vector<u8> {
        let one_b = times_b(1);
        let two_b = times_b(2);
        compress(double_scalar_mul_internal(one_b.handle, two_b.handle, scalar(1), six(len)))
    }

    public fun multi_scalar_mul(len: u64): vector<u8> {
        let points = vector[times_b(2)];
        let scalars = vector[Scalar { data: six(len) }];
        compress(multi_scalar_mul_internal<RistrettoPoint, Scalar>(&points, &scalars))
    }

    public fun reduced_from_32_bytes(len: u64): vector<u8> {
        scalar_reduced_from_32_bytes_internal(six(len))
    }

    public fun uniform_from_64_bytes(len: u64): vector<u8> {
        scalar_uniform_from_64_bytes_internal(six(len))
    }

    public fun point_from_64_uniform_bytes(len: u64): vector<u8> {
        compress(new_point_from_64_uniform_bytes_internal(six(len)))
    }
}

// 6 * 7 == 42.
// RUN: execute 0x1::ristretto255::mul_first --args 32
// CHECK: results: 0x2a00000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::mul_first --args 31
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// RUN: execute 0x1::ristretto255::mul_second --args 32
// CHECK: results: 0x2a00000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::mul_second --args 33
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// 6 + 7 == 13.
// RUN: execute 0x1::ristretto255::add_first --args 32
// CHECK: results: 0x0d00000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::add_first --args 31
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// RUN: execute 0x1::ristretto255::add_second --args 32
// CHECK: results: 0x0d00000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::add_second --args 33
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// 6 - 7 == l - 1.
// RUN: execute 0x1::ristretto255::sub_first --args 32
// CHECK: results: 0xecd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010

// RUN: execute 0x1::ristretto255::sub_first --args 31
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// 7 - 6 == 1.
// RUN: execute 0x1::ristretto255::sub_second --args 32
// CHECK: results: 0x0100000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::sub_second --args 33
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// -6 == l - 6.
// RUN: execute 0x1::ristretto255::neg --args 32
// CHECK: results: 0xe7d3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010

// RUN: execute 0x1::ristretto255::neg --args 31
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// 6B (RFC 9496 test vector B[6]).
// RUN: execute 0x1::ristretto255::point_mul --args 32
// CHECK: results: 0xf64746d3c92b13050ed8d80236a7f0007c3b3f962f5ba793d19a601ebb1df403

// RUN: execute 0x1::ristretto255::point_mul --args 31
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// RUN: execute 0x1::ristretto255::basepoint_mul --args 32
// CHECK: results: 0xf64746d3c92b13050ed8d80236a7f0007c3b3f962f5ba793d19a601ebb1df403

// RUN: execute 0x1::ristretto255::basepoint_mul --args 31
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// 6 * (2B) + B == 13B.
// RUN: execute 0x1::ristretto255::basepoint_double_mul_first --args 32
// CHECK: results: 0xaa52e000df2e16f55fb1032fc33bc42742dad6bd5a8fc0be0167436c5948501f

// RUN: execute 0x1::ristretto255::basepoint_double_mul_first --args 31
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// 2B + 6 * B == 8B.
// RUN: execute 0x1::ristretto255::basepoint_double_mul_second --args 32
// CHECK: results: 0x903293d8f2287ebe10e2374dc1a53e0bc887e592699f02d077d5263cdd55601c

// RUN: execute 0x1::ristretto255::basepoint_double_mul_second --args 33
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// 6 * B + 2B == 8B.
// RUN: execute 0x1::ristretto255::double_scalar_mul_first --args 32
// CHECK: results: 0x903293d8f2287ebe10e2374dc1a53e0bc887e592699f02d077d5263cdd55601c

// RUN: execute 0x1::ristretto255::double_scalar_mul_first --args 31
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// B + 6 * (2B) == 13B.
// RUN: execute 0x1::ristretto255::double_scalar_mul_second --args 32
// CHECK: results: 0xaa52e000df2e16f55fb1032fc33bc42742dad6bd5a8fc0be0167436c5948501f

// RUN: execute 0x1::ristretto255::double_scalar_mul_second --args 33
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// 6 * (2B) == 12B.
// RUN: execute 0x1::ristretto255::multi_scalar_mul --args 32
// CHECK: results: 0xe4549ee16b9aa03099ca208c67adafcafa4c3f3e4e5303de6026e3ca8ff84460

// RUN: execute 0x1::ristretto255::multi_scalar_mul --args 31
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// RUN: execute 0x1::ristretto255::reduced_from_32_bytes --args 32
// CHECK: results: 0x0600000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::reduced_from_32_bytes --args 31
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: expected 32 bytes

// RUN: execute 0x1::ristretto255::uniform_from_64_bytes --args 64
// CHECK: results: 0x0600000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::uniform_from_64_bytes --args 63
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: expected 64 bytes

// RUN: execute 0x1::ristretto255::point_from_64_uniform_bytes --args 64
// CHECK: results: 0xc0b22ba4c9ecd3cc668662f5034e46e83ad120bf4eb6a55f50e49d86914abb28

// RUN: execute 0x1::ristretto255::point_from_64_uniform_bytes --args 65
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") }
// CHECK-V2: error: native function invariant violation: expected 64 bytes
