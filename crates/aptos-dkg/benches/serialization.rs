use aptos_crypto::{arkworks::{random::{sample_field_element, unsafe_random_points}, shamir::ShamirThresholdConfig}, weighted_config::WeightedConfig};
use ark_bls12_381::G1Projective;
use ark_bn254::{G1Affine as ArkG1Affine, G2Affine as ArkG2Affine, G1Projective as ArkG1Projective, G2Projective as ArkG2Projective, Fr as ArkFr};
use ark_serialize::CanonicalSerialize;
use ark_std::UniformRand;
use ark_ec::CurveGroup;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use aptos_dkg::pvss::test_utils::{BENCHMARK_CONFIGS, get_weighted_configs_for_benchmarking};
use aptos_crypto::arkworks::random::sample_field_elements;
use aptos_dkg::pvss::chunky::chunked_elgamal::num_chunks_per_scalar;
use criterion::BatchSize;

#[derive(CanonicalSerialize)]
struct ArkBn254Proj {
    proj_vec: Vec<ArkG1Projective>,
}

#[derive(CanonicalSerialize)]
struct ArkBn254G1Affine {
    affine_vec: Vec<ArkG1Affine>,
}

#[derive(CanonicalSerialize)]
struct TranscriptSimulatorProj {
    G1_vec: Vec<ArkG1Projective>,
    G2_vec: Vec<ArkG2Projective>,
    scalar_vec: Vec<ArkFr>,
}

#[derive(CanonicalSerialize)]
struct TranscriptSimulatorAffine {
    G1_vec: Vec<ArkG1Affine>,
    G2_vec: Vec<ArkG2Affine>,
    scalar_vec: Vec<ArkFr>,
}

fn bench_projective_serialization_arkworks(c: &mut Criterion) {
    let mut rng = ark_std::rand::thread_rng();

    let points: Vec<ArkG1Projective> = (0..1_000)
        .map(|_| ArkG1Projective::rand(&mut rng) * ArkFr::from(2) ) // To ensure that the points are not "implicitly" affine
        .collect();

    c.bench_function("serialize 1k Arkworks Bn254 G1Projective (compressed)", |b| {
        b.iter(|| {
            let mut bytes = Vec::new();
            let points_struct = ArkBn254Proj {
                proj_vec: black_box(points.clone()),
            };

            points_struct
                .serialize_compressed(&mut bytes)
                .unwrap();

            black_box(bytes);
        })
    });


    let (_,n) = BENCHMARK_CONFIGS[0];
    let max_weight = 7;
    let ell = 16;
    let num_chunks: usize = num_chunks_per_scalar::<ark_bn254::Fr>(ell) as usize;
    let number_of_G2_elements = 2*(n) + 1 + 1 + n * num_chunks + num_chunks * max_weight; // V0, DEKART commit, 
    let number_of_G1_elements = 2* (n * num_chunks + num_chunks * max_weight) + (ell as usize) + 2 + 2 + 2;
    let number_of_scalar_elements = (ell as usize) + 2 + 2;

    let mut rng = rand::thread_rng();

    c.bench_function(
        "serialize chunky transcript simulator (compressed)",
        |b| {
            b.iter_batched(
                || {
                    // Setup: generate points and field elements once per iteration
                    TranscriptSimulatorProj {
                        G1_vec: unsafe_random_points::<ArkG1Projective, _>(number_of_G1_elements, &mut rng).into_iter()
    .map(|p| p * ArkFr::from(2) )
    .collect::<Vec<_>>(),
                        G2_vec: unsafe_random_points::<ArkG2Projective, _>(number_of_G1_elements, &mut rng).into_iter()
    .map(|p| p * ArkFr::from(2) )
    .collect::<Vec<_>>(),
                        scalar_vec: sample_field_elements(number_of_scalar_elements, &mut rng),
                    }
                },
                |points_struct| {
                    // The actual benchmark: serialize
                    let mut bytes = Vec::new();
                    points_struct.serialize_compressed(&mut bytes).unwrap();
                    black_box(bytes);
                },
                BatchSize::SmallInput, // or LargeInput depending on size
            )
        },
    );
}

fn bench_affine_serialization_arkworks(c: &mut Criterion) {
    let mut rng = ark_std::rand::thread_rng();

    let points_proj: Vec<ArkG1Projective> = (0..1_000)
        .map(|_| ArkG1Projective::rand(&mut rng)* ArkFr::from(2) ) // To ensure that the points are not "implicitly" affine
        .collect();


    c.bench_function("serialize 1k Arkworks Bn254 G1Affine (compressed)", |b| {
        b.iter(|| {
            let points_affine = ArkG1Projective::normalize_batch(&points_proj);

            let mut bytes = Vec::new();
            let points_struct = ArkBn254G1Affine {
                affine_vec: black_box(points_affine.clone()),
            };

            points_struct
                .serialize_compressed(&mut bytes)
                .unwrap();

            black_box(bytes);
        })
    });

    let (_,n) = BENCHMARK_CONFIGS[0];
    let max_weight = 7;
    let ell = 16;
    let num_chunks: usize = num_chunks_per_scalar::<ark_bn254::Fr>(ell) as usize;
    let number_of_G2_elements = 2*(n) + 1 + 1 + n * num_chunks + num_chunks * max_weight; // V0, DEKART commit, 
    let number_of_G1_elements = 2* (n * num_chunks + num_chunks * max_weight) + (ell as usize) + 2 + 2 + 2;
    let number_of_scalar_elements = (ell as usize) + 2 + 2;

    let mut rng = rand::thread_rng();

    c.bench_function(
        "serialize chunky transcript simulator (compressed)",
        |b| {
            b.iter_batched(
                || {
                    // Setup: generate points and field elements once per iteration
                        let G1_vec = unsafe_random_points::<ArkG1Projective, _>(number_of_G1_elements, &mut rng).into_iter()
    .map(|p| p * ArkFr::from(2) )
    .collect::<Vec<_>>();
                        let G2_vec = unsafe_random_points::<ArkG2Projective, _>(number_of_G1_elements, &mut rng).into_iter()
    .map(|p| p * ArkFr::from(2) )
    .collect::<Vec<_>>();
                        let scalar_vec = sample_field_elements(number_of_scalar_elements, &mut rng);
                        (G1_vec, G2_vec, scalar_vec)
                },
                |(G1_vec, G2_vec, scalar_vec)| {
                    let points_struct = TranscriptSimulatorAffine {
                        G1_vec:  ArkG1Projective::normalize_batch(&G1_vec),
                        G2_vec: ArkG2Projective::normalize_batch(&G2_vec),
                        scalar_vec,
                    };
                    // The actual benchmark: serialize
                    let mut bytes = Vec::new();
                    points_struct.serialize_compressed(&mut bytes).unwrap();
                    black_box(bytes);
                },
                BatchSize::SmallInput, // or LargeInput depending on size
            )
        },
    );
}

criterion_group!(
    benches,
    bench_projective_serialization_arkworks,
    bench_affine_serialization_arkworks
);
criterion_main!(benches);