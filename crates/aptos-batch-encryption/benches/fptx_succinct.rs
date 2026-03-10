// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use aptos_batch_encryption::{
    schemes::fptx_succinct::FPTXSuccinct, shared::key_derivation::BIBEDecryptionKeyShare,
    tests::decrypt_all, traits::BatchThresholdEncryption,
};
use aptos_crypto::arkworks::shamir::ShamirThresholdConfig;
use ark_std::rand::{distributions::Alphanumeric, thread_rng, Rng as _};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

pub fn digest(c: &mut Criterion) {
    let mut group = c.benchmark_group("FPTXSuccinct::digest");

    for batch_size in [32, 128, 512, 2048] {
        let mut rng = thread_rng();
        let tc = ShamirThresholdConfig::new(1, 1);
        let (ek, dk, _, _) =
            FPTXSuccinct::setup_for_testing(rng.r#gen(), batch_size, 1, &tc).unwrap();

        let msg: String = String::from("hi");
        let associated_data: String = String::from("");

        let cts: Vec<<FPTXSuccinct as BatchThresholdEncryption>::Ciphertext> = (0..batch_size)
            .map(|_| FPTXSuccinct::encrypt(&ek, &mut rng, &msg, &associated_data).unwrap())
            .collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &(dk, cts),
            |b, input| {
                b.iter(|| FPTXSuccinct::digest(&input.0, &input.1, 0));
            },
        );
    }
}

pub fn encrypt(c: &mut Criterion) {
    let mut group = c.benchmark_group("FPTXSuccinct::encrypt");

    for batch_size in [32, 128, 512, 2048] {
        let mut rng = thread_rng();
        let tc = ShamirThresholdConfig::new(1, 1);
        let (ek, _dk, _, _) =
            FPTXSuccinct::setup_for_testing(rng.r#gen(), batch_size, 1, &tc).unwrap();

        let msg: String = rng
            .sample_iter(&Alphanumeric)
            .take(1024)
            .map(char::from)
            .collect();

        let associated_data = String::from("");

        group.bench_with_input(BenchmarkId::from_parameter(batch_size), &msg, |b, _| {
            b.iter(|| {
                let mut rng = thread_rng();
                FPTXSuccinct::encrypt(&ek, &mut rng, &msg, &associated_data).unwrap()
            });
        });
    }
}

pub fn verify_ct(c: &mut Criterion) {
    let mut group = c.benchmark_group("FPTXSuccinct::verify_ct");

    for batch_size in [32, 128, 512, 2048] {
        let mut rng = thread_rng();
        let tc = ShamirThresholdConfig::new(1, 1);
        let (ek, _dk, _, _) =
            FPTXSuccinct::setup_for_testing(rng.r#gen(), batch_size, 1, &tc).unwrap();

        let msg: String = String::from("hi");
        let associated_data = String::from("");

        let ct = FPTXSuccinct::encrypt(&ek, &mut rng, &msg, &associated_data).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(batch_size), &ct, |b, ct| {
            b.iter(|| FPTXSuccinct::verify_ct(ct, &associated_data).unwrap());
        });
    }
}

pub fn eval_proofs_compute_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("FPTXSuccinct::eval_proofs_compute_all");
    group.sample_size(10);

    for batch_size in [32, 128, 256, 512, 2048] {
        let mut rng = thread_rng();
        let tc = ShamirThresholdConfig::new(1, 1);
        let (ek, dk, _, _) =
            FPTXSuccinct::setup_for_testing(rng.r#gen(), batch_size, 1, &tc).unwrap();

        let msg: String = String::from("hi");
        let associated_data = String::from("");

        let cts: Vec<<FPTXSuccinct as BatchThresholdEncryption>::Ciphertext> = (0..batch_size)
            .map(|_| FPTXSuccinct::encrypt(&ek, &mut rng, &msg, &associated_data).unwrap())
            .collect();

        let (_, pfs) = FPTXSuccinct::digest(&dk, &cts, 0).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &(pfs, dk),
            |b, input| {
                b.iter(|| FPTXSuccinct::eval_proofs_compute_all(&input.0, &input.1));
            },
        );
    }
}

pub fn eval_proofs_compute_all_2(c: &mut Criterion) {
    let mut group = c.benchmark_group("FPTXSuccinct::eval_proofs_compute_all_2");
    group.sample_size(10);

    for batch_size in [32, 128, 256, 512, 2048] {
        let mut rng = thread_rng();
        let tc = ShamirThresholdConfig::new(1, 1);
        let (ek, dk, _, _) =
            FPTXSuccinct::setup_for_testing(rng.r#gen(), batch_size, 1, &tc).unwrap();

        let msg: String = String::from("hi");
        let associated_data = String::from("");

        let cts: Vec<<FPTXSuccinct as BatchThresholdEncryption>::Ciphertext> = (0..batch_size)
            .map(|_| FPTXSuccinct::encrypt(&ek, &mut rng, &msg, &associated_data).unwrap())
            .collect();

        let (_, pfs) = FPTXSuccinct::digest(&dk, &cts, 0).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &(pfs, dk),
            |b, input| {
                b.iter(|| {
                    FPTXSuccinct::eval_proofs_compute_all_vzgg_multi_point_eval(&input.0, &input.1)
                });
            },
        );
    }
}

pub fn derive_decryption_key_share(c: &mut Criterion) {
    let mut group = c.benchmark_group("FPTXSuccinct::derive_decryption_key_share");
    let batch_size = 128;

    for n in [128, 256, 512, 1024] {
        let t = n * 2 / 3 + 1;
        let mut rng = thread_rng();
        let tc = ShamirThresholdConfig::new(t, n);
        let (ek, dk, _, msk_shares) =
            FPTXSuccinct::setup_for_testing(rng.r#gen(), batch_size, 1, &tc).unwrap();

        let msg: String = String::from("hi");
        let associated_data = String::from("");

        let cts: Vec<<FPTXSuccinct as BatchThresholdEncryption>::Ciphertext> = (0..batch_size)
            .map(|_| FPTXSuccinct::encrypt(&ek, &mut rng, &msg, &associated_data).unwrap())
            .collect();

        let (d, _) = FPTXSuccinct::digest(&dk, &cts, 0).unwrap();

        let msk_share = &msk_shares[0];

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("n={}, t={}", n, t)),
            &(msk_share, d),
            |b, input| {
                b.iter(|| FPTXSuccinct::derive_decryption_key_share(input.0, &input.1));
            },
        );
    }
}

pub fn verify_decryption_key_share(c: &mut Criterion) {
    let mut group = c.benchmark_group("FPTXSuccinct::verify_decryption_key_share");

    for batch_size in [32, 128, 512, 2048] {
        let mut rng = thread_rng();
        let tc = ShamirThresholdConfig::new(1, 1);
        let (ek, dk, vks, msk_shares) =
            FPTXSuccinct::setup_for_testing(rng.r#gen(), batch_size, 1, &tc).unwrap();

        let msg: String = String::from("hi");
        let associated_data = String::from("");

        let cts: Vec<<FPTXSuccinct as BatchThresholdEncryption>::Ciphertext> = (0..batch_size)
            .map(|_| FPTXSuccinct::encrypt(&ek, &mut rng, &msg, &associated_data).unwrap())
            .collect();

        let (d, _) = FPTXSuccinct::digest(&dk, &cts, 0).unwrap();

        let dk_share = FPTXSuccinct::derive_decryption_key_share(&msk_shares[0], &d).unwrap();
        let vk = &vks[0];

        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &(vk, d, dk_share),
            |b, input| {
                b.iter(|| FPTXSuccinct::verify_decryption_key_share(input.0, &input.1, &input.2));
            },
        );
    }
}

pub fn reconstruct_decryption_key(c: &mut Criterion) {
    let mut group = c.benchmark_group("FPTXSuccinct::reconstruct_decryption_key");
    let batch_size = 128;

    for n in [10, 128, 256, 512, 1024] {
        let t = n * 2 / 3 + 1;
        let mut rng = thread_rng();
        let tc = ShamirThresholdConfig::new(t, n);
        let (ek, dk, _, msk_shares) =
            FPTXSuccinct::setup_for_testing(rng.r#gen(), batch_size, 1, &tc).unwrap();

        let msg: String = String::from("hi");
        let associated_data = String::from("");

        let cts: Vec<<FPTXSuccinct as BatchThresholdEncryption>::Ciphertext> = (0..batch_size)
            .map(|_| FPTXSuccinct::encrypt(&ek, &mut rng, &msg, &associated_data).unwrap())
            .collect();

        let (d, _) = FPTXSuccinct::digest(&dk, &cts, 0).unwrap();

        let dk_shares: Vec<BIBEDecryptionKeyShare> = msk_shares
            .iter()
            .map(|msk_share| FPTXSuccinct::derive_decryption_key_share(msk_share, &d).unwrap())
            .take(t)
            .collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("n={}, t={}", n, t)),
            &(dk_shares, tc),
            |b, input| {
                b.iter(|| FPTXSuccinct::reconstruct_decryption_key(&input.0, &input.1).unwrap());
            },
        );
    }
}

pub fn decrypt(c: &mut Criterion) {
    let mut group = c.benchmark_group("FPTXSuccinct::decrypt (full batch, all cts)");

    for batch_size in [32, 128, 512, 2048] {
        let mut rng = thread_rng();
        let tc = ShamirThresholdConfig::new(1, 1);
        let (ek, dk, _, msk_shares) =
            FPTXSuccinct::setup_for_testing(rng.r#gen(), batch_size, 1, &tc).unwrap();

        let msg: String = String::from("hi");
        let associated_data = String::from("");

        let cts: Vec<<FPTXSuccinct as BatchThresholdEncryption>::Ciphertext> = (0..batch_size)
            .map(|_| FPTXSuccinct::encrypt(&ek, &mut rng, &msg, &associated_data).unwrap())
            .collect();

        let (d, pfs_promise) = FPTXSuccinct::digest(&dk, &cts, 0).unwrap();

        let pfs = FPTXSuccinct::eval_proofs_compute_all(&pfs_promise, &dk);

        let dk_shares: Vec<BIBEDecryptionKeyShare> =
            vec![FPTXSuccinct::derive_decryption_key_share(&msk_shares[0], &d).unwrap()];

        let dk = FPTXSuccinct::reconstruct_decryption_key(&dk_shares, &tc).unwrap();

        let prepared_cts: Vec<<FPTXSuccinct as BatchThresholdEncryption>::PreparedCiphertext> =
            cts.iter().map(|ct| ct.prepare(&d, &pfs).unwrap()).collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &(dk, prepared_cts),
            |b, input| {
                b.iter(|| decrypt_all::<FPTXSuccinct, String>(&input.0, &input.1).unwrap());
            },
        );
    }
}

criterion_group!(
    benches,
    digest,
    encrypt,
    verify_ct,
    eval_proofs_compute_all,
    eval_proofs_compute_all_2,
    derive_decryption_key_share,
    verify_decryption_key_share,
    reconstruct_decryption_key,
    decrypt
);
criterion_main!(benches);
