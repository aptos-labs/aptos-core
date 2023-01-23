module aptos_std::groups {

    // Prime-order groups on BLS12-381 curves.
    struct BLS12_381_G1 {}
    struct BLS12_381_G2 {}
    struct BLS12_381_Gt {}

    // Ristretto255 group on Curve25519.
    struct Ristretto255 {}

    /// Check if `P == Q` for group elements `P` and `Q`.
    public fun add<G>(element_p: &Element<G>, element_q: &Element<G>): bool {
        add_internal<G>(element_p.handle, element_q.handle)
    }

    /// This struct represents an element of the group represented by the type argument `G`.
    struct Element<phantom G> has copy, drop {
        handle: u64
    }

    /// Perform a pairing.
    public fun pairing<G1,G2,Gt>(element_1: &Element<G1>, element_2: &Element<G2>): Element<Gt> {
        Element<Gt> {
            handle: pairing_product_internal<G1,G2,Gt>(vector[element_1.handle], vector[element_2.handle])
        }
    }

    /// Check if `P == Q` for group elements `P` and `Q`.
    public fun eq<G>(element_p: &Element<G>, element_q: &Element<G>): bool {
        eq_internal<G>(element_p.handle, element_q.handle)
    }

    /// Get a generator of group `G`.
    public fun generator<G>(): Element<G> {
        Element<G> {
            handle: generator_internal<G>(),
        }
    }

    // Native functions.
    native fun add_internal<G>(handle_1: u64, handle_2: u64): bool;
    native fun eq_internal<G>(handle_1: u64, handle_2: u64): bool;
    native fun generator_internal<G>(): u64;
    native fun pairing_product_internal<G1,G2,Gt>(g1_handles: vector<u64>, g2_handles: vector<u64>): u64;

    #[test]
    fun test_bls12_381_pairing() {
        let g1 = generator<BLS12_381_G1>();
        let g2 = generator<BLS12_381_G2>();
        let pairing_result = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(&g1, &g2);
        let gt = generator<BLS12_381_Gt>();
        assert!(eq(&gt, &pairing_result), 1);
    }
}
