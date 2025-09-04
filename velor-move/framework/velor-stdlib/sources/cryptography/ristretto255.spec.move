spec velor_std::ristretto255 {
    spec point_equals {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec new_point_from_sha512_internal {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec new_point_from_64_uniform_bytes_internal {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec point_is_canonical_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_point_is_canonical_internal(bytes);
    }

    spec point_identity_internal {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec point_decompress_internal {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec point_compress_internal {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec point_mul_internal {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec basepoint_mul_internal {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec basepoint_double_mul_internal {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec double_scalar_mul_internal {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec point_add_internal {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec point_clone_internal {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec point_sub_internal {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec point_neg_internal {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec multi_scalar_mul_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_multi_scalar_mul_internal<P, S>(points, scalars);
    }

    spec scalar_is_canonical_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_scalar_is_canonical_internal(s);
    }

    spec scalar_from_u64_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_scalar_from_u64_internal(num);
    }

    spec scalar_from_u128_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_scalar_from_u128_internal(num);
    }

    spec scalar_reduced_from_32_bytes_internal {
        pragma opaque;
        ensures result == spec_scalar_reduced_from_32_bytes_internal(bytes);
    }

    spec scalar_uniform_from_64_bytes_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_scalar_uniform_from_64_bytes_internal(bytes);
    }

    spec scalar_invert_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_scalar_invert_internal(bytes);
    }

    spec double_scalar_mul {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec multi_scalar_mul {
        aborts_if len(points) == 0;
        aborts_if len(scalars) == 0;
        aborts_if len(points) != len(scalars);
        ensures result.handle == spec_multi_scalar_mul_internal(points, scalars);
    }

    spec new_scalar_from_bytes {
        aborts_if false;
        ensures spec_scalar_is_canonical_internal(bytes) ==> (std::option::spec_is_some(result)
            && std::option::spec_borrow(result).data == bytes);
        ensures !spec_scalar_is_canonical_internal(bytes) ==> std::option::spec_is_none(result);
    }

    spec scalar_from_sha512_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_scalar_from_sha512_internal(sha2_512_input);
    }

    spec new_scalar_from_sha2_512 {
        aborts_if false;
        ensures result.data == spec_scalar_from_sha512_internal(sha2_512_input);
    }

    spec new_scalar_from_u8 {
        aborts_if false;
        ensures result.data[0] == byte;
        ensures forall i in 1..len(result.data): result.data[i] == 0;
    }

    spec new_scalar_from_u32 {
        aborts_if false;
        ensures result.data == spec_scalar_from_u64_internal(four_bytes);
    }

    spec new_scalar_from_u64 {
        aborts_if false;
        ensures result.data == spec_scalar_from_u64_internal(eight_bytes);
    }

    spec new_scalar_from_u128 {
        aborts_if false;
        ensures result.data == spec_scalar_from_u128_internal(sixteen_bytes);
    }

    spec new_scalar_reduced_from_32_bytes {
        ensures len(bytes) != 32 ==> std::option::spec_is_none(result);
        ensures len(bytes) == 32 ==> std::option::spec_borrow(result).data == spec_scalar_reduced_from_32_bytes_internal(bytes);
    }

    spec new_scalar_uniform_from_64_bytes {
        ensures len(bytes) != 64 ==> std::option::spec_is_none(result);
        ensures len(bytes) == 64 ==> std::option::spec_borrow(result).data == spec_scalar_uniform_from_64_bytes_internal(bytes);
    }

    spec scalar_zero {
        ensures spec_scalar_is_zero(result);
    }

    spec scalar_is_zero {
        ensures result == spec_scalar_is_zero(s);
    }

    spec scalar_one {
        ensures spec_scalar_is_one(result);
    }

    spec scalar_is_one {
        ensures result == spec_scalar_is_one(s);
    }

    spec scalar_equals {
        aborts_if false;
        ensures result == (lhs.data == rhs.data);
    }

    spec scalar_invert {
        aborts_if false;
        ensures spec_scalar_is_zero(s) ==> std::option::spec_is_none(result);
        ensures !spec_scalar_is_zero(s) ==> (std::option::spec_is_some(result) && std::option::spec_borrow(result).data == spec_scalar_invert_internal(s.data));
    }

    spec scalar_mul_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_scalar_mul_internal(a_bytes, b_bytes);
    }

    spec scalar_add_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_scalar_add_internal(a_bytes, b_bytes);
    }

    spec scalar_sub {
        aborts_if false;
        ensures result.data == spec_scalar_sub_internal(a.data, b.data);
    }

    spec scalar_sub_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_scalar_sub_internal(a_bytes, b_bytes);
    }

    spec scalar_neg {
        pragma opaque;
        aborts_if false;
        ensures result.data == spec_scalar_neg_internal(a.data);
    }

    spec scalar_neg_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_scalar_neg_internal(a_bytes);
    }

    spec scalar_neg_assign {
        aborts_if false;
        ensures a.data == spec_scalar_neg_internal(old(a).data);
    }

    spec scalar_add {
        aborts_if false;
        ensures result.data == spec_scalar_add_internal(a.data, b.data);
    }

    spec scalar_add_assign {
        aborts_if false;
        ensures a.data == spec_scalar_add_internal(old(a).data, b.data);
    }

    spec scalar_mul {
        aborts_if false;
        ensures result.data == spec_scalar_mul_internal(a.data, b.data);
    }

    spec scalar_mul_assign {
        aborts_if false;
        ensures a.data == spec_scalar_mul_internal(old(a).data, b.data);
    }

    spec scalar_sub_assign {
        aborts_if false;
        ensures a.data == spec_scalar_sub_internal(old(a).data, b.data);
    }

    spec scalar_to_bytes {
        aborts_if false;
        ensures result == s.data;
    }

    /// # Helper functions

    spec fun spec_scalar_is_zero(s: Scalar): bool {
        s.data == x"0000000000000000000000000000000000000000000000000000000000000000"
    }

    spec fun spec_scalar_is_one(s: Scalar): bool {
        s.data == x"0100000000000000000000000000000000000000000000000000000000000000"
    }

    spec fun spec_point_is_canonical_internal(bytes: vector<u8>): bool;

    spec fun spec_double_scalar_mul_internal(point1: u64, point2: u64, scalar1: vector<u8>, scalar2: vector<u8>): u64;

    spec fun spec_multi_scalar_mul_internal<P, S>(points: vector<P>, scalars: vector<S>): u64;

    spec fun spec_scalar_is_canonical_internal(s: vector<u8>): bool;

    spec fun spec_scalar_from_u64_internal(num: u64): vector<u8>;

    spec fun spec_scalar_from_u128_internal(num: u128): vector<u8>;

    spec fun spec_scalar_reduced_from_32_bytes_internal(bytes: vector<u8>): vector<u8>;

    spec fun spec_scalar_uniform_from_64_bytes_internal(bytes: vector<u8>): vector<u8>;

    spec fun spec_scalar_invert_internal(bytes: vector<u8>): vector<u8>;

    spec fun spec_scalar_from_sha512_internal(sha2_512_input: vector<u8>): vector<u8>;

    spec fun spec_scalar_mul_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>;

    spec fun spec_scalar_add_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>;

    spec fun spec_scalar_sub_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>;

    spec fun spec_scalar_neg_internal(a_bytes: vector<u8>): vector<u8>;

}
