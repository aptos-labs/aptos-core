/// This module provides generic structs/functions for operations of algebraic structures (e.g. fields and groups),
/// which can be used to build generic cryptographic schemes atop.
/// See `algebra_*.move` for currently implemented algebraic structures.
///
/// Below are the operations currently supported.
/// - Element serialization/deserialization.
/// - Group operations.
///   - Getting group order.
///   - Getting group identity.
///   - Getting group generator.
///   - Addition.
///   - Subtraction.
///   - Negation.
///   - Sclar multiplication.
///   - Efficient multi-sclar multiplication.
///   - Efficient doubling.
///   - Equal-to-identity check.
/// - Field operations.
///   - Getting additive identity.
///   - Getting multiplicative identity.
///   - Conversion from integers.
///   - Addition.
///   - Negation.
///   - Subtraction.
///   - Multiplication.
///   - Inversion.
///   - Division.
///   - Efficient squaring.
///   - Equal-to-additive-identity check.
///   - Equal-to-multiplicative-identity check.
/// - Equality check.
/// - Upcasting/downcasting between structures.
/// - Hash-to-structure.
///
/// Note: in this module additive notions are used for groups.
module aptos_std::algebra {
    use std::option::{Option, some, none};
    use std::features::generic_algebraic_structures_basic_operations_enabled;

    /// This struct represents an element of an algebraic structure `S`.
    struct Element<phantom S> has copy, drop {
        handle: u64
    }

    // Public functions begin.

    /// Check if `x == y` for elements `x` and `y` of an algebraic structure `S`.
    public fun eq<S>(x: &Element<S>, y: &Element<S>): bool {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        eq_internal<S>(x.handle, y.handle)
    }

    /// Convert a u64 to an element of an algebraic structure `S`.
    public fun from_u64<S>(value: u64): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<S> {
            handle: from_u64_internal<S>(value)
        }
    }

    /// Return the additive identity of a field `S`.
    public fun field_zero<S>(): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<S> {
            handle: field_zero_internal<S>()
        }
    }

    /// Return the multiplicative identity of a field `S`.
    public fun field_one<S>(): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<S> {
            handle: field_one_internal<S>()
        }
    }

    /// Compute `-x` for an element `x` of a field `S`.
    public fun field_neg<S>(x: &Element<S>): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<S> {
            handle: field_neg_internal<S>(x.handle)
        }
    }

    /// Compute `x + y` for elements `x` and `y` of a field `S`.
    public fun field_add<S>(x: &Element<S>, y: &Element<S>): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<S> {
            handle: field_add_internal<S>(x.handle, y.handle)
        }
    }

    /// Compute `x - y` for elements `x` and `y` of a field `S`.
    public fun field_sub<S>(x: &Element<S>, y: &Element<S>): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<S> {
            handle: field_sub_internal<S>(x.handle, y.handle)
        }
    }

    /// Compute `x * y` for elements `x` and `y` of a field `S`.
    public fun field_mul<S>(x: &Element<S>, y: &Element<S>): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<S> {
            handle: field_mul_internal<S>(x.handle, y.handle)
        }
    }

    /// Try computing `x / y` for elements `x` and `y` of a field `S`.
    /// Return none if y is the additive identity of field `S`.
    public fun field_div<S>(x: &Element<S>, y: &Element<S>): Option<Element<S>> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        let (succ, handle) = field_div_internal<S>(x.handle, y.handle);
        if (succ) {
            some(Element<S> { handle })
        } else {
            none()
        }
    }

    /// Compute `x^2` for an element `x` of a field `S`.
    ///
    public fun field_sqr<S>(x: &Element<S>): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<S> {
            handle: field_sqr_internal<S>(x.handle)
        }
    }

    /// Try computing `x^(-1)` for an element `x` of a field `S`.
    /// Return none if `x` is the additive identity of field `S`.
    public fun field_inv<S>(x: &Element<S>): Option<Element<S>> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        let (succeeded, handle) = field_inv_internal<S>(x.handle);
        if (succeeded) {
            let scalar = Element<S> { handle };
            some(scalar)
        } else {
            none()
        }
    }

    /// Check if an element `x` is the multiplicative identity of field `S`.
    public fun field_is_one<S>(x: &Element<S>): bool {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        field_is_one_internal<S>(x.handle)
    }

    /// Check if an element `x` is the aditive identity of field `S`.
    public fun field_is_zero<S>(x: &Element<S>): bool {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        field_is_zero_internal<S>(x.handle)
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

    #[test_only]
    /// Generate a random element of an algebraic structure `S`.
    public fun insecure_random_element<S>(): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<S> {
            handle: insecure_random_element_internal<S>()
        }
    }

    // Public functions end.

    // Native functions begin.

    native fun deserialize_internal<G>(format: u64, bytes: &vector<u8>): (bool, u64);
    native fun eq_internal<S>(handle_1: u64, handle_2: u64): bool;
    native fun field_add_internal<F>(handle_1: u64, handle_2: u64): u64;
    native fun field_div_internal<F>(handle_1: u64, handle_2: u64): (bool, u64);
    native fun field_inv_internal<F>(handle: u64): (bool, u64);
    native fun field_is_one_internal<F>(handle: u64): bool;
    native fun field_is_zero_internal<F>(handle: u64): bool;
    native fun field_mul_internal<F>(handle_1: u64, handle_2: u64): u64;
    native fun field_neg_internal<F>(handle: u64): u64;
    native fun field_one_internal<S>(): u64;
    native fun field_sqr_internal<G>(handle: u64): u64;
    native fun field_sub_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun field_zero_internal<S>(): u64;
    native fun from_u64_internal<S>(value: u64): u64;
    #[test_only]
    native fun insecure_random_element_internal<G>(): u64;
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

    // Tests begin.
    #[test_only]
    struct MysteriousField {}

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x0c0000, location = Self)]
    fun test_unknown_group(fx: signer) {
        enable_initial_generic_algebraic_operations(&fx);
        let _ = field_one<MysteriousField>();
    }
    // Tests end.
}
