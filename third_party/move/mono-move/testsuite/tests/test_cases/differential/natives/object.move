// Differential test for `object::create_user_derived_object_address_impl`.

// RUN: publish
module 0x1::object {
    public native fun create_user_derived_object_address_impl(
        source: address,
        derive_from: address,
    ): address;

    public fun derive(): address {
        create_user_derived_object_address_impl(@0xa, @0xb)
    }

    // The memo cache must not change the result: the second (cached) call
    // returns the same address as the first.
    public fun derive_twice_same(): bool {
        create_user_derived_object_address_impl(@0xa, @0xb)
            == create_user_derived_object_address_impl(@0xa, @0xb)
    }
}

// RUN: execute 0x1::object::derive
// CHECK: results: 0xc168433b37d568f2c5cb143f04e177e102d9e40247cefdcb41b8dcc56caa44b0

// RUN: execute 0x1::object::derive_twice_same
// CHECK: results: true
