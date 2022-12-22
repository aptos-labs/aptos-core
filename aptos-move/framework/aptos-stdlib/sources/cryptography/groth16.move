module aptos_std::groth16 {
    #[test_only]
    use aptos_std::curves::{BLS12_381_G1, BLS12_381_G2, scalar_from_bytes, element_from_bytes, BLS12_381_Gt};
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

    public fun verify_proof<G1,G2,Gt>(vk: &VerifyingKey<G1,G2,Gt>, public_inputs: &vector<curves::Scalar<G1>>, proof: &Proof<G1,G2,Gt>): bool {
        let left = curves::pairing<G1,G2,Gt>(&proof.a, &proof.b);
        let right_1 = curves::pairing<G1,G2,Gt>(&vk.alpha_g1, &vk.beta_g2);

        let n = std::vector::length(public_inputs);
        let i = 0;
        let acc = *std::vector::borrow(&vk.gamma_abc_g1, 0);
        while (i < n) {
            let cur_scalar = std::vector::borrow(public_inputs, i);
            let cur_point = std::vector::borrow(&vk.gamma_abc_g1, i+1);
            acc = curves::point_add(&acc, &curves::point_mul(cur_scalar, cur_point));
            i = i + 1;
        };

        let right_2 = curves::pairing(&acc, &vk.gamma_g2);
        let right_3 = curves::pairing(&proof.c, &vk.delta_g2);
        let right = curves::point_add(&curves::point_add(&right_1, &right_2), &right_3);
        curves::point_eq(&left, &right)
    }

    #[test]
    fun test1() {
        let gamma_abc_g1: vector<curves::Point<BLS12_381_G1>> = vector[
            element_from_bytes<BLS12_381_G1>(x"00192808ef3f352b15066066b5784284ad310194591851848b9ca5099b7bd35d818a7902e4ec148b244d97c553599d0d0c961ac300485ea9d75a4251b7e54d9b9f2467cff599c19f399a0098f9ce6b88497c3f8e9cde85c9b4cdbf2cbc429118"),
            element_from_bytes<BLS12_381_G1>(x"cdd8b7ce59d13e8f29e7d7083b619feb96e38f0e520c46403be8df7ec7d4025b7e24aadb947528e057b5117cabe62012c8e331dc103e205add7ecdd52d109dd2a56e5e990921b5e1b3aeb724e5b8069011b7589e7ef42d975d0711d51f806e19"),
        ];

        let vk = new_vk<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(
            element_from_bytes<BLS12_381_G1>(x"adbee26661572ffa56cee2461462e6ad29666236d8e787618276e7e6ecf20eed31a80380885e8100408b90d604ca30023fcf7ec1d74cbcde16731854217c39b0f338a253fcbf9d274497191d950ef271714ca161e60427b851667b7fda1a8b0c"), //alpha_g1
            element_from_bytes<BLS12_381_G2>(x"b0df6a41cc41cb8f8ddd6b288c2e78c6d8bbeefb134d04caab17b6448bc10e0068e096b1b813d6a2b5a5346100b92807a40fcfd0bc0eeef0bd2db0aa5caa8f7e0b3d814eceee9d6d9f06ba9c72c055ff573a4b8ad99277daa9046436a3991702e7c2e6a45b4f8edfd15cd9ea6ae3e9de50fee7120bc4cec12697ec1f3c95157aa3e77705b37e895c5155e2a3d4f044118090a68579cc610b50bd81997369163d7d96970d7f92f1abe7454ec214a07d33d64f5e0aeec81cd91bd129906286a205"), //beta_g2
            element_from_bytes<BLS12_381_G2>(x"4d6587ef027e2ab5176932edc3ebfdc9cb0e2eec829ac63e7d7a3d0de3b00ec01f2933dee630e24443d1cd02c84fe5142d53bf638224cd83cf29a61fe3223caa805c7f026fc54f2057f60944e03d4ce99da2cdc0aeaf994790c72aa3f6d19a0116b0b3852cba22106a7b0ad3b011b02d9afc3c99bf82c7560a9c13a2e5e2c8a03749021f750b1883b533a584b98e361582ea83d42e8476eb3a14722d649f5f14ac354e7ada65765fc07d499da0d247753b6dbf794ccd21e632e0212e0fff7617"), //gamma_g2
            element_from_bytes<BLS12_381_G2>(x"ef1d581e38ab19caaea59e3e081d88a202c0b0797298adf4df82e4416fd462bf524dec481378182b4b671f650d7eca03bdbed4dc82e1796ca6e79a80ec06f06d6ee647146d844be01d414c1d8d5712ad76a7d6a781fbcc97b50789ceb2e2f810d480f5250547c24a7aafb8d97ea118782f0728ecd352b5e8b517b1401daf71e8e371c11844a84e5a658b3b75fbe6aa0c16ab2710e4dcb3ec13e78776fb5f0c47033e44722a3649253e90b5a889aabaee2effd57e37074378ba2cda5227262f08"), //delta_g2
            gamma_abc_g1
        );

        let proof = new_proof<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(
            element_from_bytes<BLS12_381_G1>(x"03fdf4a4b69c2236148733c44bbf53f1cf20161efbdbc3c540374e9f28273b4e436fba27e61b723a9614bcf0282131069d48db37b25d3f4f62df5c745dc57fa45565d70fe2f4f9a59e3b354f6eee0c9e69215f3509063458845ae6b13b213417"),
            element_from_bytes<BLS12_381_G2>(x"fe5aa604f9e3a4e7a6f28b400bf765a635eab3cf0c1e94d87dfd696c5a16910a1ecfe75f9249d2f1680d7c44a8f67402f895d4a3bc21468f8c4f307e357fadc551951b82d1efebd0e27d0fb6067ce25157faf384b13cd76f05eb8077b53baf0b608a5c097cced7f4775a25746c681f316541de4fd27a76dc6c7af2ebc494ab26532ade11330be114be485375557ea412b485cc40ec6b49ba1135ede83181fc483fe33442fdf969f2f13efe537107a3b7a2bd104f42c375abf0e5581dd1cc9a01"),
            element_from_bytes<BLS12_381_G1>(x"756ec20e1941b949e9a8af556925e3f6430f1cd1eeb801fe0186b3b664cb8457060f0e27551b5cc2b3dad878761c8d03acb8e0cbd8da8d0d541f60503b0726064310d0063802fad36fb362d11ef1060a22916dab9727b0d9feaf2f8636d74a02")
        );

        let public_inputs: vector<curves::Scalar<BLS12_381_G1>> = vector[
            std::option::extract(&mut scalar_from_bytes(&x"08436a5c0c09f30892728d4ad89cc85523967b1c4f57f1e7b10dffd751e0483b")),
        ];
        assert!(verify_proof(&vk, &public_inputs, &proof), 1);
    }
}
