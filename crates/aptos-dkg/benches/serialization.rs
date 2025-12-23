use aptos_crypto::{
    arkworks::random::{sample_field_element, sample_field_elements, unsafe_random_points_group},
    blstrs::{
        random::{insecure_random_g1_points, insecure_random_g2_points, random_scalars},
        random_scalar,
    },
};
use aptos_dkg::pvss::{
    chunky::chunked_elgamal::num_chunks_per_scalar, test_utils::BENCHMARK_CONFIGS,
};
use ark_bn254::{
    Fr as ArkFr, G1Affine as ArkG1Affine, G1Projective as ArkG1Projective, G2Affine as ArkG2Affine,
    G2Projective as ArkG2Projective,
};
use ark_ec::CurveGroup;
use ark_serialize::CanonicalSerialize;
use blstrs::{
    G1Affine as BlstrsG1Affine, G1Projective as BlstrsG1Projective, G2Affine as BlstrsG2Affine,
    G2Projective as BlstrsG2Projective, Scalar as BlstrsFr,
};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use ff::Field;
use group::{prime::PrimeCurveAffine, Group};
use serde::Serialize;

// The goal of this file is
// (i) To test how much speed is gained serializing projective group elements in `arkworks` and `blstrs` manually using a `normalize_batch()` type approach
// (ii) Whilst we're at it, see how arkworks and blstrs differ speedwise by timing the serialisation of the same amount of elts (note that we are NOT using the same curve atm)
// blstrs doesn't have a proper normalize_batch() function implemented, so we've commented out those benches for now

// `blstrs` seems to be missing a `normalize_batch()` implementation, so let's do that first
fn blstrs_normalize_batch_g1(v: &[BlstrsG1Projective]) -> Vec<BlstrsG1Affine> {
    // Collect all z coordinates
    let mut zs: Vec<_> = v.iter().map(|p| p.z()).collect();

    // Do batch inversion
    //zs.iter_mut().batch_invert();
    batch_inversion(&mut zs);

    // Convert to affine
    v.iter()
        .zip(zs)
        .map(|(p, z_inv)| {
            if bool::from(p.is_identity()) {
                BlstrsG1Affine::identity()
            } else {
                let x = p.x() * z_inv;
                let y = p.y() * z_inv;
                // SAFETY: this assumes x,y is a valid affine point
                BlstrsG1Affine::from_raw_unchecked(x, y, true)
            }
        })
        .collect()
}

fn blstrs_normalize_batch_g2(v: &[BlstrsG2Projective]) -> Vec<BlstrsG2Affine> {
    // Collect all z coordinates
    let mut zs: Vec<_> = v.iter().map(|p| p.z()).collect();

    // Do batch inversion
    //zs.iter_mut().batch_invert();
    batch_inversion(&mut zs);

    // Convert to affine
    v.iter()
        .zip(zs)
        .map(|(p, z_inv)| {
            if bool::from(p.is_identity()) {
                BlstrsG2Affine::identity()
            } else {
                let x = p.x() * z_inv;
                let y = p.y() * z_inv;
                // SAFETY: this assumes x,y is a valid affine point
                BlstrsG2Affine::from_raw_unchecked(x, y, true)
            }
        })
        .collect()
}

// Furthermore, this function seems to be slightly faster than the built-in `batch_invert()`, probably
// because it's not constant-time, but that's irrelevant for serialization
fn batch_inversion<F: Field>(v: &mut [F]) {
    let mut acc = F::ONE;
    // prefix products
    let mut prod = Vec::with_capacity(v.len());
    for x in v.iter() {
        prod.push(acc);
        acc *= x;
    }
    // invert the total product
    acc = acc.invert().unwrap(); // shouldn't happen, the only element with zero z-coordinate in the Weierstrass model is the identity (0 : 1 : 0)
                                 // propagate inverses backwards
    for (x, p) in v.iter_mut().rev().zip(prod.into_iter().rev()) {
        let tmp = acc * *x;
        *x = acc * p;
        acc = tmp;
    }
}

const N: usize = 10_000;

fn ark_g1_projective_1k<R: rand_core::RngCore + rand_core::CryptoRng>(
    rng: &mut R,
) -> Vec<ArkG1Projective> {
    random_projective_vec_ark::<ArkG1Projective, _>(N, rng)
}

fn blstrs_g1_projective_1k<R: rand_core::RngCore + rand_core::CryptoRng>(
    rng: &mut R,
) -> Vec<BlstrsG1Projective> {
    random_projective_g1_vec_blstrs(N, rng)
}

fn bench_arkworks_projective_1k(c: &mut Criterion) {
    let mut rng = rand::thread_rng();

    c.bench_function("arkworks serialize 1k G1 projective", |b| {
        b.iter_batched(
            || ark_g1_projective_1k(&mut rng),
            |v| {
                let mut bytes = Vec::new();
                v.serialize_compressed(&mut bytes).unwrap();
                black_box(bytes);
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_arkworks_affine_1k(c: &mut Criterion) {
    let mut rng = rand::thread_rng();

    c.bench_function("arkworks serialize 1k G1 affine", |b| {
        b.iter_batched(
            || ark_g1_projective_1k(&mut rng),
            |proj| {
                let mut bytes = Vec::new();
                let v = ArkG1Projective::normalize_batch(&proj);
                v.serialize_compressed(&mut bytes).unwrap();
                black_box(bytes);
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_blstrs_projective_1k(c: &mut Criterion) {
    let mut rng = rand::thread_rng();

    c.bench_function("blstrs serialize 1k G1 projective (BCS)", |b| {
        b.iter_batched(
            || blstrs_g1_projective_1k(&mut rng),
            |v| {
                let bytes = bcs::to_bytes(&v).unwrap();
                black_box(bytes);
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_blstrs_affine_1k(c: &mut Criterion) {
    let mut rng = rand::thread_rng();

    c.bench_function("blstrs serialize 1k G1 affine (BCS)", |b| {
        b.iter_batched(
            || blstrs_g1_projective_1k(&mut rng),
            |proj| {
                //let mut aff = vec![BlstrsG1Affine::generator(); N];
                let aff = blstrs_normalize_batch_g1(&proj);
                let bytes = bcs::to_bytes(&aff).unwrap();
                black_box(bytes);
            },
            BatchSize::SmallInput,
        )
    });
}

// Naive serialization of a transcript of arkworks elements
#[derive(CanonicalSerialize)]
#[allow(non_snake_case)]
struct ArkTranscriptSimulatorProj {
    G1_vec: Vec<ArkG1Projective>,
    G2_vec: Vec<ArkG2Projective>,
    scalar_vec: Vec<ArkFr>, // Only including this to make the benchmarks more "realistic" by making the numbers similar to chunky
}

// Serialization of a transcript of affine arkworks elements
#[derive(CanonicalSerialize)]
#[allow(non_snake_case)]
struct ArkTranscriptSimulatorAffine {
    G1_vec: Vec<ArkG1Affine>,
    G2_vec: Vec<ArkG2Affine>,
    scalar_vec: Vec<ArkFr>,
}

struct ChunkySizes {
    g1: usize,
    g2: usize,
    scalars: usize,
}

// TODO: Benchmarks are slightly off so there's probably a small bug in these numbers. Not very important
fn chunky_sizes() -> ChunkySizes {
    let (_, n) = BENCHMARK_CONFIGS[0];
    let max_weight = 7;
    let ell = 16;

    let num_chunks = num_chunks_per_scalar::<ArkFr>(ell) as usize;

    let g2 = 2 * n + 1 + 1 + n * num_chunks + num_chunks * max_weight;
    let g1 = 2 * (n * num_chunks + num_chunks * max_weight) + ell as usize + 2 + 2 + 2;
    let scalars = ell as usize + 2 + 2;

    ChunkySizes { g1, g2, scalars }
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct BlstrsTranscriptSimulatorProj {
    G1_vec: Vec<BlstrsG1Projective>,
    G2_vec: Vec<BlstrsG2Projective>,
    scalar_vec: Vec<BlstrsFr>,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct BlstrsTranscriptSimulatorAffine {
    G1_vec: Vec<BlstrsG1Affine>,
    G2_vec: Vec<BlstrsG2Affine>,
    scalar_vec: Vec<BlstrsFr>,
}

struct DasSizes {
    g1: usize,
    g2: usize,
    scalars: usize,
}

// We ignore the SoKs
fn das_sizes() -> DasSizes {
    let (_, n) = BENCHMARK_CONFIGS[0];

    let g1 = (n + n + 1 + n) * 1; // The `* 1` is for experimenting
    let g2 = (n + n + 1) * 1;

    DasSizes { g1, g2, scalars: 0 }
}

fn random_projective_vec_ark<C: CurveGroup, R>(n: usize, rng: &mut R) -> Vec<C>
where
    R: rand_core::RngCore + rand_core::CryptoRng,
{
    unsafe_random_points_group::<C, _>(n, rng)
        .into_iter()
        .map(|p| p * sample_field_element::<C::ScalarField, _>(rng))
        .collect()
}

fn random_projective_g1_vec_blstrs<R>(n: usize, rng: &mut R) -> Vec<BlstrsG1Projective>
where
    R: rand_core::RngCore + rand_core::CryptoRng,
{
    insecure_random_g1_points(n, rng)
        .into_iter()
        .map(|p| p * random_scalar(rng))
        .collect()
}

fn random_projective_g2_vec_blstrs<R>(n: usize, rng: &mut R) -> Vec<BlstrsG2Projective>
where
    R: rand_core::RngCore + rand_core::CryptoRng,
{
    insecure_random_g2_points(n, rng)
        .into_iter()
        .map(|p| p * random_scalar(rng))
        .collect()
}

fn random_projective_chunky_transcript<R>(
    sizes: &ChunkySizes,
    rng: &mut R,
) -> ArkTranscriptSimulatorProj
where
    R: rand_core::RngCore + rand_core::CryptoRng,
{
    ArkTranscriptSimulatorProj {
        G1_vec: random_projective_vec_ark::<ArkG1Projective, _>(sizes.g1, rng),
        G2_vec: random_projective_vec_ark::<ArkG2Projective, _>(sizes.g2, rng),
        scalar_vec: sample_field_elements(sizes.scalars, rng),
    }
}

fn random_projective_das_transcript<R>(
    sizes: &DasSizes,
    rng: &mut R,
) -> BlstrsTranscriptSimulatorProj
where
    R: rand_core::RngCore + rand_core::CryptoRng,
{
    BlstrsTranscriptSimulatorProj {
        G1_vec: random_projective_g1_vec_blstrs(sizes.g1, rng),
        G2_vec: random_projective_g2_vec_blstrs(sizes.g2, rng),
        scalar_vec: random_scalars(sizes.scalars, rng),
    }
}

fn affine_chunky_transcript_from_projective_transcript(
    projective_transcript: &ArkTranscriptSimulatorProj,
) -> ArkTranscriptSimulatorAffine {
    ArkTranscriptSimulatorAffine {
        G1_vec: ArkG1Projective::normalize_batch(&projective_transcript.G1_vec),
        G2_vec: ArkG2Projective::normalize_batch(&projective_transcript.G2_vec),
        scalar_vec: projective_transcript.scalar_vec.clone(),
    }
}

#[allow(non_snake_case)]
fn affine_das_transcript_from_projective_transcript(
    projective_transcript: &BlstrsTranscriptSimulatorProj,
) -> BlstrsTranscriptSimulatorAffine {
    //    let mut G1_vec = vec![BlstrsG1Affine::generator(); projective_transcript.G1_vec.len()];
    //    let mut G2_vec = vec![BlstrsG2Affine::generator(); projective_transcript.G2_vec.len()];
    let G1_vec = blstrs_normalize_batch_g1(&projective_transcript.G1_vec);
    let G2_vec = blstrs_normalize_batch_g2(&projective_transcript.G2_vec);

    BlstrsTranscriptSimulatorAffine {
        G1_vec,
        G2_vec,
        scalar_vec: projective_transcript.scalar_vec.clone(),
    }
}

fn bench_projective_serialization_arkworks(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let sizes = chunky_sizes();

    c.bench_function(
        "serialize chunky transcript (projective, compressed)",
        |b| {
            b.iter_batched(
                || random_projective_chunky_transcript(&sizes, &mut rng),
                |transcript| {
                    let mut bytes = Vec::new();
                    transcript.serialize_compressed(&mut bytes).unwrap();
                    black_box(bytes);
                },
                BatchSize::SmallInput,
            )
        },
    );
}

fn bench_affine_serialization_arkworks(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let sizes = chunky_sizes();

    c.bench_function("serialize chunky transcript (affine, compressed)", |b| {
        b.iter_batched(
            || random_projective_chunky_transcript(&sizes, &mut rng),
            |projective_transcript| {
                let mut bytes = Vec::new();

                let affine_transcript =
                    affine_chunky_transcript_from_projective_transcript(&projective_transcript);

                affine_transcript.serialize_compressed(&mut bytes).unwrap();
                black_box(bytes);
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_projective_serialization_blstrs(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let sizes = das_sizes();

    c.bench_function("serialize das transcript (projective, compressed)", |b| {
        b.iter_batched(
            || random_projective_das_transcript(&sizes, &mut rng),
            |transcript| {
                let bytes = bcs::to_bytes(&transcript).unwrap();
                black_box(bytes);
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_affine_serialization_blstrs(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let sizes = das_sizes();

    c.bench_function("serialize das transcript (affine, compressed)", |b| {
        b.iter_batched(
            || random_projective_das_transcript(&sizes, &mut rng),
            |projective_transcript| {
                let affine_transcript =
                    affine_das_transcript_from_projective_transcript(&projective_transcript);

                let bytes = bcs::to_bytes(&affine_transcript).unwrap();
                black_box(bytes);
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets =
        bench_arkworks_projective_1k,
        bench_arkworks_affine_1k,
        bench_blstrs_projective_1k,
        bench_blstrs_affine_1k,
        bench_projective_serialization_arkworks,
        bench_affine_serialization_arkworks,
        bench_projective_serialization_blstrs,
        bench_affine_serialization_blstrs
);

criterion_main!(benches);
