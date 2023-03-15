/// Module `algebra` provides structs/functions for doing arithmetic and other common operations
/// on algebraic structures (mostly groups and fields) that are widely used in cryptographic systems.
///
/// Different from existing modules like `ristretto255.move`, the functions here are generic.
/// Typically, each function represent an operation defined for ANY group/field
/// and require some marker type(s) which represents the actual structure(s) to work with.
/// See the test cases in `*_algebra.move` for more examples.
///
/// The generic APIs should allow Move developers to build generic cryptographic schemes on top of them
/// and use the schemes with different underlying algebraic structures by simply changing some type parameters.
/// E.g., Groth16 proof verifier that accepts a generic pairing is now possible.
///
/// Currently supported structures can be found in `algebra_*.move`.
///
/// Below are the operations currently supported.
/// - Serialization/deserialization.
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
/// Note: in `algebra.move` additive group notions are used.
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

    /// Compute `P + Q` for elements `P` and `Q` of a group `G`.
    public fun group_add<G>(element_p: &Element<G>, element_q: &Element<G>): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<G> {
            handle: group_add_internal<G>(element_p.handle, element_q.handle)
        }
    }

    /// Compute `2*P` for an element `P` of a group `G`. Faster and cheaper than `P + P`.
    public fun group_double<G>(element_p: &Element<G>): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<G> {
            handle: group_double_internal<G>(element_p.handle)
        }
    }

    /// Get the fixed generator of a cyclic group `G`.
    public fun group_generator<G>(): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<G> {
            handle: group_generator_internal<G>()
        }
    }

    /// Get the identity of a group `G`.
    public fun group_identity<G>(): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<G> {
            handle: group_identity_internal<G>()
        }
    }

    /// Compute `k[0]*P[0]+...+k[n-1]*P[n-1]`, where
    /// `P[]` are `n` elements of group `G` represented by parameter `elements`, and
    /// `k[]` are `n` elements of the scalarfield `S` of group `G` represented by parameter `scalars`.
    ///
    /// Abort with code 0x010000 if the sizes of `elements` and `scalars` do not match.
    public fun group_multi_scalar_mul<G, S>(elements: &vector<Element<G>>, scalars: &vector<Element<S>>): Element<G> {
        let element_handles = handles_from_elements(elements);
        let scalar_handles = handles_from_elements(scalars);
        Element<G> {
            handle: group_multi_scalar_mul_internal<G, S>(element_handles, scalar_handles)
        }
    }

    fun handles_from_elements<S>(elements: &vector<Element<S>>): vector<u64> {
        let num_elements = std::vector::length(elements);
        let element_handles = std::vector::empty();
        let i = 0;
        while (i < num_elements) {
            std::vector::push_back(&mut element_handles, std::vector::borrow(elements, i).handle);
            i = i + 1;
        };
        element_handles
    }

    /// Compute `-P` for an element `P` of a group `G`.
    public fun group_neg<G>(element_p: &Element<G>): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<G> {
            handle: group_neg_internal<G>(element_p.handle)
        }
    }

    /// Compute `k*P`, where `P` is an element of a group `G` and `k` is an element of the scalar field `S` of group `G`.
    public fun group_scalar_mul<G, S>(element_p: &Element<G>, scalar_k: &Element<S>): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<G> {
            handle: group_scalar_mul_internal<G, S>(element_p.handle, scalar_k.handle)
        }
    }

    /// Compute `P - Q` for elements `P` and `Q` of a group `G`.
    public fun group_sub<G>(element_p: &Element<G>, element_q: &Element<G>): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<G> {
            handle: group_sub_internal<G>(element_p.handle, element_q.handle)
        }

    }

    /// Efficiently compute `e(P[0],Q[0])+...+e(P[n-1],Q[n-1])`,
    /// where `e: (G1,G2) -> (Gt)` is a pre-compiled pairing function from groups `(G1,G2)` to group `Gt`,
    /// `P[]` are `n` elements of group `G1` represented by parameter `g1_elements`, and
    /// `Q[]` are `n` elements of group `G2` represented by parameter `g2_elements`.
    ///
    /// Abort with code 0x010000 if the sizes of `g1_elements` and `g2_elements` do not match.
    public fun multi_pairing<G1,G2,Gt>(g1_elements: &vector<Element<G1>>, g2_elements: &vector<Element<G2>>): Element<Gt> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        let g1_handles = handles_from_elements(g1_elements);
        let g2_handles = handles_from_elements(g2_elements);
        Element<Gt> {
            handle: multi_pairing_internal<G1,G2,Gt>(g1_handles, g2_handles)
        }
    }

    /// Compute a pre-compiled pairing function (a.k.a., bilinear map) on `element_1` and `element_2`.
    /// Return an element in the target group `Gt`.
    public fun pairing<G1,G2,Gt>(element_1: &Element<G1>, element_2: &Element<G2>): Element<Gt> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<Gt> {
            handle: pairing_internal<G1,G2,Gt>(element_1.handle, element_2.handle)
        }
    }

    /// Try deserializing a byte array to an element of an algebraic structure `S` using a given serialization format `F`.
    /// Return none if the deserialization failed.
    public fun deserialize<S, F>(bytes: &vector<u8>): Option<Element<S>> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        let (succeeded, handle) = deserialize_internal<S, F>(bytes);
        if (succeeded) {
            some(Element<S> { handle })
        } else {
            none()
        }
    }

    /// Serialize an element of an algebraic structure `S` to a byte array using a given serialization format `F`.
    public fun serialize<S, F>(element: &Element<S>): vector<u8> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        serialize_internal<S, F>(element.handle)
    }

    /// Get the order of group `G`, a big integer little-endian encoded as a byte array.
    public fun group_order<G>(): vector<u8> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        group_order_internal<G>()
    }

    /// Check if an element `x` is the identity of its group `G`.
    public fun group_is_identity<G>(element_x: &Element<G>): bool {
        group_is_identity_internal<G>(element_x.handle)
    }

    /// Cast an element of a structure `S` to a parent structure `L`.
    public fun upcast<S,L>(element: &Element<S>): Element<L> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element<L> {
            handle: upcast_internal<S,L>(element.handle)
        }
    }

    /// Try casting an element `x` of a structure `L` to a sub-structure `S`.
    /// Return none if `x` is not a member of `S`.
    ///
    /// NOTE: Membership check is performed inside, which can be expensive, depending on the structures `L` and `S`.
    public fun downcast<L,S>(element_x: &Element<L>): Option<Element<S>> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        let (succ, new_handle) = downcast_internal<L,S>(element_x.handle);
        if (succ) {
            some(Element<S> { handle: new_handle })
        } else {
            none()
        }
    }

    /// Hash an arbitrary-length byte array `msg` into structure `S` using the given `suite`.
    /// A unique domain separation tag `dst` of size 255 bytes or shorter is required
    /// for each independent collision-resistent mapping involved in the protocol built atop.
    /// Abort if `dst` is too long.
    public fun hash_to<St, Su>(dst: &vector<u8>, msg: &vector<u8>): Element<St> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        Element {
            handle: hash_to_internal<St, Su>(dst, msg)
        }
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

    native fun deserialize_internal<S, F>(bytes: &vector<u8>): (bool, u64);
    native fun downcast_internal<L,S>(handle: u64): (bool, u64);
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
    native fun group_add_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun group_double_internal<G>(element_handle: u64): u64;
    native fun group_generator_internal<G>(): u64;
    native fun group_identity_internal<G>(): u64;
    native fun group_is_identity_internal<G>(handle: u64): bool;
    native fun group_multi_scalar_mul_internal<G, S>(element_handles: vector<u64>, scalar_handles: vector<u64>): u64;
    native fun group_neg_internal<G>(handle: u64): u64;
    native fun group_order_internal<G>(): vector<u8>;
    native fun group_scalar_mul_internal<G, S>(element_handle: u64, scalar_handle: u64): u64;
    native fun group_sub_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun hash_to_internal<St, Su>(dst: &vector<u8>, bytes: &vector<u8>): u64;
    #[test_only]
    native fun insecure_random_element_internal<G>(): u64;
    native fun multi_pairing_internal<G1,G2,Gt>(g1_handles: vector<u64>, g2_handles: vector<u64>): u64;
    native fun pairing_internal<G1,G2,Gt>(g1_handle: u64, g2_handle: u64): u64;
    native fun serialize_internal<S, F>(h: u64): vector<u8>;
    native fun upcast_internal<S,L>(handle: u64): u64;

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
    struct MysteriousGroup {}

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x0c0000, location = Self)]
    fun test_unknown_group(fx: signer) {
        enable_initial_generic_algebraic_operations(&fx);
        let _ = group_order<MysteriousGroup>();
    }
    // Tests end.
}
