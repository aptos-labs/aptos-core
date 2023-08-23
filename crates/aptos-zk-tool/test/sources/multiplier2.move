/// Generic implementation of Groth16 (proof verification) as defined in https://eprint.iacr.org/2016/260.pdf, Section 3.2.
/// Actual proof verifiers can be constructed using the pairings supported in the generic algebra module.
/// See the test cases in this module for an example of constructing with BLS12-381 curves.
module self::multiplier2 {
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
        let vk_alpha_g1_bytes = vector[4,143,62,21,236,129,75,158,101,78,217,130,61,226,175,226,62,204,122,175,51,146,94,74,99,25,0,88,1,126,238,79,35,13,196,0,198,103,113,97,117,14,202,94,153,21,79,131,2,170,204,21,31,245,228,28,20,248,189,19,84,6,142,143,36,29,202,243,164,98,236,26,250,202,154,39,150,69,239,68,199,237,133,67,163,255,142,81,105,160,67,9,30,104,123,234,];
        let vk_beta_g2_bytes = vector[12,202,136,70,63,192,20,184,147,252,161,172,95,212,229,114,77,60,51,38,7,122,51,75,84,77,160,134,245,215,83,33,239,106,121,109,191,64,102,49,251,29,52,65,59,154,53,47,8,141,203,51,186,246,165,107,166,5,233,82,86,102,36,106,221,76,212,241,57,235,211,94,155,194,54,103,188,74,200,189,33,115,124,57,97,6,109,230,134,16,82,76,187,218,139,76,7,142,110,158,202,58,141,133,239,20,31,38,85,153,15,230,170,179,62,224,170,18,25,40,50,164,23,117,55,75,240,212,110,121,140,166,99,39,85,24,201,104,110,220,198,72,147,15,10,81,36,181,3,195,151,74,60,104,169,197,89,60,61,189,130,189,100,146,45,17,227,21,151,146,217,13,204,54,234,12,101,236,209,146,241,3,157,17,23,48,56,140,174,82,181,206,];
        let vk_gamma_g2_bytes = vector[5,102,107,183,125,132,152,191,141,57,13,139,207,90,230,145,51,1,236,94,234,87,150,205,152,76,154,196,148,16,30,78,231,56,96,93,63,128,152,18,136,187,207,115,56,156,165,224,7,94,246,130,167,2,248,24,171,68,48,223,154,225,79,232,154,62,175,120,114,124,83,187,210,190,156,134,111,156,71,150,28,207,207,209,130,211,105,56,243,173,242,67,100,199,158,29,1,115,197,245,233,123,178,55,15,168,165,10,50,181,113,242,196,62,17,50,113,53,113,190,98,50,140,227,179,49,246,8,201,194,129,227,162,131,90,177,173,155,109,167,31,218,205,54,4,126,9,86,52,134,109,220,60,135,159,122,35,236,199,104,2,150,251,245,116,26,140,174,19,213,33,151,235,7,165,108,145,241,152,172,118,16,62,48,102,147,172,54,143,59,13,224,];
        let vk_delta_g2_bytes = vector[11,235,169,208,247,85,149,152,240,127,105,27,105,93,108,30,187,81,205,219,166,142,197,114,9,247,12,217,142,155,186,22,118,64,98,176,90,144,41,52,203,73,145,76,225,98,84,44,10,234,161,1,149,153,182,185,212,81,146,46,236,194,138,121,149,44,191,41,73,178,30,168,129,170,130,122,193,135,59,213,145,248,94,85,13,142,245,92,221,34,124,54,163,151,166,127,21,49,202,29,166,64,133,6,234,136,217,139,239,169,109,92,33,48,90,97,246,154,231,28,104,70,18,3,197,246,144,59,121,78,250,47,190,159,223,180,242,50,148,86,20,37,206,109,2,54,222,129,26,117,87,177,225,199,151,39,22,110,167,109,163,13,119,43,255,6,52,113,211,230,210,120,96,76,148,17,69,83,226,29,5,205,142,248,24,124,210,161,215,147,125,48,];

        let vk_alpha_g1 = deserialize<G1, FormatG1Uncompr>(&vk_alpha_g1_bytes);
        let vk_beta_g2 = deserialize<G2, FormatG2Uncompr>(&vk_beta_g2_bytes);
        let vk_gamma_g2 = deserialize<G2, FormatG2Uncompr>(&vk_gamma_g2_bytes);
        let vk_delta_g2 = deserialize<G2, FormatG2Uncompr>(&vk_delta_g2_bytes);

        let vk_uvw_gamma_g1 = vector::empty<Element<G1>>();
        let vk_uvw_gamma_g1_bytes = vector[18,136,201,163,74,220,196,4,18,126,127,150,7,59,70,142,28,103,12,230,137,165,190,30,8,200,94,96,100,178,49,173,144,113,13,112,174,140,109,129,240,74,159,73,150,44,53,54,14,8,58,230,1,139,255,240,31,176,144,134,111,127,69,148,62,224,208,186,94,26,136,134,114,242,47,223,89,103,132,225,135,86,1,178,242,27,226,249,179,89,40,217,3,52,113,163,];
        let vk_uvw_gamma_g1_bytes_opt = deserialize<G1, FormatG1Uncompr>(&vk_uvw_gamma_g1_bytes);
        vector::push_back<Element<G1>>(&mut vk_uvw_gamma_g1, option::destroy_some<Element<G1>>(vk_uvw_gamma_g1_bytes_opt));
        let vk_uvw_gamma_g1_bytes = vector[24,207,219,56,170,134,58,5,205,236,198,135,92,196,226,20,143,169,4,17,229,132,51,243,154,71,138,219,165,184,118,125,13,86,235,161,155,58,139,88,57,7,218,232,151,189,37,24,5,6,234,103,245,231,139,239,197,184,199,11,88,242,35,196,139,7,40,162,130,250,26,214,167,66,146,54,98,247,132,116,155,65,39,161,25,180,222,13,49,98,159,101,72,118,227,143,];
        let vk_uvw_gamma_g1_bytes_opt = deserialize<G1, FormatG1Uncompr>(&vk_uvw_gamma_g1_bytes);
        vector::push_back<Element<G1>>(&mut vk_uvw_gamma_g1, option::destroy_some<Element<G1>>(vk_uvw_gamma_g1_bytes_opt));

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
