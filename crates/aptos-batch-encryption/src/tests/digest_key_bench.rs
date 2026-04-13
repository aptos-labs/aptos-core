// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::shared::{digest::DigestKey, digest_key_file};
#[allow(unused_imports)]
use ark_std::rand::thread_rng;
use std::{path::Path, time::Instant};

const BATCH_SIZES: &[usize] = &[128];
const NUM_ROUNDS: &[usize] = &[216_000];

#[test]
#[ignore]
fn bench_digest_key_generate_serialize_deserialize() {
    let mut rng = thread_rng();

    println!();
    println!(
        "{:<12} {:<12} {:<20} {:<15} {:<20} {:<20} {:<15}",
        "batch_size",
        "num_rounds",
        "generate (s)",
        "file_size (MB)",
        "write_file (s)",
        "deserialize (s)",
        "read_file (s)"
    );
    println!("{}", "-".repeat(114));

    for &batch_size in BATCH_SIZES {
        for &num_rounds in NUM_ROUNDS {
            // 1. Generate DigestKey
            let start = Instant::now();
            let dk = DigestKey::new(&mut rng, batch_size, num_rounds)
                .expect("DigestKey::new should succeed");
            let generate_elapsed = start.elapsed();

            // 2. Write to file
            let file_path = format!("/tmp/digest_key_b{}_r{}.bcs", batch_size, num_rounds);
            let start = Instant::now();
            digest_key_file::write_digest_key(Path::new(&file_path), dk)
                .expect("File write should succeed");
            let write_elapsed = start.elapsed();

            let file_size_mb =
                std::fs::metadata(&file_path).expect("metadata").len() as f64 / (1024.0 * 1024.0);

            // 3. Read from file + deserialize
            let start = Instant::now();
            let _dk2 = digest_key_file::read_digest_key(Path::new(&file_path))
                .expect("Read should succeed");
            let read_elapsed = start.elapsed();

            println!(
                "{:<12} {:<12} {:<20.3} {:<15} {:<20.3} {:<20.3} {:<15.3}",
                batch_size,
                num_rounds,
                generate_elapsed.as_secs_f64(),
                format!("{:.2}", file_size_mb),
                write_elapsed.as_secs_f64(),
                0.0, // deserialize is included in read
                read_elapsed.as_secs_f64(),
            );
        }
    }
}

#[test]
#[ignore]
fn bench_digest_key_deserialize_from_file() {
    let file_path = "/tmp/digest_key_b128_r216000.bcs";

    println!();
    println!("Deserializing from: {}", file_path);

    let file_size_mb =
        std::fs::metadata(file_path).expect("metadata").len() as f64 / (1024.0 * 1024.0);
    println!("File size: {:.2} MB", file_size_mb);

    let start = Instant::now();
    let _dk = digest_key_file::read_digest_key(Path::new(file_path))
        .expect("Read + deserialize should succeed");
    let read_elapsed = start.elapsed();

    println!(
        "Read + deserialize time: {:.3} s",
        read_elapsed.as_secs_f64()
    );
}
