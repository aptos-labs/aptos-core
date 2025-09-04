// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Don't forget to run this benchmark with AES-NI enable.
//! You can do this by building with the following flags:
//! `RUSTFLAGS="-Ctarget-cpu=skylake -Ctarget-feature=+aes,+sse2,+sse4.1,+ssse3"`.
//!

#[macro_use]
extern crate criterion;

use velor_crypto::{
    noise::{handshake_init_msg_len, handshake_resp_msg_len, NoiseConfig, AES_GCM_TAGLEN},
    test_utils::TEST_SEED,
    x25519, Uniform as _, ValidCryptoMaterial as _,
};
use criterion::{Criterion, Throughput};
use rand::SeedableRng;
use std::convert::TryFrom as _;

const MSG_SIZE: usize = 4096;

fn benchmarks(c: &mut Criterion) {
    // bench the handshake
    let mut group = c.benchmark_group("noise-handshake");
    group.throughput(Throughput::Elements(1));
    group.bench_function("connect", |b| {
        // setup keys first
        let mut rng = ::rand::rngs::StdRng::from_seed(TEST_SEED);
        let initiator_static = x25519::PrivateKey::generate(&mut rng);
        let initiator_static = initiator_static.to_bytes();
        let responder_static = x25519::PrivateKey::generate(&mut rng);
        let responder_public = responder_static.public_key();
        let responder_static = responder_static.to_bytes();

        let mut first_message = [0u8; handshake_init_msg_len(0)];
        let mut second_message = [0u8; handshake_resp_msg_len(0)];

        b.iter(|| {
            let initiator_static =
                x25519::PrivateKey::try_from(initiator_static.clone().as_slice()).unwrap();
            let responder_static =
                x25519::PrivateKey::try_from(responder_static.clone().as_slice()).unwrap();

            let initiator = NoiseConfig::new(initiator_static);
            let responder = NoiseConfig::new(responder_static);

            let initiator_state = initiator
                .initiate_connection(
                    &mut rng,
                    b"prologue",
                    responder_public,
                    None,
                    &mut first_message,
                )
                .unwrap();

            let _ = responder
                .respond_to_client_and_finalize(
                    &mut rng,
                    b"prologue",
                    &first_message,
                    None,
                    &mut second_message,
                )
                .unwrap();
            let _ = initiator
                .finalize_connection(initiator_state, &second_message)
                .unwrap();
        })
    });
    group.finish();

    let mut transport_group = c.benchmark_group("noise-transport");
    transport_group.throughput(Throughput::Bytes(MSG_SIZE as u64 * 2));
    transport_group.bench_function("AES-GCM throughput", |b| {
        let mut buffer_msg = [0u8; MSG_SIZE + AES_GCM_TAGLEN];

        // setup keys first
        let mut rng = ::rand::rngs::StdRng::from_seed(TEST_SEED);
        let initiator_static = x25519::PrivateKey::generate(&mut rng);
        let responder_static = x25519::PrivateKey::generate(&mut rng);
        let responder_public = responder_static.public_key();

        // handshake first
        let initiator = NoiseConfig::new(initiator_static);
        let responder = NoiseConfig::new(responder_static);

        let mut first_message = [0u8; handshake_init_msg_len(0)];
        let mut second_message = [0u8; handshake_resp_msg_len(0)];

        let initiator_state = initiator
            .initiate_connection(
                &mut rng,
                b"prologue",
                responder_public,
                None,
                &mut first_message,
            )
            .unwrap();
        let (_, mut responder_session) = responder
            .respond_to_client_and_finalize(
                &mut rng,
                b"prologue",
                &first_message,
                None,
                &mut second_message,
            )
            .unwrap();
        let (_, mut initiator_session) = initiator
            .finalize_connection(initiator_state, &second_message)
            .unwrap();

        // bench throughput post-handshake
        b.iter(move || {
            let auth_tag = initiator_session
                .write_message_in_place(&mut buffer_msg[..MSG_SIZE])
                .expect("session should not be closed");

            buffer_msg[MSG_SIZE..MSG_SIZE + AES_GCM_TAGLEN].copy_from_slice(&auth_tag);

            let _plaintext = responder_session
                .read_message_in_place(&mut buffer_msg[..MSG_SIZE + AES_GCM_TAGLEN])
                .expect("session should not be closed");
        })
    });
    transport_group.finish();
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
