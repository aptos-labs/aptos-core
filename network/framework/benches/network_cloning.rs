// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Benchmarks measuring the performance and memory impact of cloning optimizations
//! in the network layer.
//!
//! Run with: cargo bench -p aptos-network --bench network_cloning
//!
//! Each benchmark group corresponds to an optimization area:
//!   1. peers_and_metadata_cache_update  - Arc<PeerMetadata> cache update cost
//!   2. handshake_msg_construction       - HandshakeMsg supported_protocols clone
//!   3. discovered_peer_lookup           - get_discovered_peers_for_ids clone cost
//!   4. addresses_union                  - Addresses::union deduplication
//!   5. connection_notification_broadcast - broadcast clone vs move
//!   6. direct_send_msg_clone            - Vec<u8> vs Bytes payload cloning

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

// ─── Benchmark 1: PeerMetadata cache update (HashMap clone) ───────────────────

/// Simulates the current pattern: cloning the entire HashMap<NetworkId, HashMap<PeerId, PeerMetadata>>
/// to update the ArcSwap cache after every mutation.
fn bench_cache_update_full_clone(c: &mut Criterion) {
    use aptos_config::{config::PeerRole, network_id::NetworkId};
    use aptos_netcore::transport::ConnectionOrigin;
    use aptos_network::{
        application::metadata::PeerMetadata,
        protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolIdSet},
        transport::{ConnectionId, ConnectionMetadata},
    };
    use aptos_types::{network_address::NetworkAddress, PeerId};

    let mut group = c.benchmark_group("peers_and_metadata_cache_update");

    for num_peers in [10, 50, 100, 500] {
        // Build a map with `num_peers` entries
        let mut inner: HashMap<PeerId, PeerMetadata> = HashMap::new();
        for _ in 0..num_peers {
            let peer_id = PeerId::random();
            let conn_meta = ConnectionMetadata::new(
                peer_id,
                ConnectionId::default(),
                NetworkAddress::mock(),
                ConnectionOrigin::Inbound,
                MessagingProtocolVersion::V1,
                ProtocolIdSet::empty(),
                PeerRole::Unknown,
            );
            inner.insert(peer_id, PeerMetadata::new(conn_meta));
        }
        let mut map: HashMap<NetworkId, HashMap<PeerId, PeerMetadata>> = HashMap::new();
        map.insert(NetworkId::Validator, inner);

        group.bench_with_input(
            BenchmarkId::new("full_clone", num_peers),
            &num_peers,
            |b, _| {
                b.iter(|| {
                    let cloned = black_box(map.clone());
                    let _ = Arc::new(cloned);
                })
            },
        );
    }
    group.finish();
}

// ─── Benchmark 2: HandshakeMsg supported_protocols clone ──────────────────────

fn bench_handshake_msg_protocols(c: &mut Criterion) {
    use aptos_network::protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolIdSet};

    let mut group = c.benchmark_group("handshake_msg_construction");

    // Build a representative BTreeMap
    let mut supported = BTreeMap::new();
    supported.insert(MessagingProtocolVersion::V1, ProtocolIdSet::all_known());

    // Benchmark: clone BTreeMap (current)
    group.bench_function("btreemap_clone", |b| {
        b.iter(|| {
            black_box(supported.clone());
        })
    });

    // Benchmark: Arc<BTreeMap> clone (optimized)
    let arc_supported = Arc::new(supported.clone());
    group.bench_function("arc_clone", |b| {
        b.iter(|| {
            black_box(Arc::clone(&arc_supported));
        })
    });

    group.finish();
}

// ─── Benchmark 3: Discovered peer lookup and clone ────────────────────────────

fn bench_discovered_peer_clone(c: &mut Criterion) {
    // We cannot easily import DiscoveredPeer from the private module,
    // so we measure the cost of cloning a Vec<NetworkAddress> which is the
    // dominant cost inside DiscoveredPeer::clone().
    use aptos_types::{network_address::NetworkAddress, PeerId};

    let mut group = c.benchmark_group("discovered_peer_lookup");

    for num_peers in [10, 50, 200] {
        // Build a HashMap simulating the discovered peer set
        let mut peer_addrs: HashMap<PeerId, Vec<NetworkAddress>> = HashMap::new();
        for _ in 0..num_peers {
            let peer_id = PeerId::random();
            // Each peer has 2-3 addresses
            let addrs: Vec<NetworkAddress> = (0..3)
                .map(|i| {
                    format!("/ip4/10.0.0.{}/tcp/{}", i, 6180 + i)
                        .parse()
                        .unwrap()
                })
                .collect();
            peer_addrs.insert(peer_id, addrs);
        }

        let peer_ids: Vec<PeerId> = peer_addrs.keys().copied().collect();
        // Select half the peers
        let selected: HashSet<PeerId> = peer_ids.iter().take(num_peers / 2).copied().collect();

        group.bench_with_input(
            BenchmarkId::new("clone_selected_peers", num_peers),
            &num_peers,
            |b, _| {
                b.iter(|| {
                    let result: Vec<(PeerId, Vec<NetworkAddress>)> = selected
                        .iter()
                        .filter_map(|pid| peer_addrs.get(pid).map(|addrs| (*pid, addrs.clone())))
                        .collect();
                    black_box(result);
                })
            },
        );

        // Lazy lookup: just return IDs, look up data on demand
        group.bench_with_input(
            BenchmarkId::new("lazy_id_only", num_peers),
            &num_peers,
            |b, _| {
                b.iter(|| {
                    let result: Vec<PeerId> = selected.iter().copied().collect();
                    black_box(result);
                })
            },
        );
    }
    group.finish();
}

// ─── Benchmark 4: Addresses::union – HashSet vs in-place dedup ────────────────

fn bench_addresses_union(c: &mut Criterion) {
    use aptos_types::network_address::NetworkAddress;

    let mut group = c.benchmark_group("addresses_union");

    // Build address buckets (simulating 4 discovery sources with some overlap)
    let bucket1: Vec<NetworkAddress> = (0..5)
        .map(|i| format!("/ip4/10.0.0.{}/tcp/6180", i).parse().unwrap())
        .collect();
    let bucket2: Vec<NetworkAddress> = (3..8)
        .map(|i| format!("/ip4/10.0.0.{}/tcp/6180", i).parse().unwrap())
        .collect();
    let bucket3: Vec<NetworkAddress> = vec![];
    let bucket4: Vec<NetworkAddress> = (0..2)
        .map(|i| format!("/ip4/10.0.0.{}/tcp/6180", i).parse().unwrap())
        .collect();
    let buckets = [
        bucket1.clone(),
        bucket2.clone(),
        bucket3.clone(),
        bucket4.clone(),
    ];

    // Current approach: HashSet -> Vec
    group.bench_function("hashset_then_collect", |b| {
        b.iter(|| {
            let set: HashSet<_> = buckets.iter().flatten().cloned().collect();
            let result: Vec<NetworkAddress> = set.into_iter().collect();
            black_box(result);
        })
    });

    // Optimized: collect into Vec, deduplicate via retain
    group.bench_function("vec_retain_dedup", |b| {
        b.iter(|| {
            let mut all: Vec<NetworkAddress> = buckets.iter().flatten().cloned().collect();
            let mut seen = HashSet::with_capacity(all.len());
            all.retain(|addr| seen.insert(addr.clone()));
            black_box(all);
        })
    });

    group.finish();
}

// ─── Benchmark 5: ConnectionNotification broadcast clone cost ─────────────────

fn bench_notification_broadcast(c: &mut Criterion) {
    use aptos_config::{config::PeerRole, network_id::NetworkId};
    use aptos_netcore::transport::ConnectionOrigin;
    use aptos_network::{
        peer_manager::ConnectionNotification,
        protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolIdSet},
        transport::{ConnectionId, ConnectionMetadata},
    };
    use aptos_types::{network_address::NetworkAddress, PeerId};

    let mut group = c.benchmark_group("connection_notification_broadcast");

    let peer_id = PeerId::random();
    let conn_meta = ConnectionMetadata::new(
        peer_id,
        ConnectionId::default(),
        NetworkAddress::mock(),
        ConnectionOrigin::Inbound,
        MessagingProtocolVersion::V1,
        ProtocolIdSet::empty(),
        PeerRole::Unknown,
    );
    let notif = ConnectionNotification::NewPeer(conn_meta, NetworkId::Validator);

    for num_subscribers in [1usize, 5, 10] {
        group.bench_with_input(
            BenchmarkId::new("clone_all", num_subscribers),
            &num_subscribers,
            |b, &n| {
                b.iter(|| {
                    for _ in 0..n {
                        black_box(notif.clone());
                    }
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("clone_n_minus_1_move_last", num_subscribers),
            &num_subscribers,
            |b, &n| {
                b.iter(|| {
                    let notif_local = notif.clone(); // one clone to get owned
                    for _ in 0..n.saturating_sub(1) {
                        black_box(notif_local.clone());
                    }
                    black_box(notif_local); // move the last
                })
            },
        );
    }
    group.finish();
}

// ─── Benchmark 6: DirectSendMsg payload – Vec<u8> vs Bytes ────────────────────

fn bench_payload_clone(c: &mut Criterion) {
    use bytes::Bytes;

    let mut group = c.benchmark_group("message_payload_clone");

    for payload_size in [256, 1024, 4096, 65536] {
        let vec_payload: Vec<u8> = vec![42u8; payload_size];
        let bytes_payload: Bytes = Bytes::from(vec_payload.clone());

        group.bench_with_input(
            BenchmarkId::new("vec_u8_clone", payload_size),
            &payload_size,
            |b, _| {
                b.iter(|| {
                    black_box(vec_payload.clone());
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("bytes_clone", payload_size),
            &payload_size,
            |b, _| {
                b.iter(|| {
                    black_box(bytes_payload.clone());
                })
            },
        );
    }
    group.finish();
}

// ─── Benchmark 7: TcpTransport clone vs Arc ───────────────────────────────────

fn bench_tcp_transport_clone(c: &mut Criterion) {
    use aptos_netcore::transport::tcp::TcpTransport;

    let mut group = c.benchmark_group("tcp_transport_clone");

    let transport = TcpTransport {
        ttl: Some(64),
        nodelay: Some(true),
        tcp_buff_cfg: Default::default(),
    };

    group.bench_function("struct_clone", |b| {
        b.iter(|| {
            black_box(transport.clone());
        })
    });

    let arc_transport = Arc::new(transport.clone());
    group.bench_function("arc_clone", |b| {
        b.iter(|| {
            black_box(Arc::clone(&arc_transport));
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cache_update_full_clone,
    bench_handshake_msg_protocols,
    bench_discovered_peer_clone,
    bench_addresses_union,
    bench_notification_broadcast,
    bench_payload_clone,
    bench_tcp_transport_clone,
);
criterion_main!(benches);
