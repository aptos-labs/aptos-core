// Differential test for `object::exists_at`. The module is not in the bundled
// stdlib, so the native is declared here.

// RUN: publish
module 0x1::object {
    public native fun exists_at<T: key>(object: address): bool;
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
}

// RUN: execute 0x42::m::present --args 0x42, 0x42
// CHECK: results: true

// RUN: execute 0x42::m::absent --args 0x99
// CHECK: results: false
