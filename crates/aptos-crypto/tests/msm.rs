// Copyright © Aptos Foundation

use ark_bn254::{Fr, G1Affine, G1Projective};
use ark_std::{test_rng, UniformRand};
use rayon::prelude::*;

macro_rules! rand {
    ($typ:ty) => {{
        <$typ>::rand(&mut test_rng())
    }};
}

#[test]
#[ignore]
fn bench_ark_bn254_msm_g1() {
    //let num_entries = 16384;
    let num_entries = 16_777_216;

    println!("Sampling {} bases", num_entries);
    let start = std::time::Instant::now();
    let elements = (0..num_entries)
        .into_par_iter()
        .map(|_i| rand!(G1Affine))
        .collect::<Vec<_>>();
    println!("Sampling bases took: {:?}", start.elapsed());

    println!("Sampling {} scalars", num_entries);
    let start = std::time::Instant::now();
    let scalars = (0..num_entries)
        .into_par_iter()
        .map(|_i| rand!(Fr))
        .collect::<Vec<_>>();
    println!("Sampling scalars took: {:?}", start.elapsed());

    println!("Benchmarking ark_bn254 msm_g1 on {} bases", num_entries);
    let start = std::time::Instant::now();
    let _res: G1Projective =
        ark_ec::VariableBaseMSM::msm(elements.as_slice(), scalars.as_slice()).unwrap();
    let time = start.elapsed();

    println!("G1 MSM took: {:?}", time);
    println!("Avg G1 scalar mul time: {:?}", time / num_entries as u32);
}
