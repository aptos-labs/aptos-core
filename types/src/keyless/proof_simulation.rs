use ark_groth16::{prepare_verifying_key, Groth16};
use ark_crypto_primitives::snark::SNARK;
use ark_ec::pairing::Pairing;
use ark_ff::Field;
use ark_relations::r1cs::{ConstraintSynthesizer, SynthesisError};
use ark_std::{
    rand::{RngCore, SeedableRng},
    test_rng, UniformRand,
};
use ark_std::{marker::PhantomData, vec::Vec};
use ark_groth16::r1cs_to_qap::{LibsnarkReduction, R1CSToQAP};
use ark_groth16::data_structures::{VerifyingKey, Proof};
use ark_serialize::*;
use ark_std::rand::Rng;
use ark_relations::r1cs::Result as R1CSResult;
use ark_ec::{AffineRepr,CurveGroup};
use ark_ff::PrimeField;
use std::ops::AddAssign;

/// The SNARK of [[Groth16]](https://eprint.iacr.org/2016/260.pdf), where "proving" implements the
/// simulation algorithm instead, using the trapdoor output by the modified setup algorithm also
/// implemented in this struct
pub struct Groth16Simulator<E: Pairing, QAP: R1CSToQAP = LibsnarkReduction> {
    _p: PhantomData<(E, QAP)>,
}

/// The simulation prover key for for the Groth16 zkSNARK, used only for simulating proofs with the
/// secret trapdoor information
#[derive(Clone, Debug, PartialEq, CanonicalSerialize, CanonicalDeserialize)]
pub struct ProvingKeyWithTrapdoor<E: Pairing> {
    /// Vector of elements from the verifying key
    pub gamma_abc_g1: Vec<E::G1Affine>,
    /// Trapdoor alpha
    pub alpha: E::ScalarField,
    /// Trapdoor beta
    pub beta: E::ScalarField,
    /// Trapdoor delta
    pub delta: E::ScalarField,
    /// Trapdoor gamma 
    pub gamma: E::ScalarField,
    /// Generator for G1
    pub g1: E::G1Affine,
    /// Generator for G2
    pub g2: E::G2Affine,
}

impl<E: Pairing, QAP: R1CSToQAP> Groth16Simulator<E, QAP> {
    fn circuit_specific_setup_with_trapdoor<C: ConstraintSynthesizer<E::ScalarField>, R: RngCore>(
        circuit: C,
        rng: &mut R,
    ) -> Result<(ProvingKeyWithTrapdoor<E>, VerifyingKey<E>), SynthesisError> {
        let (pk, vk) = Self::generate_random_parameters_and_trapdoor_with_reduction(circuit, rng)?;

        Ok((pk, vk))
    }

    /// Generates a random common reference string for
    /// a circuit using the provided R1CS-to-QAP reduction.
    #[inline]
    pub fn generate_random_parameters_and_trapdoor_with_reduction<C>(
        circuit: C,
        rng: &mut impl Rng,
    ) -> R1CSResult<(ProvingKeyWithTrapdoor<E>, VerifyingKey<E>)>
    where
        C: ConstraintSynthesizer<E::ScalarField>,
    {
        let alpha = E::ScalarField::rand(rng);
        let beta = E::ScalarField::rand(rng);
        let gamma = E::ScalarField::rand(rng);
        let delta = E::ScalarField::rand(rng);

        let g1_generator = E::G1::rand(rng);
        let g2_generator = E::G2::rand(rng);

        let pk = Groth16::<E,QAP>::generate_parameters_with_qap(
            circuit,
            alpha,
            beta,
            gamma,
            delta,
            g1_generator,
            g2_generator,
            rng,
        ).unwrap();

        Ok((
        ProvingKeyWithTrapdoor {
            gamma_abc_g1: pk.vk.gamma_abc_g1.clone(),
            alpha,
            beta,
            delta,
            gamma,
            g1: g1_generator.into_affine(),
            g2: g2_generator.into_affine(),
        },
        pk.vk.clone()))
    }

    /// Create a Groth16 proof that is zero-knowledge using the provided
    /// R1CS-to-QAP reduction.
    /// This method samples randomness for zero knowledges via `rng`.
    #[inline]
    pub fn create_random_proof_with_trapdoor(
        public_inputs: &[E::ScalarField],
        pk: &ProvingKeyWithTrapdoor<E>,
        rng: &mut impl Rng,
    ) -> R1CSResult<Proof<E>>
    where
    {
        let a = E::ScalarField::rand(rng);
        let b = E::ScalarField::rand(rng);

        Self::create_proof_with_trapdoor(pk, a, b, public_inputs)
    }

    /// Creates proof using the trapdoor
    #[cfg(test)]
    pub fn create_proof_with_trapdoor(
        pk: &ProvingKeyWithTrapdoor<E>,
        a: E::ScalarField,
        b: E::ScalarField,
        input_assignment: &[E::ScalarField],
    ) -> R1CSResult<Proof<E>> {
        let public_inputs = input_assignment;
        let mut g_ic = pk.gamma_abc_g1[0].into_group();
        for (i, b) in public_inputs.iter().zip(pk.gamma_abc_g1.iter().skip(1)) {
            g_ic.add_assign(&b.mul_bigint(i.into_bigint()));
        }
        g_ic = g_ic * pk.gamma;

        let delta_inverse = pk.delta.inverse().unwrap();
        let ab = a * b;
        let alpha_beta = pk.alpha * pk.beta;

        let g1_ab = pk.g1 * ab;
        let g1_alpha_beta = pk.g1 * alpha_beta;
        
        let g1_a = pk.g1 * a;
        let g2_b = pk.g2 * b;

        let g1_c = (g1_ab - g1_alpha_beta - g_ic) * delta_inverse; 

        Ok(Proof {
            a: g1_a.into_affine(),
            b: g2_b.into_affine(),
            c: g1_c.into_affine(),
        })
    }
}

use ark_circom::CircomConfig;
use ark_circom::CircomBuilder;
use std::fs::File;
use std::collections::HashMap;
use std::str::FromStr;
use num_bigint::BigInt;
use ark_ff::MontBackend;
use ark_bn254::FrConfig;
use crate::keyless::bn254_circom::{g1_projective_str_to_affine, g2_projective_str_to_affine};


/// Generates a trapdoor proving and verifiying key pair intended for proof simulation, in addition to a vector of public inputs, from
/// circom-generated .r1cs and .wasm files, and a .json file containing the public inputs. To be
/// used to update `test_prove_and_verify` after circuit changes occur
/// WARNING: The files referenced in this function are not present in this repo and msut be added
/// manually
#[allow(dead_code)]
fn generate_keys_and_inputs<E: Pairing>() {
    let cfg = CircomConfig::<E>::new(
        "./circuit-files/keyless_main.wasm",
        "./circuit-files/keyless_main.r1cs",
    ).unwrap();
    let mut builder = CircomBuilder::new(cfg);
    let mut input_file = File::open("./circuit-files/keyless_input.json").unwrap();
    let mut input_json = String::new();
    input_file.read_to_string(&mut input_json).unwrap();
    let input_map: HashMap<String, Vec<String>> = serde_json::from_str(&input_json).unwrap();
    for (key, values) in input_map {
        for v in values {
            let v_bigint = BigInt::from_str(&v[..]).map_err(|_| ()).unwrap();
            builder.push_input(key.clone(), v_bigint);
        }
    }

    let circom = builder.build().unwrap();
    let inputs = circom.get_public_inputs().unwrap();
    let mut rng = ark_std::rand::rngs::StdRng::seed_from_u64(test_rng().next_u64());

    let (pk, vk) = Groth16Simulator::<E>::circuit_specific_setup_with_trapdoor(circom.clone(), &mut rng).unwrap();
    println!("generated pk: {:?}", pk.clone());
    println!("generated vk: {:?}", vk.clone());
    println!("public inputs: {:?}", inputs);
}


/// Generates and verifies a simulated proof using a hardcoded simulation prover and verifier key
/// pair and a hardcoded public input. These values were generated with the Keyless circuit at commit
/// `b715e935effe282bb998bb06c826b33d290d94ed` of `aptos-core`
#[cfg(test)]
fn test_prove_and_verify<E>(n_iters: usize)
where
    E: Pairing<ScalarField = ark_ff::Fp<MontBackend<FrConfig, 4>, 4>, G2Affine = ark_ec::short_weierstrass::Affine<ark_bn254::g2::Config>, G1Affine = ark_ec::short_weierstrass::Affine<ark_bn254::g1::Config>>, <E as Pairing>::ScalarField: From<i32>
{
        let public_input_values: [u64; 4] = [3195712670376992034, 3685578554708232021, 11025712379582751444, 3215552108872721998]; 
    let public_input = ark_ff::BigInt::new(public_input_values);
    let public_input = ark_ff::Fp::<MontBackend<FrConfig, 4>, 4>::from_bigint(public_input).unwrap();

    let gamma_abc_g1_0 = g1_projective_str_to_affine("10890983729299535957423468711833583987663214856519593250327338307275052520378", "14825528083605787384494675905346505429633386239381351287094949056284905008336").unwrap();
    let gamma_abc_g1_1 = g1_projective_str_to_affine("6701484920320830429728101779419714521238246657648220634336419105800782345479", "15142509597605507689258403703394950610511337146392408727160892424844922997703").unwrap();
    let gamma_abc_g1 = vec![gamma_abc_g1_0, gamma_abc_g1_1];

    let alpha = ark_ff::BigInt::new([13589250698370566876, 10784887203457314976, 6639402089555444182, 1191924897023214780]);
    let beta = ark_ff::BigInt::new([14178762603900149007, 12962024561264135011, 14428984149348267640, 2476511004800185890]);
    let delta = ark_ff::BigInt::new([3179598508510334931, 14251246036142938839, 16048432879094000504, 631025878161227752]); 
    let gamma = ark_ff::BigInt::new([11598791714797084619, 8636816033478259993, 9421779656337856707, 1282424503525360291]);
    let g1 = g1_projective_str_to_affine("4222373349639520364951440530881871792125172922277902916438521241182902659786", "17927966855233484418691891293716534853276480020896221403452331194253900034172").unwrap();
    let g2 = g2_projective_str_to_affine(["7060239192912576352445678919251015303857900508169996987700616563495505759758", "2459845072558806286978423063428307489778927966556743480120663459709217599487"], ["19288633317757364243662951827532421887714035432540311650844990893553936393814", "20639282316004454458884347800936381746504150536012576786666607919028441606072"]).unwrap();
    let alpha = ark_ff::Fp::<MontBackend<FrConfig, 4>, 4>::from_bigint(alpha).unwrap();
    let beta = ark_ff::Fp::<MontBackend<FrConfig, 4>, 4>::from_bigint(beta).unwrap();
    let delta = ark_ff::Fp::<MontBackend<FrConfig, 4>, 4>::from_bigint(delta).unwrap();
    let gamma = ark_ff::Fp::<MontBackend<FrConfig, 4>, 4>::from_bigint(gamma).unwrap();
    let pk =  ProvingKeyWithTrapdoor { 
        gamma_abc_g1: gamma_abc_g1.clone(),
        alpha,
        beta,
        delta,
        gamma,
        g1,
        g2
    };
    let alpha_g1 = g1_projective_str_to_affine("5572059596569521478142909013551365241483584539326713643538402534559131771215", "17730641409534717358676668589645204443673285030614991773453774266753084779839").unwrap();
    let beta_g2 = g2_projective_str_to_affine(["18770618686917993373652785848897272442830690230800448000834753889342693280548", "1244082553567860317529082195476871724475625917678564162594525799206784796895"], ["2130688070722815857544427076253407755416575070301508652936230484102777632154", "6104769200283876349074000611313817829507631277250251203644007998595148003804"]).unwrap();
    let gamma_g2 = g2_projective_str_to_affine(["13321756384019475282834053010962858734065256385792198252178574019857707055625", "9904540203481000972785329888895853465145640470161185325980745361477345980499"],["13183258375250648244090549119792217999633468590401818473812106012080096645793", "9163822098487266592309953971558453292100379671136954613307467823219261972973"]).unwrap();
    let delta_g2 = g2_projective_str_to_affine(["9263958447477535187142724208180520744776704295633711436406632372106465499165", "6807912405557884826193725256367335580321369623359346147279599622449143736970"], ["18838367547891272887641438914091432084648683803724358191808362736715304958346", "7114529694217827778623886772036286266862226319773425773369673734499262479817"]).unwrap();
    let vk = VerifyingKey { alpha_g1, beta_g2, gamma_g2, delta_g2, gamma_abc_g1 };
    let pvk = prepare_verifying_key::<E>(&vk);

    let mut rng = ark_std::rand::rngs::StdRng::seed_from_u64(test_rng().next_u64());
    for _ in 0..n_iters {
        let proof = Groth16Simulator::<E>::create_random_proof_with_trapdoor(
            &[public_input],
            &pk,
            &mut rng,
        )
        .unwrap();

        assert!(Groth16::<E>::verify_with_processed_vk(&pvk, &[public_input], &proof).unwrap());
        let a = E::ScalarField::rand(&mut rng);
        assert!(!Groth16::<E>::verify_with_processed_vk(&pvk, &[a], &proof).unwrap());
    }
}

use ark_bn254::Bn254;

#[test]
fn prove_and_verify() {
    test_prove_and_verify::<Bn254>(25);
}
