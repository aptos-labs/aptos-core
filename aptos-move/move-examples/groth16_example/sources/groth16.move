/// Generic implementation of Groth16 (proof verification) as defined in https://eprint.iacr.org/2016/260.pdf, Section 3.2.
/// Actual proof verifiers can be constructed using the pairings supported in the generic algebra module.
/// See the test cases in this module for an example of constructing with BLS12-381 curves.
///
/// **WARNING:** This code has NOT been audited. If using it in a production system, proceed at your own risk.
module groth16_example::groth16 {
    use aptos_std::crypto_algebra::{Element, from_u64, multi_scalar_mul, eq, multi_pairing, upcast, pairing, add, zero};

    /// Proof verification as specified in the original paper,
    /// with the following input (in the original paper notations).
    /// - Verification key: $\left([\alpha]_1, [\beta]_2, [\gamma]_2, [\delta]_2, \left\\{ \left[ \frac{\beta \cdot u_i(x) + \alpha \cdot v_i(x) + w_i(x)}{\gamma} \right]_1 \right\\}\_{i=0}^l \right)$.
    /// - Public inputs: $\\{a_i\\}_{i=1}^l$.
    /// - Proof $\left( \left[ A \right]_1, \left[ B \right]_2, \left[ C \right]_1 \right)$.
    public fun verify_proof<G1,G2,Gt,S>(
        vk_alpha_g1: &Element<G1>,
        vk_beta_g2: &Element<G2>,
        vk_gamma_g2: &Element<G2>,
        vk_delta_g2: &Element<G2>,
        vk_uvw_gamma_g1: &vector<Element<G1>>,
        public_inputs: &vector<Element<S>>,
        proof_a: &Element<G1>,
        proof_b: &Element<G2>,
        proof_c: &Element<G1>,
    ): bool {
        let left = pairing<G1,G2,Gt>(proof_a, proof_b);
        let scalars = vector[from_u64<S>(1)];
        std::vector::append(&mut scalars, *public_inputs);
        let right = zero<Gt>();
        let right = add(&right, &pairing<G1,G2,Gt>(vk_alpha_g1, vk_beta_g2));
        let right = add(&right, &pairing(&multi_scalar_mul(vk_uvw_gamma_g1, &scalars), vk_gamma_g2));
        let right = add(&right, &pairing(proof_c, vk_delta_g2));
        eq(&left, &right)
    }

    /// Modified proof verification which is optimized for low verification latency
    /// but requires a pairing and 2 `G2` negations to be pre-computed.
    /// Below are the full input (in the original paper notations).
    /// - Prepared verification key: $\left([\alpha]_1 \cdot [\beta]_2, -[\gamma]_2, -[\delta]_2, \left\\{ \left[ \frac{\beta \cdot u_i(x) + \alpha \cdot v_i(x) + w_i(x)}{\gamma} \right]_1 \right\\}\_{i=0}^l \right)$.
    /// - Public inputs: $\\{a_i\\}_{i=1}^l$.
    /// - Proof: $\left( \left[ A \right]_1, \left[ B \right]_2, \left[ C \right]_1 \right)$.
    public fun verify_proof_prepared<G1,G2,Gt,S>(
        pvk_alpha_g1_beta_g2: &Element<Gt>,
        pvk_gamma_g2_neg: &Element<G2>,
        pvk_delta_g2_neg: &Element<G2>,
        pvk_uvw_gamma_g1: &vector<Element<G1>>,
        public_inputs: &vector<Element<S>>,
        proof_a: &Element<G1>,
        proof_b: &Element<G2>,
        proof_c: &Element<G1>,
    ): bool {
        let scalars = vector[from_u64<S>(1)];
        std::vector::append(&mut scalars, *public_inputs);
        let g1_elements = vector[*proof_a, multi_scalar_mul(pvk_uvw_gamma_g1, &scalars), *proof_c];
        let g2_elements = vector[*proof_b, *pvk_gamma_g2_neg, *pvk_delta_g2_neg];
        eq(pvk_alpha_g1_beta_g2, &multi_pairing<G1,G2,Gt>(&g1_elements, &g2_elements))
    }

    /// A variant of `verify_proof_prepared()` that requires `pvk_alpha_g1_beta_g2` to be an element of `Fq12` instead of its subgroup `Gt`.
    /// With this variant, the caller may save a `Gt` deserialization (which involves an expensive `Gt` membership test).
    /// Below are the full input (in the original paper notations).
    /// - Prepared verification key: $\left([\alpha]_1 \cdot [\beta]_2, -[\gamma]_2, -[\delta]_2, \left\\{ \left[ \frac{\beta \cdot u_i(x) + \alpha \cdot v_i(x) + w_i(x)}{\gamma} \right]_1 \right\\}\_{i=0}^l \right)$.
    /// - Public inputs: $\\{a_i\\}_{i=1}^l$.
    /// - Proof: $\left( \left[ A \right]_1, \left[ B \right]_2, \left[ C \right]_1 \right)$.
    public fun verify_proof_prepared_fq12<G1, G2, Gt, Fq12, S>(
        pvk_alpha_g1_beta_g2: &Element<Fq12>,
        pvk_gamma_g2_neg: &Element<G2>,
        pvk_delta_g2_neg: &Element<G2>,
        pvk_uvw_gamma_g1: &vector<Element<G1>>,
        public_inputs: &vector<Element<S>>,
        proof_a: &Element<G1>,
        proof_b: &Element<G2>,
        proof_c: &Element<G1>,
    ): bool {
        let scalars = vector[from_u64<S>(1)];
        std::vector::append(&mut scalars, *public_inputs);
        let g1_elements = vector[*proof_a, multi_scalar_mul(pvk_uvw_gamma_g1, &scalars), *proof_c];
        let g2_elements = vector[*proof_b, *pvk_gamma_g2_neg, *pvk_delta_g2_neg];
        eq(pvk_alpha_g1_beta_g2, &upcast(&multi_pairing<G1,G2,Gt>(&g1_elements, &g2_elements)))
    }

    #[test_only]
    use aptos_std::crypto_algebra::{deserialize, enable_cryptography_algebra_natives};
    #[test_only]
    use aptos_std::bls12381_algebra::{Fr, FormatFrLsb, FormatG1Compr, FormatG2Compr, FormatFq12LscLsb, G1, G2, Gt, Fq12, FormatGt};

    #[test(fx = @std)]
    fun test_verify_proof_with_bls12381(fx: signer) {
        enable_cryptography_algebra_natives(&fx);

        // Below is an example MIMC proof sampled from test case https://github.com/arkworks-rs/groth16/blob/b6f9166bcf15ff4bfe101bb34e1bdc0d92302e37/tests/mimc.rs#L147.
        let vk_alpha_g1 = std::option::extract(&mut deserialize<G1, FormatG1Compr>(&x"9819f632fa8d724e351d25081ea31ccf379991ac25c90666e07103fffb042ed91c76351cd5a24041b40e26d231a5087e"));
        let vk_beta_g2 = std::option::extract(&mut deserialize<G2, FormatG2Compr>(&x"871f36a996c71a89499ffe99aa7d3f94decdd2ca8b070dbb467e42d25aad918af6ec94d61b0b899c8f724b2b549d99fc1623a0e51b6cfbea220e70e7da5803c8ad1144a67f98934a6bf2881ec6407678fd52711466ad608d676c60319a299824"));
        let vk_gamma_g2 = std::option::extract(&mut deserialize<G2, FormatG2Compr>(&x"96750d8445596af8d679487c7267ae9734aeac584ace191d225680a18ecff8ebae6dd6a5fd68e4414b1611164904ee120363c2b49f33a873d6cfc26249b66327a0de03e673b8139f79809e8b641586cde9943fa072ee5ed701c81b3fd426c220"));
        let vk_delta_g2 = std::option::extract(&mut deserialize<G2, FormatG2Compr>(&x"8d3ac832f2508af6f01872ada87ea66d2fb5b099d34c5bac81e7482c956276dfc234c8d2af5fd2394b5440d0708a2c9f124a53c0755e9595cf9f8adade5deefcb8a574a67debd3b74d08c49c23ddc14cd6d48b65dce500c8a5d330e760fe85bb"));
        let vk_gamma_abc_g1: vector<Element<G1>> = vector[
            std::option::extract(&mut deserialize<G1, FormatG1Compr>(&x"b0df760d0f2d67fdff69d0ed3a0653dd8808df3c407ea4d0e27f8612c3fbb748cb4372d33cac512ee5ef4ee1683c3fe5")),
            std::option::extract(&mut deserialize<G1, FormatG1Compr>(&x"96ec80d6b1050bbfc209f727678acce8788c05475771daffdd444ad8786c7a40195d859850fe2e72be3054e9fb8ce805")),
        ];
        let public_inputs: vector<Element<Fr>> = vector[
            std::option::extract(&mut deserialize<Fr, FormatFrLsb>(&x"0ee291cfc951388c3c7f7c85ff2dfd42bbc66a6b4acaef9a5a51ce955125a74f")),
        ];
        let proof_a = std::option::extract(&mut deserialize<G1, FormatG1Compr>(&x"8a7c7364403d91bfa5c723ce93b920c8d2e559ea5e7e34eb68cea437aa4f26bf56ba22d9400988a86f2943c79401e959"));
        let proof_b = std::option::extract(&mut deserialize<G2, FormatG2Compr>(&x"9352f8a2f9ff60d390e363d063354e9728adf39c91294499575855e803dd80eeaa1488cd24d1b80eb1b2625011e22a5d139e24f2c7ac3508874ec4bdb9c71ddf109e7853d641d23ed27bef265248d78eabe9137c03b088d8adbdf39e10f87eab"));
        let proof_c = std::option::extract(&mut deserialize<G1, FormatG1Compr>(&x"896f68b438e076d3017e64aa47621fcd69b45f49a7038e2b1b9ed4f2de9b8eb8e0a76785a39a08f024435811a73a6818"));

        assert!(verify_proof<G1, G2, Gt, Fr>(
            &vk_alpha_g1,
            &vk_beta_g2,
            &vk_gamma_g2,
            &vk_delta_g2,
            &vk_gamma_abc_g1,
            &public_inputs,
            &proof_a,
            &proof_b,
            &proof_c,
        ), 1);
    }

    #[test(fx = @std)]
    fun test_verify_proof_prepared_with_bls12381(fx: signer) {
        enable_cryptography_algebra_natives(&fx);

        // Below is an example MIMC proof sampled from test case https://github.com/arkworks-rs/groth16/blob/b6f9166bcf15ff4bfe101bb34e1bdc0d92302e37/tests/mimc.rs#L147.
        let pvk_alpha_g1_beta_g2 = std::option::extract(&mut deserialize<Gt, FormatGt>(&x"15cee98b42f8d158f421bce13983e23597123817a3b19b006294b9145f3f382686706ad9161d6234661fb1a32da19d0e2a9e672901fe4abe9efd4da96bcdb8324459b93aa48a8abb92ddd28ef053f118e190eddd6c6212bc09428ea05e709104290e37f320a3aac1dcf96f66efd9f5826b69cd075b72801ef54ccb740a0947bb3f73174e5d2fdc04292f58841ad9cc0d0c25021dfd8d592943b5e61c97f1ba68dcabd7de970ecc347c04bbaf9a062d9d49476f0b5bc77b2b9c7222781c53b713c0aae7a4cc57ff8cfb433d27fb1328d0c5453dbb97f3a70e9ce3b1da52cee2047cad225410b6dacb28e7b6876795d005cf0aefb7f25350d0197a5c2aa7369a5e06a210580bba1cc1941e1871a465cf68c84f32a29e6e898e4961a2b1fd5f8f03f03b1e1a0e191becdc8f01fb15adeb7cb6cc39e686edfcf7d65e952cf5e19a477fb5f6d2dab61a4d6c07777c1842150646c8b6fcb5989d9e524a97e7bf8b7be6b12983205970f16aeaccbdbe6cd565fa570dc45b0ad8f51c46e1f05e9f3f230dcf7567db5fc9a59a55c39139c7b357103c26bca9b70032cccff2345b76f596901ea81dc28f1d490a129501cf02204e00e8b59770188d69379144629239933523a8ec71ce6f91fbd01b2b9c411f89948183fea3949d89919e239a4aadb2347803e97ae8f7f20ade26da001f803cd61eb9bf8a67356f7cf6ec1744720b078eb992529f5c219bf16d5ef2e233a04572730e7c9572eadd9aa63c69c9f7dcf3423b1dc4c9b2032c8a7bbe91505283163a85413ecf0a0095fe1899b29f60011226f009"));
        let pvk_gamma_g2_neg = std::option::extract(&mut deserialize<G2, FormatG2Compr>(&x"b6750d8445596af8d679487c7267ae9734aeac584ace191d225680a18ecff8ebae6dd6a5fd68e4414b1611164904ee120363c2b49f33a873d6cfc26249b66327a0de03e673b8139f79809e8b641586cde9943fa072ee5ed701c81b3fd426c220"));
        let pvk_delta_g2_neg = std::option::extract(&mut deserialize<G2, FormatG2Compr>(&x"ad3ac832f2508af6f01872ada87ea66d2fb5b099d34c5bac81e7482c956276dfc234c8d2af5fd2394b5440d0708a2c9f124a53c0755e9595cf9f8adade5deefcb8a574a67debd3b74d08c49c23ddc14cd6d48b65dce500c8a5d330e760fe85bb"));
        let pvk_gamma_abc_g1: vector<Element<G1>> = vector[
            std::option::extract(&mut deserialize<G1, FormatG1Compr>(&x"b0df760d0f2d67fdff69d0ed3a0653dd8808df3c407ea4d0e27f8612c3fbb748cb4372d33cac512ee5ef4ee1683c3fe5")),
            std::option::extract(&mut deserialize<G1, FormatG1Compr>(&x"96ec80d6b1050bbfc209f727678acce8788c05475771daffdd444ad8786c7a40195d859850fe2e72be3054e9fb8ce805")),
        ];
        let public_inputs: vector<Element<Fr>> = vector[
            std::option::extract(&mut deserialize<Fr, FormatFrLsb>(&x"0ee291cfc951388c3c7f7c85ff2dfd42bbc66a6b4acaef9a5a51ce955125a74f")),
        ];
        let proof_a = std::option::extract(&mut deserialize<G1, FormatG1Compr>(&x"8a7c7364403d91bfa5c723ce93b920c8d2e559ea5e7e34eb68cea437aa4f26bf56ba22d9400988a86f2943c79401e959"));
        let proof_b = std::option::extract(&mut deserialize<G2, FormatG2Compr>(&x"9352f8a2f9ff60d390e363d063354e9728adf39c91294499575855e803dd80eeaa1488cd24d1b80eb1b2625011e22a5d139e24f2c7ac3508874ec4bdb9c71ddf109e7853d641d23ed27bef265248d78eabe9137c03b088d8adbdf39e10f87eab"));
        let proof_c = std::option::extract(&mut deserialize<G1, FormatG1Compr>(&x"896f68b438e076d3017e64aa47621fcd69b45f49a7038e2b1b9ed4f2de9b8eb8e0a76785a39a08f024435811a73a6818"));

        assert!(verify_proof_prepared<G1, G2, Gt, Fr>(
            &pvk_alpha_g1_beta_g2,
            &pvk_gamma_g2_neg,
            &pvk_delta_g2_neg,
            &pvk_gamma_abc_g1,
            &public_inputs,
            &proof_a,
            &proof_b,
            &proof_c,
        ), 1);
    }

    #[test(fx = @std)]
    fun test_verify_proof_prepared_fq12_with_bls12381(fx: signer) {
        enable_cryptography_algebra_natives(&fx);

        // Below is an example MIMC proof sampled from test case https://github.com/arkworks-rs/groth16/blob/b6f9166bcf15ff4bfe101bb34e1bdc0d92302e37/tests/mimc.rs#L147.
        let pvk_alpha_g1_beta_g2 = std::option::extract(&mut deserialize<Fq12, FormatFq12LscLsb>(&x"15cee98b42f8d158f421bce13983e23597123817a3b19b006294b9145f3f382686706ad9161d6234661fb1a32da19d0e2a9e672901fe4abe9efd4da96bcdb8324459b93aa48a8abb92ddd28ef053f118e190eddd6c6212bc09428ea05e709104290e37f320a3aac1dcf96f66efd9f5826b69cd075b72801ef54ccb740a0947bb3f73174e5d2fdc04292f58841ad9cc0d0c25021dfd8d592943b5e61c97f1ba68dcabd7de970ecc347c04bbaf9a062d9d49476f0b5bc77b2b9c7222781c53b713c0aae7a4cc57ff8cfb433d27fb1328d0c5453dbb97f3a70e9ce3b1da52cee2047cad225410b6dacb28e7b6876795d005cf0aefb7f25350d0197a5c2aa7369a5e06a210580bba1cc1941e1871a465cf68c84f32a29e6e898e4961a2b1fd5f8f03f03b1e1a0e191becdc8f01fb15adeb7cb6cc39e686edfcf7d65e952cf5e19a477fb5f6d2dab61a4d6c07777c1842150646c8b6fcb5989d9e524a97e7bf8b7be6b12983205970f16aeaccbdbe6cd565fa570dc45b0ad8f51c46e1f05e9f3f230dcf7567db5fc9a59a55c39139c7b357103c26bca9b70032cccff2345b76f596901ea81dc28f1d490a129501cf02204e00e8b59770188d69379144629239933523a8ec71ce6f91fbd01b2b9c411f89948183fea3949d89919e239a4aadb2347803e97ae8f7f20ade26da001f803cd61eb9bf8a67356f7cf6ec1744720b078eb992529f5c219bf16d5ef2e233a04572730e7c9572eadd9aa63c69c9f7dcf3423b1dc4c9b2032c8a7bbe91505283163a85413ecf0a0095fe1899b29f60011226f009"));
        let pvk_gamma_g2_neg = std::option::extract(&mut deserialize<G2, FormatG2Compr>(&x"b6750d8445596af8d679487c7267ae9734aeac584ace191d225680a18ecff8ebae6dd6a5fd68e4414b1611164904ee120363c2b49f33a873d6cfc26249b66327a0de03e673b8139f79809e8b641586cde9943fa072ee5ed701c81b3fd426c220"));
        let pvk_delta_g2_neg = std::option::extract(&mut deserialize<G2, FormatG2Compr>(&x"ad3ac832f2508af6f01872ada87ea66d2fb5b099d34c5bac81e7482c956276dfc234c8d2af5fd2394b5440d0708a2c9f124a53c0755e9595cf9f8adade5deefcb8a574a67debd3b74d08c49c23ddc14cd6d48b65dce500c8a5d330e760fe85bb"));
        let pvk_gamma_abc_g1: vector<Element<G1>> = vector[
            std::option::extract(&mut deserialize<G1, FormatG1Compr>(&x"b0df760d0f2d67fdff69d0ed3a0653dd8808df3c407ea4d0e27f8612c3fbb748cb4372d33cac512ee5ef4ee1683c3fe5")),
            std::option::extract(&mut deserialize<G1, FormatG1Compr>(&x"96ec80d6b1050bbfc209f727678acce8788c05475771daffdd444ad8786c7a40195d859850fe2e72be3054e9fb8ce805")),
        ];
        let public_inputs: vector<Element<Fr>> = vector[
            std::option::extract(&mut deserialize<Fr, FormatFrLsb>(&x"0ee291cfc951388c3c7f7c85ff2dfd42bbc66a6b4acaef9a5a51ce955125a74f")),
        ];
        let proof_a = std::option::extract(&mut deserialize<G1, FormatG1Compr>(&x"8a7c7364403d91bfa5c723ce93b920c8d2e559ea5e7e34eb68cea437aa4f26bf56ba22d9400988a86f2943c79401e959"));
        let proof_b = std::option::extract(&mut deserialize<G2, FormatG2Compr>(&x"9352f8a2f9ff60d390e363d063354e9728adf39c91294499575855e803dd80eeaa1488cd24d1b80eb1b2625011e22a5d139e24f2c7ac3508874ec4bdb9c71ddf109e7853d641d23ed27bef265248d78eabe9137c03b088d8adbdf39e10f87eab"));
        let proof_c = std::option::extract(&mut deserialize<G1, FormatG1Compr>(&x"896f68b438e076d3017e64aa47621fcd69b45f49a7038e2b1b9ed4f2de9b8eb8e0a76785a39a08f024435811a73a6818"));

        assert!(verify_proof_prepared_fq12<G1, G2, Gt, Fq12, Fr>(
            &pvk_alpha_g1_beta_g2,
            &pvk_gamma_g2_neg,
            &pvk_delta_g2_neg,
            &pvk_gamma_abc_g1,
            &public_inputs,
            &proof_a,
            &proof_b,
            &proof_c,
        ), 1);
    }
}
