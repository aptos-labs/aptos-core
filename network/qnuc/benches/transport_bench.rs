// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Benchmarks comparing the QUIC-like UDP transport against the existing TCP+Noise stack.
//!
//! Measures:
//! 1. Handshake latency (connection establishment)
//! 2. Noise encrypt/decrypt per datagram
//! 3. Packet encoding/decoding
//! 4. Reliability layer overhead (ACK processing, retransmission check)
//! 5. Stream messaging throughput
//! 6. End-to-end UDP connection handshake vs TCP+Noise handshake

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use aptos_crypto::{noise::NoiseConfig, x25519, Uniform};
use aptos_qnuc::{
    connection::{Connection, ConnectionConfig},
    crypto::{DatagramCrypto, NoiseHandshake},
    packet::{Packet, PacketHeader, PacketType, SelectiveAck},
    reliability::{RecvTracker, ReliabilityConfig, SendTracker},
    stream::Stream,
};
use bytes::Bytes;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::sync::Arc;
use tokio::net::UdpSocket;

fn make_keypair(seed: [u8; 32]) -> (x25519::PrivateKey, x25519::PublicKey) {
    let mut rng = StdRng::from_seed(seed);
    let priv_key = x25519::PrivateKey::generate(&mut rng);
    let pub_key = priv_key.public_key();
    (priv_key, pub_key)
}

fn reconstruct_key(seed: [u8; 32]) -> x25519::PrivateKey {
    let mut rng = StdRng::from_seed(seed);
    x25519::PrivateKey::generate(&mut rng)
}

fn make_crypto_pair() -> (DatagramCrypto, DatagramCrypto) {
    let (init_priv, _) = make_keypair([1u8; 32]);
    let (resp_priv, resp_pub) = make_keypair([2u8; 32]);

    let initiator = NoiseHandshake::new(init_priv);
    let responder = NoiseHandshake::new(resp_priv);

    let prologue = b"bench-prologue";
    let (init_state, init_msg) = initiator
        .build_initiator_message(prologue, resp_pub, b"ts")
        .unwrap();
    let (_, resp_session, _, resp_msg) = responder
        .handle_initiator_message(prologue, &init_msg, None)
        .unwrap();
    let (_, init_session) = initiator
        .finalize_initiator(init_state, &resp_msg)
        .unwrap();

    (
        DatagramCrypto::new(init_session),
        DatagramCrypto::new(resp_session),
    )
}

fn bench_packet_encode_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("packet_codec");

    for size in [64, 256, 1024].iter() {
        let payload = vec![0xABu8; *size];
        let header = PacketHeader::new(PacketType::Data, 1, 1, 42, *size as u16);
        let pkt = Packet::new(header, Bytes::from(payload));

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::new("encode", size), size, |b, _| {
            b.iter(|| {
                let _ = pkt.encode();
            });
        });

        let encoded = pkt.encode();
        group.bench_with_input(BenchmarkId::new("decode", size), size, |b, _| {
            b.iter(|| {
                let _ = Packet::decode(&encoded).unwrap();
            });
        });
    }

    group.finish();
}

fn bench_noise_encrypt_decrypt(c: &mut Criterion) {
    let mut group = c.benchmark_group("noise_datagram_crypto");

    for size in [64, 256, 1024, 4096].iter() {
        let plaintext = vec![0xCDu8; *size];

        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::new("encrypt", size), size, |b, _| {
            let (mut enc, _) = make_crypto_pair();
            b.iter(|| {
                let _ = enc.encrypt(&plaintext).unwrap();
            });
        });

        group.bench_with_input(BenchmarkId::new("decrypt", size), size, |b, _| {
            let (mut enc, mut dec) = make_crypto_pair();
            let ciphertext = enc.encrypt(&plaintext).unwrap();
            b.iter(|| {
                let _ = dec.decrypt(&ciphertext);
            });
        });
    }

    group.finish();
}

fn bench_noise_handshake(c: &mut Criterion) {
    c.bench_function("noise_ik_handshake", |b| {
        b.iter(|| {
            let init_priv = reconstruct_key([10u8; 32]);
            let resp_priv = reconstruct_key([20u8; 32]);
            let resp_pub = resp_priv.public_key();

            let initiator = NoiseHandshake::new(init_priv);
            let responder = NoiseHandshake::new(resp_priv);

            let prologue = b"bench-handshake";
            let (init_state, init_msg) = initiator
                .build_initiator_message(prologue, resp_pub, b"ts")
                .unwrap();
            let (_, _resp_session, _, resp_msg) = responder
                .handle_initiator_message(prologue, &init_msg, None)
                .unwrap();
            let _ = initiator
                .finalize_initiator(init_state, &resp_msg)
                .unwrap();
        });
    });
}

fn bench_reliability_send_ack(c: &mut Criterion) {
    let mut group = c.benchmark_group("reliability");

    group.bench_function("register_sent_100", |b| {
        b.iter(|| {
            let mut tracker = SendTracker::new(ReliabilityConfig::default());
            for _i in 0u64..100 {
                tracker.register_sent(vec![0u8; 100]);
            }
        });
    });

    group.bench_function("process_ack_100", |b| {
        b.iter_with_setup(
            || {
                let mut tracker = SendTracker::new(ReliabilityConfig::default());
                for _ in 0u64..100 {
                    tracker.register_sent(vec![0u8; 100]);
                }
                tracker
            },
            |mut tracker| {
                let sack = SelectiveAck::new(99, vec![]);
                tracker.process_ack(&sack);
            },
        );
    });

    group.bench_function("recv_reorder_100", |b| {
        b.iter(|| {
            let mut tracker = RecvTracker::new();
            for i in (0u64..100).rev() {
                tracker.receive(i, vec![0u8; 100]);
            }
        });
    });

    group.finish();
}

fn bench_stream_send_receive(c: &mut Criterion) {
    let mut group = c.benchmark_group("stream_messaging");

    for size in [128, 1024, 8192, 65536].iter() {
        let message = vec![0xEFu8; *size];

        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(
            BenchmarkId::new("prepare_send", size),
            size,
            |b, _| {
                b.iter_with_setup(
                    || Stream::new(1, 1, ReliabilityConfig::default()),
                    |mut stream| {
                        let _ = stream.prepare_send(&message).unwrap();
                    },
                );
            },
        );
    }

    group.finish();
}

fn bench_udp_handshake_e2e(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("udp_connection_handshake_e2e", |b| {
        b.iter(|| {
            rt.block_on(async {
                let client_priv = reconstruct_key([30u8; 32]);
                let server_priv = reconstruct_key([40u8; 32]);
                let server_pub = server_priv.public_key();

                let server_sock = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
                let server_addr = server_sock.local_addr().unwrap();
                let client_sock = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());

                let server_sock_c = server_sock.clone();

                let server_handle = tokio::spawn(async move {
                    let mut buf = vec![0u8; 65535];
                    let (n, from) = server_sock_c.recv_from(&mut buf).await.unwrap();
                    let pkt = Packet::decode(&buf[..n]).unwrap();
                    let mut conn = Connection::new(
                        pkt.header.connection_id,
                        from,
                        server_sock_c,
                        ConnectionConfig::default(),
                    );
                    conn.accept_inbound(server_priv, &pkt).await.unwrap()
                });

                let mut client_conn = Connection::new(
                    1,
                    server_addr,
                    client_sock,
                    ConnectionConfig::default(),
                );
                client_conn
                    .connect_outbound(client_priv, server_pub)
                    .await
                    .unwrap();

                server_handle.await.unwrap();
            });
        });
    });
}

fn bench_tcp_noise_handshake_comparison(c: &mut Criterion) {
    use aptos_memsocket::MemorySocket;
    use futures::executor::block_on;
    use futures::future::join;

    c.bench_function("tcp_noise_handshake_memsocket", |b| {
        let (_, resp_pub) = make_keypair([60u8; 32]);

        b.iter(|| {
            let initiator = NoiseConfig::new(reconstruct_key([50u8; 32]));
            let responder = NoiseConfig::new(reconstruct_key([60u8; 32]));

            let (dialer, listener) = MemorySocket::new_pair();

            let prologue = b"bench-tcp-comparison";
            let payload = 0u64.to_le_bytes();

            block_on(join(
                async {
                    let msg_len =
                        aptos_crypto::noise::handshake_init_msg_len(payload.len());
                    let mut buffer = vec![0u8; msg_len];
                    let mut rng = rand::rngs::OsRng;
                    let state = initiator
                        .initiate_connection(
                            &mut rng,
                            prologue,
                            resp_pub,
                            Some(&payload),
                            &mut buffer,
                        )
                        .unwrap();

                    use futures::io::AsyncWriteExt;
                    use futures::io::AsyncReadExt;

                    let mut dialer = dialer;
                    dialer.write_all(&buffer).await.unwrap();
                    dialer.flush().await.unwrap();

                    let resp_len = aptos_crypto::noise::handshake_resp_msg_len(0);
                    let mut resp_buf = vec![0u8; resp_len];
                    dialer.read_exact(&mut resp_buf).await.unwrap();

                    initiator.finalize_connection(state, &resp_buf).unwrap();
                },
                async {
                    use futures::io::AsyncWriteExt;
                    use futures::io::AsyncReadExt;

                    let msg_len =
                        aptos_crypto::noise::handshake_init_msg_len(payload.len());
                    let mut init_buf = vec![0u8; msg_len];
                    let mut listener = listener;
                    listener.read_exact(&mut init_buf).await.unwrap();

                    let resp_len = aptos_crypto::noise::handshake_resp_msg_len(0);
                    let mut resp_buf = vec![0u8; resp_len];
                    let mut rng = rand::rngs::OsRng;

                    let (_rs, hs, _payload) = responder
                        .parse_client_init_message(prologue, &init_buf)
                        .unwrap();
                    responder
                        .respond_to_client(&mut rng, hs, None, &mut resp_buf)
                        .unwrap();

                    listener.write_all(&resp_buf).await.unwrap();
                    listener.flush().await.unwrap();
                },
            ));
        });
    });
}

criterion_group!(
    benches,
    bench_packet_encode_decode,
    bench_noise_encrypt_decrypt,
    bench_noise_handshake,
    bench_reliability_send_ack,
    bench_stream_send_receive,
    bench_udp_handshake_e2e,
    bench_tcp_noise_handshake_comparison,
);

criterion_main!(benches);
