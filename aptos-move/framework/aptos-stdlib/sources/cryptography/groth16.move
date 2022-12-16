module aptos_std::groth16 {
    #[test_only]
    use aptos_std::curves::{BLS12_381_G1, BLS12_381_G2, bytes_into_point, bytes_into_scalar, BLS12_381_Gt};
    use aptos_std::curves;

    struct VerifyingKey<phantom G1, phantom G2, phantom Gt> has drop {
        alpha_g1: curves::Point<G1>,
        beta_g2: curves::Point<G2>,
        gamma_g2: curves::Point<G2>,
        delta_g2: curves::Point<G2>,
        gamma_abc_g1: vector<curves::Point<G1>>,
    }

    struct Proof<phantom G1, phantom G2, phantom Gt> has drop {
        a: curves::Point<G1>,
        b: curves::Point<G2>,
        c: curves::Point<G1>,
    }

    public fun new_vk<G1,G2,Gt>(alpha_g1: curves::Point<G1>, beta_g2: curves::Point<G2>, gamma_g2: curves::Point<G2>, delta_g2: curves::Point<G2>, gamma_abc_g1: vector<curves::Point<G1>>): VerifyingKey<G1,G2,Gt> {
        VerifyingKey {
            alpha_g1,
            beta_g2,
            gamma_g2,
            delta_g2,
            gamma_abc_g1,
        }
    }

    public fun new_proof<G1,G2,Gt>(a: curves::Point<G1>, b: curves::Point<G2>, c: curves::Point<G1>): Proof<G1,G2,Gt> {
        Proof { a, b, c }
    }

    public fun verify_proof<G1,G2,Gt>(_vk: &VerifyingKey<G1,G2,Gt>, _public_inputs: &vector<curves::Scalar<G1>>, _proof: &Proof<G1,G2,Gt>): bool {
        let gamma_abc_g1_handles = vector[];
        let gamma_abc_g1_count = std::vector::length(&_vk.gamma_abc_g1);
        let i = 0;
        while (i < gamma_abc_g1_count) {
            let item = std::vector::borrow(&_vk.gamma_abc_g1, i);
            let handle = curves::get_point_handle(item);
            std::vector::push_back(&mut gamma_abc_g1_handles, (handle as u8));
            i = i + 1;
        };

        let public_input_handles: vector<u8> = vector[];
        let public_input_count = std::vector::length(_public_inputs);
        let i = 0;
        while (i < public_input_count) {
            let item = std::vector::borrow(_public_inputs, i);
            let handle = curves::get_scalar_handle(item);
            std::vector::push_back(&mut public_input_handles, (handle as u8));
            i = i + 1;
        };

        verify_proof_internal(
            curves::get_point_handle(&_vk.alpha_g1),
            curves::get_point_handle(&_vk.beta_g2),
            curves::get_point_handle(&_vk.gamma_g2),
            curves::get_point_handle(&_vk.delta_g2),
            gamma_abc_g1_handles,
            curves::get_point_handle(&_proof.a),
            curves::get_point_handle(&_proof.b),
            curves::get_point_handle(&_proof.c),
            public_input_handles,
            curves::get_pairing_id<G1,G2,Gt>()
        )
    }

    native fun verify_proof_internal(
        vk_alpha_g1_handle: u8, vk_beta_g_handle: u8, vk_gamma_g2_handle: u8, vk_delta_g2_handle: u8, gamma_abc_g1_handles: vector<u8>,
        proof_a_handle: u8, proof_b_handle: u8, proof_c_handle: u8,
        public_input_handle: vector<u8>,
        pairing_id: u8
    ): bool;

    #[test]
    fun test1() {
        let gamma_abc_g1: vector<curves::Point<BLS12_381_G1>> = vector[bytes_into_point<BLS12_381_G1>(x"97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb"), bytes_into_point<BLS12_381_G1>(x"97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb")];
        let vk = new_vk<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(
            bytes_into_point<BLS12_381_G1>(x"97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb"), //alpha_g1
            bytes_into_point<BLS12_381_G2>(x"93e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8"), //beta_g2
            bytes_into_point<BLS12_381_G2>(x"93e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8"), //gamma_g2
            bytes_into_point<BLS12_381_G2>(x"93e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8"), //delta_g2
            gamma_abc_g1
        );

        let proof = new_proof<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(
            bytes_into_point<BLS12_381_G1>(x"97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb"),
            bytes_into_point<BLS12_381_G2>(x"93e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8"),
            bytes_into_point<BLS12_381_G1>(x"97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb")
        );

        let public_inputs: vector<curves::Scalar<BLS12_381_G1>> = vector[bytes_into_scalar(x"0100000000000000000000000000000000000000000000000000000000000000"), bytes_into_scalar(x"0100000000000000000000000000000000000000000000000000000000000000")];
        assert!(verify_proof(&vk, &public_inputs, &proof), 1);
    }
}
