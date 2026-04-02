// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::shared::digest::DigestKey;
#[allow(unused_imports)]
use ark_std::rand::thread_rng;
use std::time::Instant;

const BATCH_SIZES: &[usize] = &[64, 96, 128];
const NUM_ROUNDS: &[usize] = &[200_000, 300_000, 400_000];

#[test]
#[ignore]
fn bench_digest_key_generate_serialize_deserialize() {
    let mut rng = thread_rng();

    println!();
    println!(
        "{:<12} {:<12} {:<20} {:<20} {:<15} {:<20} {:<20} {:<15}",
        "batch_size",
        "num_rounds",
        "generate (s)",
        "serialize (s)",
        "file_size (MB)",
        "write_file (s)",
        "deserialize (s)",
        "read_file (s)"
    );
    println!("{}", "-".repeat(134));

    for &batch_size in BATCH_SIZES {
        for &num_rounds in NUM_ROUNDS {
            // 1. Generate DigestKey
            let start = Instant::now();
            let dk = DigestKey::new(&mut rng, batch_size, num_rounds)
                .expect("DigestKey::new should succeed");
            let generate_elapsed = start.elapsed();

            // 2. BCS serialize + write to file
            let start = Instant::now();
            let bytes = bcs::to_bytes(&dk).expect("BCS serialization should succeed");
            let serialize_elapsed = start.elapsed();

            let file_path = format!("/tmp/digest_key_b{}_r{}.bcs", batch_size, num_rounds);
            let start = Instant::now();
            std::fs::write(&file_path, &bytes).expect("File write should succeed");
            let write_elapsed = start.elapsed();

            let file_size_mb = bytes.len() as f64 / (1024.0 * 1024.0);

            // 3. Read from file + BCS deserialize
            let start = Instant::now();
            let read_bytes = std::fs::read(&file_path).expect("File read should succeed");
            let read_elapsed = start.elapsed();

            let start = Instant::now();
            let _dk2: DigestKey =
                bcs::from_bytes(&read_bytes).expect("BCS deserialization should succeed");
            let deserialize_elapsed = start.elapsed();

            // Keep files around for inspection
            // let _ = std::fs::remove_file(&file_path);

            println!(
                "{:<12} {:<12} {:<20.3} {:<20.3} {:<15} {:<20.3} {:<20.3} {:<15.3}",
                batch_size,
                num_rounds,
                generate_elapsed.as_secs_f64(),
                serialize_elapsed.as_secs_f64(),
                format!("{:.2}", file_size_mb),
                write_elapsed.as_secs_f64(),
                deserialize_elapsed.as_secs_f64(),
                read_elapsed.as_secs_f64(),
            );
        }
    }
}

#[test]
#[ignore]
fn bench_digest_key_deserialize_from_file() {
    let file_path = "/tmp/digest_key_b64_r200000.bcs";

    println!();
    println!("Deserializing from: {}", file_path);

    let start = Instant::now();
    let read_bytes = std::fs::read(file_path).expect("File read should succeed");
    let read_elapsed = start.elapsed();

    let file_size_mb = read_bytes.len() as f64 / (1024.0 * 1024.0);
    println!("File size: {:.2} MB", file_size_mb);
    println!("Read time: {:.3} s", read_elapsed.as_secs_f64());

    let start = Instant::now();
    let _dk: DigestKey = bcs::from_bytes(&read_bytes).expect("BCS deserialization should succeed");
    let deserialize_elapsed = start.elapsed();

    println!(
        "Deserialize time: {:.3} s",
        deserialize_elapsed.as_secs_f64()
    );
}
