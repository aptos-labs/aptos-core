use ark_groth16::{prepare_verifying_key, Groth16};
use ark_crypto_primitives::snark::{CircuitSpecificSetupSNARK, SNARK};
use ark_ec::pairing::Pairing;
use ark_ff::Field;
use ark_relations::{
    lc,
    r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError},
};
use ark_std::{
    rand::{RngCore, SeedableRng},
    test_rng, UniformRand,
};

struct MySillyCircuit<F: Field> {
    a: Option<F>,
    b: Option<F>,
}

impl<ConstraintF: Field> ConstraintSynthesizer<ConstraintF> for MySillyCircuit<ConstraintF> {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<ConstraintF>,
    ) -> Result<(), SynthesisError> {
        let a = cs.new_witness_variable(|| self.a.ok_or(SynthesisError::AssignmentMissing))?;
        let b = cs.new_witness_variable(|| self.b.ok_or(SynthesisError::AssignmentMissing))?;
        let c = cs.new_input_variable(|| {
            let mut a = self.a.ok_or(SynthesisError::AssignmentMissing)?;
            let b = self.b.ok_or(SynthesisError::AssignmentMissing)?;

            a *= &b;
            Ok(a)
        })?;

        cs.enforce_constraint(lc!() + a, lc!() + b, lc!() + c)?;
        cs.enforce_constraint(lc!() + a, lc!() + b, lc!() + c)?;
        cs.enforce_constraint(lc!() + a, lc!() + b, lc!() + c)?;
        cs.enforce_constraint(lc!() + a, lc!() + b, lc!() + c)?;
        cs.enforce_constraint(lc!() + a, lc!() + b, lc!() + c)?;
        cs.enforce_constraint(lc!() + a, lc!() + b, lc!() + c)?;

        Ok(())
    }
}

//use ark_crypto_primitives::snark::*;
//use ark_ec::pairing::Pairing;
//use ark_relations::r1cs::{ConstraintSynthesizer, SynthesisError};
//use ark_std::rand::RngCore;
use ark_std::{marker::PhantomData, vec::Vec};
use ark_groth16::r1cs_to_qap::{LibsnarkReduction, R1CSToQAP};
use ark_groth16::data_structures::{ProvingKey, VerifyingKey, Proof};
use ark_serialize::*;
use ark_std::rand::Rng;
use ark_relations::r1cs::{ConstraintSystem, OptimizationGoal, Result as R1CSResult};
use ark_ec::{AffineRepr,CurveGroup};
use ark_ff::PrimeField;
use std::ops::AddAssign;

/// The SNARK of [[Groth16]](https://eprint.iacr.org/2016/260.pdf).
pub struct Groth16Simulator<E: Pairing, QAP: R1CSToQAP = LibsnarkReduction> {
    _p: PhantomData<(E, QAP)>,
}

/// The prover key for for the Groth16 zkSNARK.
#[derive(Clone, Debug, PartialEq, CanonicalSerialize, CanonicalDeserialize)]
pub struct ProvingKeyWithTrapdoor<E: Pairing> {
    /// The underlying proving key
    pub pk: ProvingKey<E>,
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
        let pk = Self::generate_random_parameters_and_trapdoor_with_reduction(circuit, rng)?;
        let vk = pk.pk.vk.clone();

        Ok((pk, vk))
    }

/// Generates a random common reference string for
    /// a circuit using the provided R1CS-to-QAP reduction.
    #[inline]
    pub fn generate_random_parameters_and_trapdoor_with_reduction<C>(
        circuit: C,
        rng: &mut impl Rng,
    ) -> R1CSResult<ProvingKeyWithTrapdoor<E>>
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

        Ok(
        ProvingKeyWithTrapdoor {
            pk,
            alpha,
            beta,
            delta,
            gamma,
            g1: g1_generator.into_affine(),
            g2: g2_generator.into_affine(),
        })
    }


pub fn prove_with_trapdoor<C: ConstraintSynthesizer<E::ScalarField>, R: RngCore>(
        pk: &ProvingKeyWithTrapdoor<E>,
        circuit: C,
        rng: &mut R,
    ) -> Result<Proof<E>, SynthesisError> {
        Self::create_random_proof_with_trapdoor(circuit, pk, rng)
    }

    /// Create a Groth16 proof that is zero-knowledge using the provided
    /// R1CS-to-QAP reduction.
    /// This method samples randomness for zero knowledges via `rng`.
    #[inline]
    pub fn create_random_proof_with_trapdoor<C>(
        circuit: C,
        pk: &ProvingKeyWithTrapdoor<E>,
        rng: &mut impl Rng,
    ) -> R1CSResult<Proof<E>>
    where
        C: ConstraintSynthesizer<E::ScalarField>,
    {
        let a = E::ScalarField::rand(rng);
        let b = E::ScalarField::rand(rng);

        let cs = ConstraintSystem::new_ref();

        // Set the optimization goal
        cs.set_optimization_goal(OptimizationGoal::Constraints);

        // Synthesize the circuit.
        //let synthesis_time = start_timer!(|| "Constraint synthesis");
        circuit.generate_constraints(cs.clone())?;
        debug_assert!(cs.is_satisfied().unwrap());
        //end_timer!(synthesis_time);

        //let lc_time = start_timer!(|| "Inlining LCs");
        cs.finalize();
        //end_timer!(lc_time);

        //let witness_map_time = start_timer!(|| "R1CS to QAP witness map");
        //end_timer!(witness_map_time);

        let prover = cs.borrow().unwrap();

        Self::create_proof_with_trapdoor(pk, a, b, &prover.instance_assignment[1..])
    }

    /// Creates proof using the trapdoor
    pub fn create_proof_with_trapdoor(
        pk: &ProvingKeyWithTrapdoor<E>,
        a: E::ScalarField,
        b: E::ScalarField,
        input_assignment: &[E::ScalarField],
    ) -> R1CSResult<Proof<E>> {
        let public_inputs = input_assignment;
        let mut g_ic = pk.pk.vk.gamma_abc_g1[0].into_group();
        for (i, b) in public_inputs.iter().zip(pk.pk.vk.gamma_abc_g1.iter().skip(1)) {
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
use std::io::prelude::*;
use std::collections::HashMap;
use std::str::FromStr;
//use ark_ff::{BigInt,BigInteger};
//use num_bigint::{BigInt,BigUint};
use num_bigint::BigInt;
use ark_ff::MontBackend;
use ark_bn254::FrConfig;

fn u8_to_bits(x: u8) -> Vec<bool> {
    let mut result = Vec::new();
    for i in 0..8 {
        let mask = 1 << x;
        let bit_is_set = (mask & x) > 0;
        result.push(bit_is_set);
    }
    result
}

fn test_prove_and_verify<E>(n_iters: usize)
where
    E: Pairing<ScalarField = ark_ff::Fp<MontBackend<FrConfig, 4>, 4>>, <E as Pairing>::ScalarField: From<i32>
{
    println!("starting test");

    let cfg = CircomConfig::<Bn254>::new(
    //"./circuit-files/keyless_main.wasm",
    //"./proof_simulation.rs",
    //"/Users/michael/aptos-labs/aptos-core/types/src/keyless/proof_simulation.rs",
    //"/Users/michael/aptos-labs/aptos-core/types/src/keyless/circuit-files/keyless_main.wasm",
    "/Users/michael/aptos-labs/aptos-core/types/src/keyless/toy-circuit-files/multiplier2_js/multiplier2.wasm",
    //"./circuit-files/keyless_main.r1cs",
    //"./proof_simulation.rs",
    //"/Users/michael/aptos-labs/aptos-core/types/src/keyless/circuit-files/keyless_main.r1cs",
    "/Users/michael/aptos-labs/aptos-core/types/src/keyless/toy-circuit-files/multiplier2.r1cs",
).unwrap();
    /*let cfg = CircomConfig::<Bn254>::new(
    "/Users/michael/aptos-labs/aptos-core/types/src/keyless/circuit-files/keyless_main.wasm",
    "/Users/michael/aptos-labs/aptos-core/types/src/keyless/circuit-files/keyless_main.r1cs",
).unwrap();*/

    let mut builder = CircomBuilder::new(cfg);
    //let mut input_file = File::open("/Users/michael/aptos-labs/aptos-core/types/src/keyless/circuit-files/keyless_input.json").unwrap();
    let mut input_file = File::open("/Users/michael/aptos-labs/aptos-core/types/src/keyless/toy-circuit-files/multiplier2_input.json").unwrap();
    let mut input_json = String::new();
    input_file.read_to_string(&mut input_json).unwrap();
    let input_map: HashMap<String, Vec<String>> = serde_json::from_str(&input_json).unwrap();
    for (key, values) in input_map {
        for v in values {
            let v_bigint = BigInt::from_str(&v[..]).map_err(|_| ()).unwrap();
            /*let v_biguint_bytes = v_biguint.to_bytes_le();
            let mut v_biguint_bits = Vec::new();
            for byte in v_biguint_bytes {
                let mut v_bits = u8_to_bits(byte);
                v_biguint_bits.append(&mut v_bits);
            }
            let v_bigint = ark_ff::BigInt::from_bits_le(&v_biguint_bits);*/
            //let v_bigint = ark_ff::BigInt::try_from(v_biguint);
            builder.push_input(key.clone(), v_bigint);
        }
    }

    let circom = builder.setup();
    println!("circom: {:?}", circom);
    let witness: Vec<<E as Pairing>::ScalarField> = builder
                 .cfg 
                 .wtns
                 .calculate_witness_element::<E, _>(builder.inputs.clone(), builder.cfg.sanity_check).unwrap(); 
    println!("witness: {:?}", witness);
    let circom = builder.build().unwrap();
    let inputs = circom.get_public_inputs().unwrap();
    println!("{:?}", inputs);
    let mut rng = ark_std::rand::rngs::StdRng::seed_from_u64(test_rng().next_u64());

    //let (pk, vk) = Groth16Simulator::<E>::circuit_specific_setup_with_trapdoor(MySillyCircuit { a: None, b: None }, &mut rng).unwrap();
    let (pk, vk) = Groth16Simulator::<E>::circuit_specific_setup_with_trapdoor(circom.clone(), &mut rng).unwrap();
    let pvk = prepare_verifying_key::<E>(&vk);

    for _ in 0..n_iters {
        let a = E::ScalarField::from(3);//rand(&mut rng);
        let b = E::ScalarField::from(4);//rand(&mut rng);
        let mut c = a;
        c *= b;

        /*let proof = Groth16Simulator::<E>::prove_with_trapdoor(
            &pk,
            MySillyCircuit {
                a: Some(a),
                b: Some(b),
            },
            &mut rng,
        )
        .unwrap();*/

        let proof = Groth16Simulator::<E>::prove_with_trapdoor(
            &pk,
            circom.clone(),
            &mut rng,
        )
        .unwrap();

        assert!(Groth16::<E>::verify_with_processed_vk(&pvk, &[c], &proof).unwrap());
        assert!(!Groth16::<E>::verify_with_processed_vk(&pvk, &[a], &proof).unwrap());
    }
}

use ark_bn254::Bn254;

#[test]
fn prove_and_verify() {
    test_prove_and_verify::<Bn254>(1);
}
