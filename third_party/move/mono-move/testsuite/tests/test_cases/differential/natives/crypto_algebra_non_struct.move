// crypto_algebra natives with a type argument that is not a struct (u64, a vector) abort with
// 786433 (not implemented), as for an unknown marker.

// RUN: publish
module 0x1::crypto_algebra {
    native fun zero_internal<S>(): u64;
    native fun order_internal<G>(): vector<u8>;

    public fun zero_of_u64(): u64 {
        zero_internal<u64>()
    }

    public fun order_of_vector(): vector<u8> {
        order_internal<vector<u8>>()
    }
}

// RUN: execute 0x1::crypto_algebra::zero_of_u64
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::order_of_vector
// CHECK: aborted: code 786433 in 0x1::crypto_algebra
