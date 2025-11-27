use aptos_batch_encryption::group::{
    Fr, G1Affine, G1Projective, G2Affine, G2Prepared, PairingOutput, PairingSetting
};
use ark_ec::{
    pairing::{Pairing},
    VariableBaseMSM,
};
use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
use ark_std::{rand::thread_rng, UniformRand};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};


pub fn critical_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("BEAT-MEV::critical_path");
    let mut rng = thread_rng();

        let g1 = G1Affine::rand(&mut rng);
        let g2 = G2Prepared::from(G2Affine::rand(&mut rng));

        group.bench_with_input(
            BenchmarkId::from_parameter(1),
            &(g1, g2),
            |b, input| {
                b.iter(|| {
                    (0..128)
                        .into_par_iter()
                        .map(|_| {
                            let pad =
                                PairingSetting::pairing(&input.0, input.1.clone());
                            let pad2 = pad + pad;
                            pad2
                        })
                        .collect::<Vec<PairingOutput>>()
                });
            },
        );
}


pub fn ifft_gt(c: &mut Criterion) {
    let mut group = c.benchmark_group("BEAT-MEV::ifft_gt");
    let mut rng = thread_rng();

    for batch_size in [32, 128, 512] {
        let gt = vec![ PairingOutput::rand(&mut rng); batch_size ];
        let domain : Radix2EvaluationDomain<Fr> = Radix2EvaluationDomain::new(batch_size).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &(gt, domain),
            |b, input| {
                b.iter(|| {
                    input.1.ifft(&input.0)
                });
            },
        );
    }
}



criterion_group!(benches, critical_path, ifft_gt);
criterion_main!(benches);
