// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
extern crate criterion;

use ark_bn254::Bn254;
use ark_ff::Field;
use ark_groth16::Groth16;
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystemRef, LinearCombination, SynthesisError, Variable,
};
use ark_snark::SNARK;
use ark_std::rand::{thread_rng, Rng};
use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, BenchmarkId, Criterion,
};

/// Target dimensions for the Keyless NP relation
const NUM_CONSTRAINTS: usize = 1_438_805;
const NUM_VARIABLES: usize = 1_406_686; // includes 1 public input

/// Helper: produce a witness of length (NUM_VARIABLES - 1) with 128-bit field elements.
fn random_128bit_witness<F: Field>(len: usize) -> Vec<F> {
    let mut rng = thread_rng();
    // Sample u128 and embed into Fr; BN254 field is ~254 bits, so u128 values fit naturally.
    (0..len)
        .map(|_| {
            let x: u128 = rng.r#gen();
            F::from(x)
        })
        .collect()
}

/// Minimal dummy circuit with precise size.
#[derive(Clone)]
struct SizedCircuit<F>
where
    F: Field,
{
    num_constraints: usize,
    num_variables: usize,
    // Optional witness vector; when None, witnesses default to 1.
    // When Some(vec), `vec.len()` must be num_variables - 1 (excluding x0).
    witness: Option<Vec<F>>,
}

impl<F: Field> ConstraintSynthesizer<F> for SizedCircuit<F> {
    fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
        // Public input x0 = 1
        let x0 = cs.new_input_variable(|| Ok(F::one()))?;

        // Allocate witnesses so total vars == num_variables
        // If self.witness is provided, use those values. Otherwise use 1s.
        let mut vars = Vec::with_capacity(self.num_variables);
        vars.push(x0);

        if let Some(w) = self.witness {
            assert_eq!(w.len(), self.num_variables - 1, "witness length mismatch");
            for val in w {
                vars.push(cs.new_witness_variable(|| Ok(val))?);
            }
        } else {
            for _ in 0..(self.num_variables - 1) {
                vars.push(cs.new_witness_variable(|| Ok(F::one()))?);
            }
        }

        // Enforce `v * 1 = v` repeatedly to reach exactly num_constraints constraints.
        // This preserves the matrix size but does NOT force any relation between
        // different variables—so arbitrary (e.g., 128-bit) witnesses are valid.
        let one: LinearCombination<F> = LinearCombination::from((F::one(), Variable::One));
        // We'll cycle across the witness indices [1..num_variables) to touch them uniformly.
        let w_count = self.num_variables.saturating_sub(1).max(1);
        for i in 0..self.num_constraints {
            let idx = 1 + (i % w_count); // skip x0 at index 0
            let v: LinearCombination<F> = LinearCombination::from(vars[idx]);
            cs.enforce_constraint(v.clone(), one.clone(), v)?;
        }

        Ok(())
    }
}

/// Build the proving key once (excluded from the benchmark measurements).
fn setup_bn254_pk() -> (SizedCircuit<ark_bn254::Fr>, ark_groth16::ProvingKey<Bn254>) {
    let mut rng = thread_rng();
    // Shape-only circuit (no witness values needed)
    let circuit = SizedCircuit {
        num_variables: NUM_VARIABLES,
        num_constraints: NUM_CONSTRAINTS,
        witness: None,
    };
    let (pk, _vk) = Groth16::<Bn254>::circuit_specific_setup(circuit.clone(), &mut rng)
        .expect("setup should succeed");

    println!("a_query: {}", pk.a_query.len());
    println!("b_g1_query: {}", pk.b_g1_query.len());
    println!("b_g2_query: {}", pk.b_g2_query.len());
    // TODO: |h_query| = the smallest power of 2 less than the # of constraints. Why? And what is it?
    println!("h_query: {}", pk.h_query.len());
    println!("l_query: {}", pk.l_query.len());

    (circuit, pk)
}

fn bench_group(c: &mut Criterion) {
    // One-time setup (not timed)
    let (mut circuit, pk) = setup_bn254_pk();

    let mut group = c.benchmark_group("ark_groth16");

    // Tweak these knobs if runs are too long/short on your machine.
    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_secs(2));
    group.measurement_time(std::time::Duration::from_secs(30));
    group.throughput(criterion::Throughput::Elements(NUM_CONSTRAINTS as u64));

    // Benchmark the circuit with a norm-1 witness vector
    bench_bn254_circuit(circuit.clone(), &pk, &mut group, "1");

    // Benchmark the circuit with a "large-norm" witness vector: each entry is 128-bit.
    let witness = random_128bit_witness(NUM_VARIABLES - 1);
    circuit.witness = Some(witness);
    bench_bn254_circuit(circuit, &pk, &mut group, "128-bit");

    group.finish();
}

fn bench_bn254_circuit(
    circuit: SizedCircuit<ark_bn254::Fr>,
    pk: &ark_groth16::ProvingKey<Bn254>,
    group: &mut BenchmarkGroup<WallTime>,
    norm: &str,
) {
    group.bench_function(
        BenchmarkId::new("bn254/prove/multithreaded/norm", norm.to_string()),
        |b| {
            b.iter(|| {
                // Clone because Groth16::prove consumes the circuit
                let proof = Groth16::<Bn254>::prove(pk, circuit.clone(), &mut thread_rng())
                    .expect("proving");
                criterion::black_box(proof);
            });
        },
    );
}

criterion_group!(
    name = ark_groth16;
    config = Criterion::default().sample_size(10);
    targets = bench_group);
criterion_main!(ark_groth16);
