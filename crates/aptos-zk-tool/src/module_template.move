/// Generic implementation of Groth16 (proof verification) as defined in https://eprint.iacr.org/2016/260.pdf, Section 3.2.
/// Actual proof verifiers can be constructed using the pairings supported in the generic algebra module.
/// See the test cases in this module for an example of constructing with BLS12-381 curves.
module self::{{name}} {
    use aptos_std::crypto_algebra::{Element, from_u64, multi_scalar_mul, eq, multi_pairing, upcast, pairing, add, zero, deserialize};
    use aptos_std::bls12381_algebra::{G1, FormatG1Uncompr, G2, FormatG2Uncompr, Gt, Fr, FormatFrLsb};
    use std::option;
    use std::vector;

    /// Proof verification as specified in the original paper,
    /// with the following input (in the original paper notations).
    /// - Verification key: $\left([\alpha]_1, [\beta]_2, [\gamma]_2, [\delta]_2, \left\\{ \left[ \frac{\beta \cdot u_i(x) + \alpha \cdot v_i(x) + w_i(x)}{\gamma} \right]_1 \right\\}\_{i=0}^l \right)$.
    /// - Public inputs: $\\{a_i\\}_{i=1}^l$.
    /// - Proof $\left( \left[ A \right]_1, \left[ B \right]_2, \left[ C \right]_1 \right)$.
    fun verify_proof_impl(
        vk_alpha_g1: &Element<G1>,
        vk_beta_g2: &Element<G2>,
        vk_gamma_g2: &Element<G2>,
        vk_delta_g2: &Element<G2>,
        vk_uvw_gamma_g1: &vector<Element<G1>>,
        public_inputs: &vector<Element<Fr>>,
        proof_a: &Element<G1>,
        proof_b: &Element<G2>,
        proof_c: &Element<G1>,
    ): bool {
        let left = pairing<G1,G2,Gt>(proof_a, proof_b);
        let scalars = vector[from_u64<Fr>(1)];
        std::vector::append(&mut scalars, *public_inputs);
        let right = zero<Gt>();
        let right = add(&right, &pairing<G1,G2,Gt>(vk_alpha_g1, vk_beta_g2));
        let right = add(&right, &pairing(&multi_scalar_mul(vk_uvw_gamma_g1, &scalars), vk_gamma_g2));
        let right = add(&right, &pairing(proof_c, vk_delta_g2));
        eq(&left, &right)
    }

    public fun verify_proof_with_elements(
        public_inputs: &vector<Element<Fr>>,
        proof_a: &Element<G1>,
        proof_b: &Element<G2>,
        proof_c: &Element<G1>,
    ): bool {
        let vk_alpha_g1_bytes = {{ vk_alpha_g1_bytes }};
        let vk_beta_g2_bytes = {{ vk_beta_g2_bytes }};
        let vk_gamma_g2_bytes = {{ vk_gamma_g2_bytes }};
        let vk_delta_g2_bytes = {{ vk_delta_g2_bytes }};

        let vk_alpha_g1 = deserialize<G1, FormatG1Uncompr>(&vk_alpha_g1_bytes);
        let vk_beta_g2 = deserialize<G2, FormatG2Uncompr>(&vk_beta_g2_bytes);
        let vk_gamma_g2 = deserialize<G2, FormatG2Uncompr>(&vk_gamma_g2_bytes);
        let vk_delta_g2 = deserialize<G2, FormatG2Uncompr>(&vk_delta_g2_bytes);

        let vk_uvw_gamma_g1 = vector::empty<Element<G1>>();
        {{#each vk_uvw_gamma_g1}}
        let vk_uvw_gamma_g1_bytes = {{ this }};
        let vk_uvw_gamma_g1_bytes_opt = deserialize<G1, FormatG1Uncompr>(&vk_uvw_gamma_g1_bytes);
        vector::push_back<Element<G1>>(&mut vk_uvw_gamma_g1, option::destroy_some<Element<G1>>(vk_uvw_gamma_g1_bytes_opt));
        {{/each}}

        verify_proof_impl(
            option::borrow<Element<G1>>(&vk_alpha_g1),
            option::borrow<Element<G2>>(&vk_beta_g2),
            option::borrow<Element<G2>>(&vk_gamma_g2),
            option::borrow<Element<G2>>(&vk_delta_g2),
            &vk_uvw_gamma_g1,
            public_inputs,
            proof_a,
            proof_b,
            proof_c,
        )
    }

    #[view]
    public fun verify_proof(
        public_inputs: vector<vector<u8>>,
        proof_a_bytes: vector<u8>,
        proof_b_bytes: vector<u8>,
        proof_c_bytes: vector<u8>,
    ): bool {
        let public_inputs_elems = vector::map_ref(&public_inputs, |public_input| {
            let proof = deserialize<Fr, FormatFrLsb>(public_input);
            option::destroy_some<Element<Fr>>(proof)
        });
        let proof_a = deserialize<G1, FormatG1Uncompr>(&proof_a_bytes);
        let proof_b = deserialize<G2, FormatG2Uncompr>(&proof_b_bytes);
        let proof_c = deserialize<G1, FormatG1Uncompr>(&proof_c_bytes);

        verify_proof_with_elements(
            &public_inputs_elems,
            option::borrow<Element<G1>>(&proof_a),
            option::borrow<Element<G2>>(&proof_b),
            option::borrow<Element<G1>>(&proof_c),
        )
    }
}
