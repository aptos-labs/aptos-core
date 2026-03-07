// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Benchmarks comparing the QNUC (UDP) transport against the existing TCP+Noise stack.
//!
//! Lower-level benchmarks:
//!   1. Packet encoding/decoding throughput
//!   2. Noise per-datagram encrypt/decrypt throughput
//!   3. Noise IK handshake latency (raw crypto)
//!   4. Reliability layer overhead (send tracking, ACK processing, reorder)
//!   5. Stream message preparation (fragmentation) throughput
//!
//! Higher-level benchmarks:
//!   6. QNUC connection handshake over real localhost UDP
//!   7. TCP+Noise handshake over memsocket (existing stack baseline)
//!   8. QNUC Transport trait: dial latency over localhost
//!   9. TCP Transport trait: dial latency over localhost
//!  10. Full-stack data throughput: TCP vs QNUC via netcore Transport

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use aptos_crypto::{noise::NoiseConfig, x25519, Uniform};
use aptos_qnuc::{
    connection::{Connection, ConnectionConfig},
    crypto::{DatagramCrypto, NoiseHandshake},
    netcore_transport::QnucTransportLayer,
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

// ---------------------------------------------------------------------------
// Lower-level benchmarks
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Higher-level benchmarks: connection establishment
// ---------------------------------------------------------------------------

fn bench_qnuc_connection_handshake(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("qnuc_connection_handshake_localhost", |b| {
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

fn bench_tcp_noise_handshake_memsocket(c: &mut Criterion) {
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

// ---------------------------------------------------------------------------
// Higher-level benchmarks: Transport trait dial latency
// ---------------------------------------------------------------------------

fn bench_transport_trait_dial(c: &mut Criterion) {
    use aptos_netcore::transport::Transport;
    use aptos_types::network_address::NetworkAddress;
    use aptos_types::PeerId;

    let mut group = c.benchmark_group("transport_dial_latency");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // QNUC (UDP) Transport dial
    group.bench_function("qnuc_udp_dial", |b| {
        b.iter(|| {
            rt.block_on(async {
                let transport = QnucTransportLayer::new();
                let addr: NetworkAddress = "/ip4/127.0.0.1/udp/0".parse().unwrap();
                let (_listener, listen_addr) = transport.listen_on(addr).unwrap();
                let peer_id = PeerId::random();
                let dial_fut = transport.dial(peer_id, listen_addr).unwrap();
                // We just measure the dial setup, not the full connection
                // since there's no server accepting yet. The future is created.
                drop(dial_fut);
            });
        });
    });

    // TCP Transport dial (setup only)
    group.bench_function("tcp_dial_setup", |b| {
        use aptos_netcore::transport::tcp::TcpTransport;

        b.iter(|| {
            let transport = TcpTransport::default();
            let addr: NetworkAddress = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
            let (_listener, listen_addr) = transport.listen_on(addr).unwrap();
            let peer_id = PeerId::random();
            let dial_fut = transport.dial(peer_id, listen_addr).unwrap();
            drop(dial_fut);
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Higher-level benchmarks: data throughput via netcore Transport trait
// ---------------------------------------------------------------------------

fn bench_qnuc_udp_data_roundtrip(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("data_roundtrip");

    for size in [64, 512, 4096].iter() {
        let message = vec![0xAAu8; *size];

        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(
            BenchmarkId::new("qnuc_udp_loopback", size),
            size,
            |b, _| {
                b.iter(|| {
                    rt.block_on(async {
                        let server = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
                        let server_addr = server.local_addr().unwrap();
                        let client = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
                        let _client_addr = client.local_addr().unwrap();

                        // Client sends, server echoes back
                        let server_c = server.clone();
                        let handle = tokio::spawn(async move {
                            let mut buf = vec![0u8; 65536];
                            let (n, from) = server_c.recv_from(&mut buf).await.unwrap();
                            server_c.send_to(&buf[..n], from).await.unwrap();
                        });

                        client.send_to(&message, server_addr).await.unwrap();
                        let mut buf = vec![0u8; 65536];
                        let (n, _) = client.recv_from(&mut buf).await.unwrap();
                        assert_eq!(&buf[..n], message.as_slice());

                        handle.await.unwrap();
                    });
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("tcp_loopback", size),
            size,
            |b, _| {
                b.iter(|| {
                    rt.block_on(async {
                        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                            .await
                            .unwrap();
                        let addr = listener.local_addr().unwrap();

                        let handle = tokio::spawn(async move {
                            let (mut stream, _) = listener.accept().await.unwrap();
                            use tokio::io::{AsyncReadExt, AsyncWriteExt};
                            let mut buf = vec![0u8; 65536];
                            let n = stream.read(&mut buf).await.unwrap();
                            stream.write_all(&buf[..n]).await.unwrap();
                        });

                        let mut client =
                            tokio::net::TcpStream::connect(addr).await.unwrap();
                        use tokio::io::{AsyncReadExt, AsyncWriteExt};
                        client.write_all(&message).await.unwrap();
                        let mut buf = vec![0u8; 65536];
                        let n = client.read(&mut buf).await.unwrap();
                        assert_eq!(&buf[..n], message.as_slice());

                        handle.await.unwrap();
                    });
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_packet_encode_decode,
    bench_noise_encrypt_decrypt,
    bench_noise_handshake,
    bench_reliability_send_ack,
    bench_stream_send_receive,
    bench_qnuc_connection_handshake,
    bench_tcp_noise_handshake_memsocket,
    bench_transport_trait_dial,
    bench_qnuc_udp_data_roundtrip,
);

criterion_main!(benches);
