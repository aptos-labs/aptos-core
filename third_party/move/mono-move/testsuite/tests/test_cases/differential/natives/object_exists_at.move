// Differential test for `object::exists_at`. The module is not in the bundled
// stdlib, so the native is declared here.

// RUN: publish
module 0x1::object {
    // The framework bounds `T` by `key`. The bound is dropped here to reach
    // the native's own guard, which nothing the bound admits can reach.
    public native fun exists_at<T>(object: address): bool;
}
module 0x42::m {
    struct R has key { v: u64 }

    // Publishes `R` at the signer's address, then checks it exists there.
    public fun present(s: signer, a: address): bool {
        move_to(&s, R { v: 7 });
        0x1::object::exists_at<R>(a)
    }

    // No `R` has been published at `a`.
    public fun absent(a: address): bool {
        0x1::object::exists_at<R>(a)
    }

    public fun non_struct(a: address): bool {
        0x1::object::exists_at<u64>(a)
    }
}

// RUN: execute 0x42::m::present --args 0x42, 0x42
// CHECK: results: true

// RUN: execute 0x42::m::absent --args 0x99
// CHECK: results: false

// RUN: execute 0x42::m::non_struct --args 0x99
// CHECK: aborted: code 11 (Object type argument must be a resource (struct) type) in 0x1::object
