// In-place point add and sub with the same handle as both operands abort with 655363. These
// steps run on MonoMove only, since V1's debug build asserts on this input. Each has a control
// on both VMs with two distinct handles holding the same point, so the abort depends on the
// handle, not on the point's value.

// RUN: publish
module 0x1::ristretto255 {
    struct RistrettoPoint has drop {
        handle: u64
    }

    native fun scalar_from_u64_internal(num: u64): vector<u8>;

    native fun basepoint_mul_internal(scalar: vector<u8>): u64;

    native fun point_compress_internal(point: &RistrettoPoint): vector<u8>;

    native fun point_add_internal(
        left: &RistrettoPoint, right: &RistrettoPoint, in_place: bool
    ): u64;

    native fun point_sub_internal(
        left: &RistrettoPoint, right: &RistrettoPoint, in_place: bool
    ): u64;

    fun three_b(): RistrettoPoint {
        RistrettoPoint { handle: basepoint_mul_internal(scalar_from_u64_internal(3)) }
    }

    public fun add_same_handle(): u64 {
        let point = three_b();
        point_add_internal(&point, &point, true)
    }

    // 3B + 3B == 6B, written into the first handle.
    public fun add_distinct_handles(): (bool, vector<u8>) {
        let dest = three_b();
        let other = three_b();
        let returned = point_add_internal(&dest, &other, true);
        (returned == dest.handle, point_compress_internal(&dest))
    }

    public fun sub_same_handle(): u64 {
        let point = three_b();
        point_sub_internal(&point, &point, true)
    }

    // 3B - 3B == identity, written into the first handle.
    public fun sub_distinct_handles(): (bool, vector<u8>) {
        let dest = three_b();
        let other = three_b();
        let returned = point_sub_internal(&dest, &other, true);
        (returned == dest.handle, point_compress_internal(&dest))
    }
}

// RUN: execute 0x1::ristretto255::add_same_handle
// CHECK-V2: aborted: code 655363 (Duplicate Ristretto point handle) in 0x1::ristretto255

// RUN: execute 0x1::ristretto255::add_distinct_handles
// CHECK: results: true, 0xf64746d3c92b13050ed8d80236a7f0007c3b3f962f5ba793d19a601ebb1df403

// RUN: execute 0x1::ristretto255::sub_same_handle
// CHECK-V2: aborted: code 655363 (Duplicate Ristretto point handle) in 0x1::ristretto255

// RUN: execute 0x1::ristretto255::sub_distinct_handles
// CHECK: results: true, 0x0000000000000000000000000000000000000000000000000000000000000000
