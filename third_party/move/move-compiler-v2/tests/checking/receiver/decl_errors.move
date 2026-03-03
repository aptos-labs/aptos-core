module 0x42::m {
    use 0x42::n::T;

    struct S { x: u64 }
    struct G<T, R> { x: T, y: R }

    // Ok
    fun receiver(self: S) {}

    // Error
    fun receiver_for_external_type(self: T) {}

    // Error
    fun receiver_for_primitive(self: &u64) {}

    // Error
    fun receiver_for_external_vector(self: vector<u64>) {}

    // Error
    fun receiver_partial_instantiated<T>(self: G<u64, T>) {}

    // Error
    fun receiver_non_linear_instantiated<T>(self: G<T, T>) {}

    // Error: external module cannot add receiver functions on signer
    fun receiver_for_signer(self: &signer): address { abort 0 }

    // Error: cannot overload address_of on signer from outside module
    fun address_of(self: &signer): address { abort 0 }

}

module 0x42::n {
    struct T { x: u64 }
}
