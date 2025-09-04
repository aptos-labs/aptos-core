/// Generic implementation of Groth16 (proof verification) as defined in https://eprint.iacr.org/2016/260.pdf, Section 3.2.
/// Actual proof verifiers can be constructed using the pairings supported in the generic algebra module.
/// See the test cases in this module for an example of constructing with BLS12-381 curves.
///
/// **WARNING:** This code has NOT been audited. If using it in a production system, proceed at your own risk.
module groth16_example::groth16 {
    use velor_std::crypto_algebra::{Element, from_u64, multi_scalar_mul, eq, multi_pairing, upcast, pairing, add, zero};

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
    use velor_std::crypto_algebra::{deserialize, enable_cryptography_algebra_natives};
    #[test_only]
    use velor_std::bls12381_algebra::{Fr, FormatFrLsb, FormatG1Compr, FormatG2Compr, FormatFq12LscLsb, G1, G2, Gt, Fq12, FormatGt};
    #[test_only]
    use velor_std::bn254_algebra;
    #[test_only]
    use std::bcs;
    #[test_only]
    use std::vector;

    // This test gives an example of how to take a proof, verification key, and public input in the decimal string format
    // output by snarkjs and verify the proof.
    // Documentation for the serialization formats used can be found in `velor-core/velor-move/framework/velor-stdlib/sources/cryptography/X.move`,
    // where X is bn254_algebra for BN254 and bls12381_algebra for BLS12_381
    // The names are preserved from the snarkjs proof and verifier key JSON file format, with the
    // exception of "IC", which has been renamed to `vk_gamma_abc`
    // The JSON files output by snarkjs used for this example can be found in "groth16_example/example_snarkjs_proof.json"
    // and "groth16_example/example_snarkjs_vk.json"
    #[test(fx = @std)]
    fun test_verify_circom_proof(fx: signer) {
        enable_cryptography_algebra_natives(&fx);
        let a_x = 9291141442484249183824149917322150275993152355313319552386216014158050680949u256;
        let a_y = 4751084799539532208179359846086616641767957505361605807745261011239799367574u256;
        let a_bytes = bcs::to_bytes<u256>(&a_x);
        let a_y_bytes = bcs::to_bytes<u256>(&a_y);
        vector::append(&mut a_bytes, a_y_bytes);
        let a = std::option::extract(&mut deserialize<bn254_algebra::G1, bn254_algebra::FormatG1Uncompr>(&a_bytes));

        let b_x1 = 4154738608741966676660560127107026081842675422117462672893103452342068780854u256;
        let b_y1 = 4513470140932917342403349901925141325820502953664313447973655116956106256795u256;
        let b_x2 = 15981382089229198179693168711034036915586021039523535710774744447138572769902u256;
        let b_y2 = 11691946641863119124627852663455054061430853487917262585560660740296157381098u256;
        let b_bytes = bcs::to_bytes<u256>(&b_x1);
        let b_y1_bytes = bcs::to_bytes<u256>(&b_y1);
        let b_x2_bytes = bcs::to_bytes<u256>(&b_x2);
        let b_y2_bytes = bcs::to_bytes<u256>(&b_y2);
        vector::append(&mut b_bytes, b_y1_bytes);
        vector::append(&mut b_bytes, b_x2_bytes);
        vector::append(&mut b_bytes, b_y2_bytes);
        let b = std::option::extract(&mut deserialize<bn254_algebra::G2, bn254_algebra::FormatG2Uncompr>(&b_bytes));

        let c_x = 19416574444268205378069689424519026208317515867624593374135746889327790637883u256;
        let c_y = 9387724931669771435449663200581094189180308746057595118467671565223418773035u256;
        let c_bytes = bcs::to_bytes<u256>(&c_x);
        let c_y_bytes = bcs::to_bytes<u256>(&c_y);
        vector::append(&mut c_bytes, c_y_bytes);
        let c = std::option::extract(&mut deserialize<bn254_algebra::G1, bn254_algebra::FormatG1Uncompr>(&c_bytes));

        let vk_alpha_x = 20491192805390485299153009773594534940189261866228447918068658471970481763042u256;
        let vk_alpha_y = 9383485363053290200918347156157836566562967994039712273449902621266178545958u256;
        let vk_alpha_bytes = bcs::to_bytes<u256>(&vk_alpha_x);
        let vk_alpha_y_bytes = bcs::to_bytes<u256>(&vk_alpha_y);
        vector::append(&mut vk_alpha_bytes, vk_alpha_y_bytes);
        let vk_alpha = std::option::extract(&mut deserialize<bn254_algebra::G1, bn254_algebra::FormatG1Uncompr>(&vk_alpha_bytes));

        let vk_beta_x1 = 6375614351688725206403948262868962793625744043794305715222011528459656738731u256;
        let vk_beta_y1 = 4252822878758300859123897981450591353533073413197771768651442665752259397132u256;
        let vk_beta_x2 = 10505242626370262277552901082094356697409835680220590971873171140371331206856u256;
        let vk_beta_y2 = 21847035105528745403288232691147584728191162732299865338377159692350059136679u256;
        let vk_beta_bytes = bcs::to_bytes<u256>(&vk_beta_x1);
        let vk_beta_y1_bytes = bcs::to_bytes<u256>(&vk_beta_y1);
        let vk_beta_x2_bytes = bcs::to_bytes<u256>(&vk_beta_x2);
        let vk_beta_y2_bytes = bcs::to_bytes<u256>(&vk_beta_y2);
        vector::append(&mut vk_beta_bytes, vk_beta_y1_bytes);
        vector::append(&mut vk_beta_bytes, vk_beta_x2_bytes);
        vector::append(&mut vk_beta_bytes, vk_beta_y2_bytes);
        let vk_beta = std::option::extract(&mut deserialize<bn254_algebra::G2, bn254_algebra::FormatG2Uncompr>(&vk_beta_bytes));

        let vk_gamma_x1 = 10857046999023057135944570762232829481370756359578518086990519993285655852781u256;
        let vk_gamma_y1 = 11559732032986387107991004021392285783925812861821192530917403151452391805634u256;
        let vk_gamma_x2 = 8495653923123431417604973247489272438418190587263600148770280649306958101930u256;
        let vk_gamma_y2 = 4082367875863433681332203403145435568316851327593401208105741076214120093531u256;
        let vk_gamma_bytes = bcs::to_bytes<u256>(&vk_gamma_x1);
        let vk_gamma_y1_bytes = bcs::to_bytes<u256>(&vk_gamma_y1);
        let vk_gamma_x2_bytes = bcs::to_bytes<u256>(&vk_gamma_x2);
        let vk_gamma_y2_bytes = bcs::to_bytes<u256>(&vk_gamma_y2);
        vector::append(&mut vk_gamma_bytes, vk_gamma_y1_bytes);
        vector::append(&mut vk_gamma_bytes, vk_gamma_x2_bytes);
        vector::append(&mut vk_gamma_bytes, vk_gamma_y2_bytes);
        let vk_gamma = std::option::extract(&mut deserialize<bn254_algebra::G2, bn254_algebra::FormatG2Uncompr>(&vk_gamma_bytes));

        let vk_delta_x1 = 11733257046589851891850695012146277477007262722187040969185039828348964552798u256;
        let vk_delta_y1 = 4027038803827470819590008730534113934894139311083936102089700708335772383417u256;
        let vk_delta_x2 = 4501048010313692533367858190733760821904297928029128233318781536412685771070u256;
        let vk_delta_y2 = 7929485975251451284651333169168875690528578182699769192928243180764480545757u256;
        let vk_delta_bytes = bcs::to_bytes<u256>(&vk_delta_x1);
        let vk_delta_y1_bytes = bcs::to_bytes<u256>(&vk_delta_y1);
        let vk_delta_x2_bytes = bcs::to_bytes<u256>(&vk_delta_x2);
        let vk_delta_y2_bytes = bcs::to_bytes<u256>(&vk_delta_y2);
        vector::append(&mut vk_delta_bytes, vk_delta_y1_bytes);
        vector::append(&mut vk_delta_bytes, vk_delta_x2_bytes);
        vector::append(&mut vk_delta_bytes, vk_delta_y2_bytes);
        let vk_delta = std::option::extract(&mut deserialize<bn254_algebra::G2, bn254_algebra::FormatG2Uncompr>(&vk_delta_bytes));

        let vk_gamma_abc_1_x = 9301933260370907965380929235907744187309044275532228633956723711236164592702u256;
        let vk_gamma_abc_1_y = 16430819258686105004298644553325509170608676387027348203797023622583733864344u256;
        let vk_gamma_abc_2_x = 15788660278421993534955189104796829710153510462607851239961905416961864489081u256;
        let vk_gamma_abc_2_y = 15949953543860974252716711139898663592700226282992131841266708887108944899694u256;
        let vk_gamma_abc_3_x = 2752114278074204756951480614592040829268118205913128303990769829555505490153u256;
        let vk_gamma_abc_3_y = 73756394237398953482375632482393116165824820760002925371208078608366682284u256;
        let vk_gamma_abc_4_x = 6831852747912655055472532439405874457935232091421568713540315004023659266911u256;
        let vk_gamma_abc_4_y = 17612881006477748801680400530139134796116043408186867599538777507587075595161u256;
        let vk_gamma_abc_5_x = 17635013362332631023685688861083101650289128874790189338507065664254475202088u256;
        let vk_gamma_abc_5_y = 6682655906896444146648448177201874759860197304706943082757442475451670349909u256;
        let vk_gamma_abc_6_x = 9475873236009016297956856337772183876551495716493352835259515853844766276811u256;
        let vk_gamma_abc_6_y = 354515196483384658424215379959670593913045973021122448612086705709310867552u256;
        let vk_gamma_abc_7_x = 7739081130943509516619482455397124703705394954310688728375429231271874275446u256;
        let vk_gamma_abc_7_y = 20649108686175166181372170979134000369449535282768431130801436273782009562466u256;
        let vk_gamma_abc_8_x = 19048468636913770448398586085397679668705519948654488617907272996340406724088u256;
        let vk_gamma_abc_8_y = 16091090919051613132321664644473341983081123954146774949203587504747978913249u256;
        let vk_gamma_abc_9_x = 15589510145441310638849264936668688491890711017850837908639714876170500087371u256;
        let vk_gamma_abc_9_y = 160324255716095477979225131314833211463231522810162446268019950262710535809u256;

        let vk_gamma_abc_1_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_1_x);
        let vk_gamma_abc_1_y_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_1_y);
        vector::append(&mut vk_gamma_abc_1_bytes, vk_gamma_abc_1_y_bytes);
        let vk_gamma_abc_1 = std::option::extract(&mut deserialize<bn254_algebra::G1, bn254_algebra::FormatG1Uncompr>(&vk_gamma_abc_1_bytes));

        let vk_gamma_abc_2_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_2_x);
        let vk_gamma_abc_2_y_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_2_y);
        vector::append(&mut vk_gamma_abc_2_bytes, vk_gamma_abc_2_y_bytes);
        let vk_gamma_abc_2 = std::option::extract(&mut deserialize<bn254_algebra::G1, bn254_algebra::FormatG1Uncompr>(&vk_gamma_abc_2_bytes));

        let vk_gamma_abc_3_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_3_x);
        let vk_gamma_abc_3_y_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_3_y);
        vector::append(&mut vk_gamma_abc_3_bytes, vk_gamma_abc_3_y_bytes);
        let vk_gamma_abc_3 = std::option::extract(&mut deserialize<bn254_algebra::G1, bn254_algebra::FormatG1Uncompr>(&vk_gamma_abc_3_bytes));

        let vk_gamma_abc_4_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_4_x);
        let vk_gamma_abc_4_y_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_4_y);
        vector::append(&mut vk_gamma_abc_4_bytes, vk_gamma_abc_4_y_bytes);
        let vk_gamma_abc_4 = std::option::extract(&mut deserialize<bn254_algebra::G1, bn254_algebra::FormatG1Uncompr>(&vk_gamma_abc_4_bytes));

        let vk_gamma_abc_5_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_5_x);
        let vk_gamma_abc_5_y_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_5_y);
        vector::append(&mut vk_gamma_abc_5_bytes, vk_gamma_abc_5_y_bytes);
        let vk_gamma_abc_5 = std::option::extract(&mut deserialize<bn254_algebra::G1, bn254_algebra::FormatG1Uncompr>(&vk_gamma_abc_5_bytes));

        let vk_gamma_abc_6_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_6_x);
        let vk_gamma_abc_6_y_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_6_y);
        vector::append(&mut vk_gamma_abc_6_bytes, vk_gamma_abc_6_y_bytes);
        let vk_gamma_abc_6 = std::option::extract(&mut deserialize<bn254_algebra::G1, bn254_algebra::FormatG1Uncompr>(&vk_gamma_abc_6_bytes));

        let vk_gamma_abc_7_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_7_x);
        let vk_gamma_abc_7_y_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_7_y);
        vector::append(&mut vk_gamma_abc_7_bytes, vk_gamma_abc_7_y_bytes);
        let vk_gamma_abc_7 = std::option::extract(&mut deserialize<bn254_algebra::G1, bn254_algebra::FormatG1Uncompr>(&vk_gamma_abc_7_bytes));

        let vk_gamma_abc_8_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_8_x);
        let vk_gamma_abc_8_y_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_8_y);
        vector::append(&mut vk_gamma_abc_8_bytes, vk_gamma_abc_8_y_bytes);
        let vk_gamma_abc_8 = std::option::extract(&mut deserialize<bn254_algebra::G1, bn254_algebra::FormatG1Uncompr>(&vk_gamma_abc_8_bytes));

        let vk_gamma_abc_9_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_9_x);
        let vk_gamma_abc_9_y_bytes = bcs::to_bytes<u256>(&vk_gamma_abc_9_y);
        vector::append(&mut vk_gamma_abc_9_bytes, vk_gamma_abc_9_y_bytes);
        let vk_gamma_abc_9 = std::option::extract(&mut deserialize<bn254_algebra::G1, bn254_algebra::FormatG1Uncompr>(&vk_gamma_abc_9_bytes));

        let vk_gamma_abc: vector<Element<bn254_algebra::G1>> = vector[
            vk_gamma_abc_1, vk_gamma_abc_2, vk_gamma_abc_3, vk_gamma_abc_4, vk_gamma_abc_5, vk_gamma_abc_6, vk_gamma_abc_7, vk_gamma_abc_8, vk_gamma_abc_9
        ];

        let public_1_val = 0u256;
        let public_2_val = 7714357208561315320836530795186262204499958856333073618293621003566744654598u256;
        let public_3_val = 91557130945874u256;
        let public_4_val = 34458076785421617u256;
        let public_5_val = 2800000u256;
        let public_6_val = 5591876u256;
        let public_7_val = 5591876u256;
        let public_8_val = 751199308u256;
        let public_1_bytes = bcs::to_bytes<u256>(&public_1_val);
        let public_2_bytes = bcs::to_bytes<u256>(&public_2_val);
        let public_3_bytes = bcs::to_bytes<u256>(&public_3_val);
        let public_4_bytes = bcs::to_bytes<u256>(&public_4_val);
        let public_5_bytes = bcs::to_bytes<u256>(&public_5_val);
        let public_6_bytes = bcs::to_bytes<u256>(&public_6_val);
        let public_7_bytes = bcs::to_bytes<u256>(&public_7_val);
        let public_8_bytes = bcs::to_bytes<u256>(&public_8_val);
        let public_1 = std::option::extract(&mut deserialize<bn254_algebra::Fr, bn254_algebra::FormatFrLsb>(&public_1_bytes));
        let public_2 = std::option::extract(&mut deserialize<bn254_algebra::Fr, bn254_algebra::FormatFrLsb>(&public_2_bytes));
        let public_3 = std::option::extract(&mut deserialize<bn254_algebra::Fr, bn254_algebra::FormatFrLsb>(&public_3_bytes));
        let public_4 = std::option::extract(&mut deserialize<bn254_algebra::Fr, bn254_algebra::FormatFrLsb>(&public_4_bytes));
        let public_5 = std::option::extract(&mut deserialize<bn254_algebra::Fr, bn254_algebra::FormatFrLsb>(&public_5_bytes));
        let public_6 = std::option::extract(&mut deserialize<bn254_algebra::Fr, bn254_algebra::FormatFrLsb>(&public_6_bytes));
        let public_7 = std::option::extract(&mut deserialize<bn254_algebra::Fr, bn254_algebra::FormatFrLsb>(&public_7_bytes));
        let public_8 = std::option::extract(&mut deserialize<bn254_algebra::Fr, bn254_algebra::FormatFrLsb>(&public_8_bytes));

        let public_inputs: vector<Element<bn254_algebra::Fr>> = vector[
            public_1, public_2, public_3, public_4, public_5, public_6, public_7, public_8
        ];

        assert!(verify_proof<bn254_algebra::G1, bn254_algebra::G2, bn254_algebra::Gt, bn254_algebra::Fr>(
            &vk_alpha,
            &vk_beta,
            &vk_gamma,
            &vk_delta,
            &vk_gamma_abc,
            &public_inputs,
            &a,
            &b,
            &c,
        ), 1);
    }

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
