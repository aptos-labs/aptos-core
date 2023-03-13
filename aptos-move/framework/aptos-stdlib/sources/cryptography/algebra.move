/// This module provides generic structs/functions for operations of algebraic structures (e.g. fields and groups),
/// which can be used to build generic cryptographic schemes atop.
/// See `algebra_*.move` for currently implemented algebraic structures.
///
/// Below are the operations currently supported.
/// - Element serialization/deserialization.
/// - Field operations.
///   - Addition.
module aptos_std::algebra {
    use std::option::{Option, some, none};
    use std::features::generic_algebraic_structures_basic_operations_enabled;

    /// This struct represents an element of an algebraic structure `S`.
    struct Element<phantom S> has copy, drop {
        handle: u64
    }

    // Public functions begin.

    /// Compute `x + y` for elements `x` and `y` of a field `S`.
    public fun field_add<S>(x: &Element<S>, y: &Element<S>): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<S> {
            handle: field_add_internal<S>(x.handle, y.handle)
        }
    }

    /// Try deserializing a byte array to an element of an algebraic structure `S` using a given `format`.
    /// Return none if the deserialization failed.
    public fun deserialize<S>(format: u64, bytes: &vector<u8>): Option<Element<S>> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        let (succeeded, handle) = deserialize_internal<S>(format, bytes);
        if (succeeded) {
            some(Element<S> { handle })
        } else {
            none()
        }
    }

    /// Serialize an element of an algebraic structure `S` to a byte array using a given `format`.
    public fun serialize<S>(format: u64, element: &Element<S>): vector<u8> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        serialize_internal<S>(format, element.handle)
    }

    // Public functions end.

    // Native functions begin.

    native fun deserialize_internal<G>(format: u64, bytes: &vector<u8>): (bool, u64);
    native fun field_add_internal<F>(handle_1: u64, handle_2: u64): u64;
    native fun serialize_internal<G>(format: u64, h: u64): vector<u8>;

    // Native functions end.

    // private functions begin.

    fun abort_unless_generic_algebraic_structures_basic_operations_enabled() {
        if (generic_algebraic_structures_basic_operations_enabled()) return;
        abort(std::error::not_implemented(0))
    }

    #[test_only]
    public fun enable_initial_generic_algebraic_operations(fx: &signer) {
        std::features::change_feature_flags(fx, vector[std::features::get_generic_agebraic_structures_basic_operations_feature()], vector[]);
    }

    // Private functions end.
}
