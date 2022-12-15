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
        false
    }

    #[test]
    fun test1() {
        let gamma_abc_g1: vector<curves::Point<BLS12_381_G1>> = vector[bytes_into_point<BLS12_381_G1>(b""), bytes_into_point<BLS12_381_G1>(b"")];

        let vk = new_vk<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(
            bytes_into_point<BLS12_381_G1>(b""), //alpha_g1
            bytes_into_point<BLS12_381_G2>(b""), //beta_g2
            bytes_into_point<BLS12_381_G2>(b""), //gamma_g2
            bytes_into_point<BLS12_381_G2>(b""), //delta_g2
            gamma_abc_g1
        );

        let proof = new_proof<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(
            bytes_into_point<BLS12_381_G1>(b""),
            bytes_into_point<BLS12_381_G2>(b""),
            bytes_into_point<BLS12_381_G1>(b"")
        );

        let public_inputs: vector<curves::Scalar<BLS12_381_G1>> = vector[bytes_into_scalar(b""), bytes_into_scalar(b"")];
        assert!(verify_proof(&vk, &public_inputs, &proof), 1);
    }
}
