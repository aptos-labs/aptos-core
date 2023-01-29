module aptos_std::groth16 {
    #[test_only]
    use aptos_std::groups::{BLS12_381_G1, BLS12_381_G2, scalar_deserialize, deserialize_element_uncompressed, BLS12_381_Gt, BLS12_381_Fr};
    use aptos_std::groups;

    /// A Groth16 verifying key.
    struct VerifyingKey<phantom G1, phantom G2, phantom Gt> has drop {
        alpha_g1: groups::Element<G1>,
        beta_g2: groups::Element<G2>,
        gamma_g2: groups::Element<G2>,
        delta_g2: groups::Element<G2>,
        gamma_abc_g1: vector<groups::Element<G1>>,
    }

    /// A Groth16 verifying key pre-processed for faster verification.
    struct PreparedVerifyingKey<phantom G1, phantom G2, phantom Gt> has drop {
        alpha_g1_beta_g2: groups::Element<Gt>,
        gamma_g2_neg: groups::Element<G2>,
        delta_g2_neg: groups::Element<G2>,
        gamma_abc_g1: vector<groups::Element<G1>>,
    }

    /// A Groth16 proof.
    struct Proof<phantom G1, phantom G2, phantom Gt> has drop {
        a: groups::Element<G1>,
        b: groups::Element<G2>,
        c: groups::Element<G1>,
    }

    /// Create a new Groth16 verifying key.
    public fun new_vk<G1,G2,Gt>(alpha_g1: groups::Element<G1>, beta_g2: groups::Element<G2>, gamma_g2: groups::Element<G2>, delta_g2: groups::Element<G2>, gamma_abc_g1: vector<groups::Element<G1>>): VerifyingKey<G1,G2,Gt> {
        VerifyingKey {
            alpha_g1,
            beta_g2,
            gamma_g2,
            delta_g2,
            gamma_abc_g1,
        }
    }

    /// Create a new pre-processed Groth16 verifying key.
    public fun new_pvk<G1,G2,Gt>(alpha_g1_beta_g2: groups::Element<Gt>, gamma_g2_neg: groups::Element<G2>, delta_g2_neg: groups::Element<G2>, gamma_abc_g1: vector<groups::Element<G1>>): PreparedVerifyingKey<G1,G2,Gt> {
        PreparedVerifyingKey {
            alpha_g1_beta_g2,
            gamma_g2_neg,
            delta_g2_neg,
            gamma_abc_g1,
        }
    }

    /// Pre-process a Groth16 verification key `vk` for faster verification.
    public fun prepare_verifying_key<G1,G2,Gt>(vk: &VerifyingKey<G1,G2,Gt>): PreparedVerifyingKey<G1,G2,Gt> {
        PreparedVerifyingKey {
            alpha_g1_beta_g2: groups::pairing<G1,G2,Gt>(&vk.alpha_g1, &vk.beta_g2),
            gamma_g2_neg: groups::element_neg(&vk.gamma_g2),
            delta_g2_neg: groups::element_neg(&vk.delta_g2),
            gamma_abc_g1: vk.gamma_abc_g1,
        }
    }

    /// Create a Groth16 proof.
    public fun new_proof<G1,G2,Gt>(a: groups::Element<G1>, b: groups::Element<G2>, c: groups::Element<G1>): Proof<G1,G2,Gt> {
        Proof { a, b, c }
    }

    /// Verify a Groth16 proof.
    public fun verify_proof<G1,G2,Gt,S>(vk: &VerifyingKey<G1,G2,Gt>, public_inputs: &vector<groups::Scalar<S>>, proof: &Proof<G1,G2,Gt>): bool {
        let left = groups::pairing<G1,G2,Gt>(&proof.a, &proof.b);
        let right_1 = groups::pairing<G1,G2,Gt>(&vk.alpha_g1, &vk.beta_g2);
        let scalars = vector[groups::scalar_from_u64<S>(1)];
        std::vector::append(&mut scalars, *public_inputs);
        let right_2 = groups::pairing(&groups::element_multi_scalar_mul(&vk.gamma_abc_g1, &scalars), &vk.gamma_g2);
        let right_3 = groups::pairing(&proof.c, &vk.delta_g2);
        let right = groups::element_add(&groups::element_add(&right_1, &right_2), &right_3);
        groups::element_eq(&left, &right)
    }

    /// Verify a Groth16 proof `proof` against the public inputs `public_inputs` with a prepared verification key `pvk`.
    public fun verify_proof_with_pvk<G1,G2,Gt,S>(pvk: &PreparedVerifyingKey<G1,G2,Gt>, public_inputs: &vector<groups::Scalar<S>>, proof: &Proof<G1,G2,Gt>): bool {
        let scalars = vector[groups::scalar_from_u64<S>(1)];
        std::vector::append(&mut scalars, *public_inputs);
        let g1_elements: vector<groups::Element<G1>> = vector[proof.a, groups::element_multi_scalar_mul(&pvk.gamma_abc_g1, &scalars), proof.c];
        let g2_elements: vector<groups::Element<G2>> = vector[proof.b, pvk.gamma_g2_neg, pvk.delta_g2_neg];

        groups::element_eq(&pvk.alpha_g1_beta_g2, &groups::pairing_product<G1,G2,Gt>(&g1_elements, &g2_elements))
    }

    #[test(fx = @std)]
    fun test_verify_mimc_proof(fx: signer) {
        std::features::change_feature_flags(&fx, vector[std::features::get_generic_group_basic_operations_feature(), std::features::get_bls12_381_groups_feature()], vector[]);

        let gamma_abc_g1: vector<groups::Element<BLS12_381_G1>> = vector[
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G1>(x"00192808ef3f352b15066066b5784284ad310194591851848b9ca5099b7bd35d818a7902e4ec148b244d97c553599d0d0c961ac300485ea9d75a4251b7e54d9b9f2467cff599c19f399a0098f9ce6b88497c3f8e9cde85c9b4cdbf2cbc429118")),
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G1>(x"cdd8b7ce59d13e8f29e7d7083b619feb96e38f0e520c46403be8df7ec7d4025b7e24aadb947528e057b5117cabe62012c8e331dc103e205add7ecdd52d109dd2a56e5e990921b5e1b3aeb724e5b8069011b7589e7ef42d975d0711d51f806e19")),
        ];

        let vk = new_vk<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G1>(x"adbee26661572ffa56cee2461462e6ad29666236d8e787618276e7e6ecf20eed31a80380885e8100408b90d604ca30023fcf7ec1d74cbcde16731854217c39b0f338a253fcbf9d274497191d950ef271714ca161e60427b851667b7fda1a8b0c")), //alpha_g1
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G2>(x"b0df6a41cc41cb8f8ddd6b288c2e78c6d8bbeefb134d04caab17b6448bc10e0068e096b1b813d6a2b5a5346100b92807a40fcfd0bc0eeef0bd2db0aa5caa8f7e0b3d814eceee9d6d9f06ba9c72c055ff573a4b8ad99277daa9046436a3991702e7c2e6a45b4f8edfd15cd9ea6ae3e9de50fee7120bc4cec12697ec1f3c95157aa3e77705b37e895c5155e2a3d4f044118090a68579cc610b50bd81997369163d7d96970d7f92f1abe7454ec214a07d33d64f5e0aeec81cd91bd129906286a205")), //beta_g2
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G2>(x"4d6587ef027e2ab5176932edc3ebfdc9cb0e2eec829ac63e7d7a3d0de3b00ec01f2933dee630e24443d1cd02c84fe5142d53bf638224cd83cf29a61fe3223caa805c7f026fc54f2057f60944e03d4ce99da2cdc0aeaf994790c72aa3f6d19a0116b0b3852cba22106a7b0ad3b011b02d9afc3c99bf82c7560a9c13a2e5e2c8a03749021f750b1883b533a584b98e361582ea83d42e8476eb3a14722d649f5f14ac354e7ada65765fc07d499da0d247753b6dbf794ccd21e632e0212e0fff7617")), //gamma_g2
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G2>(x"ef1d581e38ab19caaea59e3e081d88a202c0b0797298adf4df82e4416fd462bf524dec481378182b4b671f650d7eca03bdbed4dc82e1796ca6e79a80ec06f06d6ee647146d844be01d414c1d8d5712ad76a7d6a781fbcc97b50789ceb2e2f810d480f5250547c24a7aafb8d97ea118782f0728ecd352b5e8b517b1401daf71e8e371c11844a84e5a658b3b75fbe6aa0c16ab2710e4dcb3ec13e78776fb5f0c47033e44722a3649253e90b5a889aabaee2effd57e37074378ba2cda5227262f08")), //delta_g2
            gamma_abc_g1
        );

        let proof = new_proof<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G1>(x"03fdf4a4b69c2236148733c44bbf53f1cf20161efbdbc3c540374e9f28273b4e436fba27e61b723a9614bcf0282131069d48db37b25d3f4f62df5c745dc57fa45565d70fe2f4f9a59e3b354f6eee0c9e69215f3509063458845ae6b13b213417")),
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G2>(x"fe5aa604f9e3a4e7a6f28b400bf765a635eab3cf0c1e94d87dfd696c5a16910a1ecfe75f9249d2f1680d7c44a8f67402f895d4a3bc21468f8c4f307e357fadc551951b82d1efebd0e27d0fb6067ce25157faf384b13cd76f05eb8077b53baf0b608a5c097cced7f4775a25746c681f316541de4fd27a76dc6c7af2ebc494ab26532ade11330be114be485375557ea412b485cc40ec6b49ba1135ede83181fc483fe33442fdf969f2f13efe537107a3b7a2bd104f42c375abf0e5581dd1cc9a01")),
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G1>(x"756ec20e1941b949e9a8af556925e3f6430f1cd1eeb801fe0186b3b664cb8457060f0e27551b5cc2b3dad878761c8d03acb8e0cbd8da8d0d541f60503b0726064310d0063802fad36fb362d11ef1060a22916dab9727b0d9feaf2f8636d74a02"))
        );

        let public_inputs: vector<groups::Scalar<BLS12_381_Fr>> = vector[
            std::option::extract(&mut scalar_deserialize<BLS12_381_Fr>(&x"08436a5c0c09f30892728d4ad89cc85523967b1c4f57f1e7b10dffd751e0483b")),
        ];
        assert!(verify_proof(&vk, &public_inputs, &proof), 1);

        let pvk = prepare_verifying_key(&vk);
        assert!(verify_proof_with_pvk(&pvk, &public_inputs, &proof), 1);
    }

    #[test(fx = @std)]
    fun test_verify_mimc_proof_with_pvk(fx: signer) {
        std::features::change_feature_flags(&fx, vector[std::features::get_generic_group_basic_operations_feature(), std::features::get_bls12_381_groups_feature()], vector[]);

        let gamma_abc_g1: vector<groups::Element<BLS12_381_G1>> = vector[
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G1>(x"00192808ef3f352b15066066b5784284ad310194591851848b9ca5099b7bd35d818a7902e4ec148b244d97c553599d0d0c961ac300485ea9d75a4251b7e54d9b9f2467cff599c19f399a0098f9ce6b88497c3f8e9cde85c9b4cdbf2cbc429118")),
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G1>(x"cdd8b7ce59d13e8f29e7d7083b619feb96e38f0e520c46403be8df7ec7d4025b7e24aadb947528e057b5117cabe62012c8e331dc103e205add7ecdd52d109dd2a56e5e990921b5e1b3aeb724e5b8069011b7589e7ef42d975d0711d51f806e19")),
        ];

        let pvk = new_pvk<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_Gt>(x"4cfc28c9ab7ca59d92819a690a0004740db171caf62cef4109010774e51868028b13461af578193f68cbcfd37bef33163654f0e3333f1ef493ddb76c8932a974f790448906505f24872fbef890ae9b290d69b1ae464ce7c63ce1d13c87a8fe012d35c66da1b39b630499ba6bad54f646011f02726caf16350cc339209a01ffa0d7cd5abed0d48e51dc332fed47af9f00732542456d9db6cdf95eb2b149031457fbcef576280b8f32768acc1e391483a098419a9857144e89c8a3b58439d9ea04ed53c3641fabb30a39e664662c1978fad1f915e7924f7aab6d2e7b23d361031387be29ceca4fa5ce33dc65536a736a045a11203cb362ae631f7049a0b269edcdd4b42bf5e3e7e0c6e28e26fbc66a7769c8d9dc01a9194e5f174af60f4b7e10163292e5ca352a5a1f73bd8a6f02bd19ad61744fd2e401dbc8f8badf8fd4d1059d16d9b616585cc18cda03b2bf11d5380ceb365b304f7e6326a4d3dcb4435ce341b9c77a82e0a7e343aee4b5c787ab363e38b71e7fd0ae725ad12c17b7e800fc06b3180a5f7d9e3aa1d9d443dd50ccdbb55adc1cc7dd837289c87b7eeb1f240fdd9375599c01e7ff91cedde6b36850b20e4f08ea047119efc2804ffefc80ecaad6c4fdc5446a75b1cb76ecf223c820c0f0d84d556e4c7489b8685ffab110cba0098688abc91994a68f52f86ef76c7ef944e8bad9085158be7962947e43b0fe28ab4ae5e70a6347188af75610810e3735184f31e7638ed336ef08b8bb64d01299e9648bece8b7eebe97c800ff9c9fbc2a57dc46be435c5e9fe0a4fa51dc9bd95d04")), //alpha_g1_beta_g2
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G2>(x"4d6587ef027e2ab5176932edc3ebfdc9cb0e2eec829ac63e7d7a3d0de3b00ec01f2933dee630e24443d1cd02c84fe5142d53bf638224cd83cf29a61fe3223caa805c7f026fc54f2057f60944e03d4ce99da2cdc0aeaf994790c72aa3f6d19a0195fa4b7ad345dca9958449de4deefbf089f9735de14f6910b57671519f68aec39f634924419c03c8e4b2dab43083ca0429c07b2bd17b88cec4ebe1839a604c0a78c0627cc66cba07ff943b56e4782fef9b3f8cc969daf96467065e0bdb128a02")), //gamma_g2_neg
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G2>(x"ef1d581e38ab19caaea59e3e081d88a202c0b0797298adf4df82e4416fd462bf524dec481378182b4b671f650d7eca03bdbed4dc82e1796ca6e79a80ec06f06d6ee647146d844be01d414c1d8d5712ad76a7d6a781fbcc97b50789ceb2e2f810d7290adafab83c6f85509bd77f5e93a6f4ee880acd7f7b7e09fbd3b2679c057cf33a8a2a72ffccf0345b44c4ee2a560d95ffd7ef1b234bcdeb18cc3a03a09fd720b86c84769ce7418182cf4afba0bc75a8ad75c47ea0d8d2dfb9a5e6c2ebd111")), //delta_g2_neg
            gamma_abc_g1
        );

        let proof = new_proof<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G1>(x"03fdf4a4b69c2236148733c44bbf53f1cf20161efbdbc3c540374e9f28273b4e436fba27e61b723a9614bcf0282131069d48db37b25d3f4f62df5c745dc57fa45565d70fe2f4f9a59e3b354f6eee0c9e69215f3509063458845ae6b13b213417")),
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G2>(x"fe5aa604f9e3a4e7a6f28b400bf765a635eab3cf0c1e94d87dfd696c5a16910a1ecfe75f9249d2f1680d7c44a8f67402f895d4a3bc21468f8c4f307e357fadc551951b82d1efebd0e27d0fb6067ce25157faf384b13cd76f05eb8077b53baf0b608a5c097cced7f4775a25746c681f316541de4fd27a76dc6c7af2ebc494ab26532ade11330be114be485375557ea412b485cc40ec6b49ba1135ede83181fc483fe33442fdf969f2f13efe537107a3b7a2bd104f42c375abf0e5581dd1cc9a01")),
            std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G1>(x"756ec20e1941b949e9a8af556925e3f6430f1cd1eeb801fe0186b3b664cb8457060f0e27551b5cc2b3dad878761c8d03acb8e0cbd8da8d0d541f60503b0726064310d0063802fad36fb362d11ef1060a22916dab9727b0d9feaf2f8636d74a02"))
        );

        let public_inputs: vector<groups::Scalar<BLS12_381_Fr>> = vector[
            std::option::extract(&mut scalar_deserialize(&x"08436a5c0c09f30892728d4ad89cc85523967b1c4f57f1e7b10dffd751e0483b")),
        ];
        assert!(verify_proof_with_pvk(&pvk, &public_inputs, &proof), 1);
    }
}
