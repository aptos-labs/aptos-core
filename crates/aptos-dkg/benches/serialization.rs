use ark_bn254::{G1Affine as ArkG1Affine, G1Projective as ArkG1Projective, Fr as ArkFr};
use ark_serialize::CanonicalSerialize;
use ark_std::UniformRand;
use ark_ec::CurveGroup;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[derive(CanonicalSerialize)]
struct ArkBn254G1 {
    proj_vec: Vec<ArkG1Projective>,
}

#[derive(CanonicalSerialize)]
struct ArkBn254G1Affine {
    affine_vec: Vec<ArkG1Affine>,
}

fn bench_projective_serialization_arkworks(c: &mut Criterion) {
    let mut rng = ark_std::rand::thread_rng();

    let points: Vec<ArkG1Projective> = (0..1_000)
        .map(|_| ArkG1Projective::rand(&mut rng) * ArkFr::from(2) ) // To ensure that the points are not "implicitly" affine
        .collect();

    c.bench_function("serialize 1k Arkworks Bn254 G1Projective (compressed)", |b| {
        b.iter(|| {
            let mut bytes = Vec::new();
            let points_struct = ArkBn254G1 {
                proj_vec: black_box(points.clone()),
            };

            points_struct
                .serialize_compressed(&mut bytes)
                .unwrap();

            black_box(bytes);
        })
    });
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
}

criterion_group!(
    benches,
    bench_projective_serialization_arkworks,
    bench_affine_serialization_arkworks
);
criterion_main!(benches);