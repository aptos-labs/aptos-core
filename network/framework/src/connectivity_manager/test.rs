// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::transport::{
    util::{new_mock_transport, MockTransportEvent},
    ConnectionMetadata,
};
use aptos_config::{
    config::{Peer, PeerRole, PeerSet, HANDSHAKE_VERSION},
    network_id::NetworkId,
};
use aptos_crypto::{test_utils::TEST_SEED, x25519, Uniform};
use aptos_logger::info;
use aptos_time_service::{MockTimeService, TimeService};
use aptos_types::{account_address::AccountAddress, network_address::NetworkAddress};
use futures::future;
use maplit::{hashmap, hashset};
use rand::{rngs::StdRng, SeedableRng};
use std::{io, str::FromStr, sync::Once};
use tokio::runtime::Runtime;
use tokio_retry::strategy::FixedInterval;

const MAX_TEST_CONNECTIONS: usize = 3;
const CONNECTIVITY_CHECK_INTERVAL: Duration = Duration::from_secs(5);
const CONNECTION_DELAY: Duration = Duration::from_millis(100);
// const MAX_CONNECTION_DELAY: Duration = Duration::from_secs(60);
const DEFAULT_BASE_ADDR: &str = "/ip4/127.0.0.1/tcp/9090";

// TODO: the test code could use a lot of love.

// TODO(philiphayes): just use `CONNECTION_DELAY + MAX_CONNNECTION_DELAY_JITTER`
// when the const adds are stabilized, instead of this weird thing...
const MAX_DELAY_WITH_JITTER: Duration = Duration::from_millis(
    CONNECTION_DELAY.as_millis() as u64 + MAX_CONNECTION_DELAY_JITTER.as_millis() as u64,
);

static SETUP_ONCE: Once = Once::new();

fn setup() {
    SETUP_ONCE.call_once(|| {
        println!("connectivity_manager::test::setup() called");
        console_subscriber::init();
        aptos_logger::Logger::init_for_testing();
    });
}

fn network_address(addr_str: &'static str) -> NetworkAddress {
    NetworkAddress::from_str(addr_str).unwrap()
}
fn network_address_with_pubkey(
    addr_str: &'static str,
    pubkey: x25519::PublicKey,
) -> NetworkAddress {
    network_address(addr_str).append_prod_protos(pubkey, HANDSHAKE_VERSION)
}

fn test_peer(index: AccountAddress) -> (PeerId, Peer, x25519::PublicKey, NetworkAddress) {
    test_peer_with_address(index, DEFAULT_BASE_ADDR)
}

fn test_peer_with_address(
    peer_id: AccountAddress,
    addr_str: &'static str,
) -> (PeerId, Peer, x25519::PublicKey, NetworkAddress) {
    let pubkey = x25519::PrivateKey::generate_for_testing().public_key();
    let pubkeys = hashset! { pubkey };
    let addr = network_address_with_pubkey(addr_str, pubkey);
    (
        peer_id,
        Peer::new(vec![addr.clone()], pubkeys, PeerRole::Validator),
        pubkey,
        addr,
    )
}

fn update_peer_with_address(mut peer: Peer, addr_str: &'static str) -> (Peer, NetworkAddress) {
    let keys: Vec<_> = peer.keys.iter().collect();
    let key = *keys.first().unwrap();
    let addr = network_address_with_pubkey(addr_str, *key);
    peer.addresses = vec![addr.clone()];
    (peer, addr)
}

struct TestHarness {
    network_context: NetworkContext,
    peers_and_metadata: Arc<PeersAndMetadata>,
    mock_time: MockTimeService,
    peer_senders: Arc<OutboundPeerConnections>,

    mock_transport_events: tokio::sync::mpsc::Receiver<MockTransportEvent>,
    conn_mgr_reqs_tx: tokio::sync::mpsc::Sender<ConnectivityRequest>,
}

impl TestHarness {
    fn new(seeds: PeerSet) -> (Self, ConnectivityManager<FixedInterval>) {
        let network_context = NetworkContext::mock();
        let time_service = TimeService::mock();
        // let (connection_reqs_tx, connection_reqs_rx) =
        //     aptos_channel::new(QueueStyle::FIFO, 1, None);
        // let (connection_notifs_tx, connection_notifs_rx) = conn_notifs_channel::new();
        // let (conn_mgr_reqs_tx, conn_mgr_reqs_rx) = aptos_channels::new_test(0);
        let (conn_mgr_reqs_tx, requests_rx) = tokio::sync::mpsc::channel(10);
        let peers_and_metadata = PeersAndMetadata::new(&[network_context.network_id()]);
        let config = NetworkConfig::network_with_id(NetworkId::Validator);
        // let mutual_auth = true;
        // let chain_id = ChainId::new(4);
        // let key = PrivateKey::generate_for_testing();
        // let protos = ProtocolIdSet::all_known();
        let (transport, mock_transport_events) = new_mock_transport(time_service.clone());
        // let ant = AptosNetTransport::<MemoryTransport>::new(
        //     MemoryTransport,
        //     network_context,
        //     time_service.clone(),
        //     key,
        //     peers_and_metadata.clone(),
        //     mutual_auth,
        //     HANDSHAKE_VERSION,
        //     chain_id,
        //     protos,
        //     false,
        // );
        // let transport = AptosNetTransportActual::Memory(ant);
        let peer_senders = Arc::new(OutboundPeerConnections::new());
        let apps = Arc::new(ApplicationCollector::new());

        let conn_mgr = ConnectivityManager::new(
            config,
            network_context,
            time_service.clone(),
            peers_and_metadata.clone(),
            seeds,
            requests_rx,
            FixedInterval::new(CONNECTION_DELAY),
            transport,
            apps,
            peer_senders.clone(),
        );
        let mock = Self {
            network_context,
            peers_and_metadata,
            mock_time: time_service.into_mock(),
            peer_senders,
            mock_transport_events,
            conn_mgr_reqs_tx,
        };
        (mock, conn_mgr)
    }

    async fn trigger_connectivity_check(&self) {
        info!("Advance time to trigger connectivity check");
        self.mock_time
            .advance_async(CONNECTIVITY_CHECK_INTERVAL)
            .await;
    }

    async fn trigger_pending_dials(&self) {
        info!("Advance time to trigger dial");
        self.mock_time.advance_async(MAX_DELAY_WITH_JITTER).await;
    }

    async fn get_connected_size(&mut self) -> usize {
        self.peers_and_metadata.count_connected_peers(None)
    }

    async fn get_dial_queue_size(&mut self) -> usize {
        // info!("Sending ConnectivityRequest::GetDialQueueSize");
        let (queue_size_tx, queue_size_rx) = oneshot::channel();
        self.conn_mgr_reqs_tx
            .send(ConnectivityRequest::GetDialQueueSize(queue_size_tx))
            .await
            .unwrap();
        queue_size_rx.await.unwrap()
    }

    // #[cfg(obsolete)]
    async fn send_new_peer_await_delivery(
        &mut self,
        peer_id: PeerId,
        notif_peer_id: PeerId,
        address: NetworkAddress,
    ) {
        println!(
            "Sending NewPeer notification for peer: {}",
            peer_id.short_str()
        );
        let mut metadata = ConnectionMetadata::mock_with_role_and_origin(
            notif_peer_id,
            PeerRole::Unknown,
            ConnectionOrigin::Outbound,
        );
        metadata.addr = address;
        let peer_network_id = PeerNetworkId::new(self.network_context.network_id(), peer_id);
        _ = self
            .peers_and_metadata
            .insert_connection_metadata(peer_network_id, metadata);
        // TODO: this does not 'await delivery' of subscribers and their new-peer message
        let _ = tokio::time::sleep(Duration::from_millis(1)).await;
    }

    // #[cfg(obsolete)]
    async fn send_lost_peer_await_delivery(&mut self, peer_id: PeerId, _address: NetworkAddress) {
        println!(
            "Sending LostPeer notification for peer: {}",
            peer_id.short_str()
        );
        // let mut metadata = ConnectionMetadata::mock_with_role_and_origin(
        //     peer_id,
        //     PeerRole::Unknown,
        //     ConnectionOrigin::Outbound,
        // );
        // metadata.addr = address;
        // let notif = peer_manager::ConnectionNotification::LostPeer(metadata, NetworkId::Validator);
        // self.send_notification_await_delivery(peer_id, notif).await;
        let peer_network_id = PeerNetworkId::new(self.network_context.network_id(), peer_id);
        let pm = self
            .peers_and_metadata
            .get_metadata_for_peer(peer_network_id)
            .unwrap();
        match self
            .peers_and_metadata
            .remove_peer_metadata(peer_network_id, pm.connection_metadata.connection_id)
        {
            Ok(_) => {},
            Err(err) => {
                panic!("could not remove peer: {:?}", err);
            },
        }
        // TODO: this does not 'await delivery' of subscribers and their disconnect message
        let _ = tokio::time::sleep(Duration::from_millis(1)).await;
    }

    #[cfg(obsolete)]
    async fn send_notification_await_delivery(
        &mut self,
        peer_id: PeerId,
        notif: peer_manager::ConnectionNotification,
    ) {
        let (delivered_tx, delivered_rx) = oneshot::channel();
        self.connection_notifs_tx
            .push_with_feedback(peer_id, notif, Some(delivered_tx))
            .unwrap();
        delivered_rx.await.unwrap();
    }

    async fn expect_disconnect_inner(
        &mut self,
        peer_id: PeerId,
        address: NetworkAddress,
        success: bool,
    ) {
        let peer_network_id = PeerNetworkId::new(self.network_context.network_id(), peer_id);
        let they = match self.peer_senders.get_generational(0) {
            None => {
                info!("no peer_senders");
                return;
            },
            Some((they, _gen)) => they,
        };
        match they.get(&peer_network_id) {
            None => {
                info!(
                    "expect_disconnect_inner: {:?} not present, already gone?",
                    peer_network_id
                );
                // already gone, okay
            },
            Some(stub) => {
                info!(
                    "Waiting to receive disconnect request for {:?}",
                    peer_network_id
                );
                let mut close = stub.close.clone();
                close.wait().await;
                info!("close finished for {:?}", peer_network_id);
            },
        }
        // let success = result.is_ok();
        // match self.connection_reqs_rx.next().await.unwrap() {
        //     ConnectionRequest::DisconnectPeer(p, result_tx) => {
        //         assert_eq!(peer_id, p);
        //         result_tx.send(result).unwrap();
        //     },
        //     request => panic!(
        //         "Unexpected ConnectionRequest, expected DisconnectPeer: {:?}",
        //         request
        //     ),
        // }
        if success {
            info!("send_lost_peer_await_delivery");
            self.send_lost_peer_await_delivery(peer_id, address).await;
        }
    }

    async fn expect_disconnect_success(&mut self, peer_id: PeerId, address: NetworkAddress) {
        self.expect_disconnect_inner(peer_id, address, true).await;
    }

    async fn expect_disconnect_fail(&mut self, peer_id: PeerId, address: NetworkAddress) {
        // let error = PeerManagerError::NotConnected(peer_id);
        self.expect_disconnect_inner(peer_id, address, false).await;
    }

    async fn wait_until_empty_dial_queue(&mut self, timeout: Duration) {
        // Wait for dial queue to be empty. Without this, it's impossible to guarantee that a completed
        // dial is removed from a dial queue. We need this guarantee to see the effects of future
        // triggers for connectivity check.
        println!("Waiting for dial queue to be empty");
        let after = tokio::time::Instant::now() + timeout;
        loop {
            let dqsize = self.get_dial_queue_size().await;
            if dqsize == 0 {
                return;
            }
            if tokio::time::Instant::now() > after {
                panic!("dial queue still {:?}", dqsize);
            }
            let _ = tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }

    // expect a dial, send it a result
    async fn expect_one_dial_inner(
        &mut self,
        result: io::Result<()>,
        timeout: Duration,
    ) -> Option<(PeerId, NetworkAddress)> {
        println!("Waiting to receive dial request");
        let success = result.is_ok();
        let event = match tokio::time::timeout(timeout, self.mock_transport_events.recv()).await {
            Ok(result) => match result {
                None => {
                    println!("dial request returned None");
                    return None;
                },
                Some(event) => event,
            },
            Err(_timeout) => {
                println!("dial request timed out after {:?}", timeout);
                return None;
            },
        };
        let (peer_id, address) = match event {
            MockTransportEvent::Dial(dial) => {
                println!("got dial {:?}", dial);
                _ = dial.result_sender.send(result);
                (dial.remote_peer_network_id.peer_id(), dial.network_address)
            },
        };
        if success {
            self.send_new_peer_await_delivery(peer_id, peer_id, address.clone())
                .await;
        }
        Some((peer_id, address))
    }

    // expect a dial, send it a result, ensure dial was to destination we expect
    async fn expect_one_dial(
        &mut self,
        expected_peer_id: PeerId,
        expected_address: NetworkAddress,
        result: io::Result<()>,
        timeout: Duration,
    ) {
        let (peer_id, address) = match self.expect_one_dial_inner(result, timeout).await {
            None => {
                panic!("expect_one_dial timeout");
            },
            Some((peer_id, address)) => (peer_id, address),
        };

        assert_eq!(peer_id, expected_peer_id);
        assert_eq!(address, expected_address);

        self.wait_until_empty_dial_queue(timeout).await;
    }

    // expect a dial, tell it Ok(()), ensure dial was to destination we expect
    async fn expect_one_dial_success(
        &mut self,
        expected_peer_id: PeerId,
        expected_address: NetworkAddress,
        timeout: Duration,
    ) {
        self.expect_one_dial(expected_peer_id, expected_address, Ok(()), timeout)
            .await;
    }

    // expect a dial, send it a failure, ensure dial was to destination we expect
    async fn expect_one_dial_fail(
        &mut self,
        expected_peer_id: PeerId,
        expected_address: NetworkAddress,
        timeout: Duration,
    ) {
        let error = io::Error::from(io::ErrorKind::ConnectionRefused);
        self.expect_one_dial(expected_peer_id, expected_address, Err(error), timeout)
            .await;
    }

    async fn expect_num_dials(&mut self, num_expected: usize, timeout: Duration) {
        // TODO: ideally this would be one total timeout, not N sub timeouts, but this is probably okay for test code
        for _ in 0..num_expected {
            let _ = self.expect_one_dial_inner(Ok(()), timeout).await;
        }
        self.wait_until_empty_dial_queue(timeout).await;
    }

    async fn send_update_discovered_peers(&mut self, src: DiscoverySource, peers: PeerSet) {
        println!("Sending UpdateDiscoveredPeers");
        self.conn_mgr_reqs_tx
            .send(ConnectivityRequest::UpdateDiscoveredPeers(src, peers))
            .await
            .unwrap();
        // allow the ConnectivityManager thread to run
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[test]
fn connect_to_seeds_on_startup() {
    setup();
    let (seed_peer_id, seed_peer, _, seed_addr) = test_peer(AccountAddress::ONE);
    let seeds: PeerSet = hashmap! {seed_peer_id => seed_peer.clone()};
    let (mut mock, conn_mgr) = TestHarness::new(seeds.clone());

    let test = async move {
        println!("connect_to_seeds_on_startup start");

        // Peer manager receives a request to connect to the other peer.
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        mock.expect_one_dial_success(seed_peer_id, seed_addr.clone(), Duration::from_secs(1))
            .await;

        println!("connect_to_seeds_on_startup 2");
        // Sending an UpdateDiscoveredPeers with the same seed address should not
        // trigger any dials.
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, seeds)
            .await;
        mock.trigger_connectivity_check().await;
        assert_eq!(0, mock.get_dial_queue_size().await);

        println!("connect_to_seeds_on_startup 3");
        // Sending new address of seed peer
        let (new_seed, new_seed_addr) =
            update_peer_with_address(seed_peer, "/ip4/127.0.1.1/tcp/8080");
        let update = hashmap! {seed_peer_id => new_seed};
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, update)
            .await;

        println!("connect_to_seeds_on_startup 4");
        // We expect the peer which changed its address to also disconnect.
        mock.send_lost_peer_await_delivery(seed_peer_id, seed_addr.clone())
            .await;

        println!("connect_to_seeds_on_startup 5");
        // We should try to connect to both the new address and seed address.
        mock.trigger_connectivity_check().await;
        println!("connect_to_seeds_on_startup 5.1");
        // mock.trigger_pending_dials().await;
        mock.trigger_connectivity_check().await;
        println!("connect_to_seeds_on_startup 5.2");
        mock.trigger_pending_dials().await;
        println!("connect_to_seeds_on_startup 5.3");
        // wait for a dial but send that dial an error
        mock.expect_one_dial_fail(seed_peer_id, new_seed_addr, Duration::from_secs(5))
            .await;

        println!("connect_to_seeds_on_startup 6");
        // Waiting to receive dial request to seed peer at seed address
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        mock.expect_one_dial_success(seed_peer_id, seed_addr, Duration::from_secs(1))
            .await;

        mock.peers_and_metadata.close_subscribers();
        println!("connect_to_seeds_on_startup done");
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_time()
        .build()
        .unwrap();
    let _enter_context = runtime.enter();
    let conn_mgr_future = conn_mgr.start(runtime.handle().clone());
    runtime.block_on(future::join(conn_mgr_future, test));
}

#[test]
fn addr_change() {
    setup();
    let (other_peer_id, other_peer, _, other_addr) = test_peer(AccountAddress::ZERO);
    let (mut mock, mut conn_mgr) = TestHarness::new(HashMap::new());
    conn_mgr.config.enable_latency_aware_dialing = false;

    let test = async move {
        // Sending address of other peer
        let update = hashmap! {other_peer_id => other_peer.clone()};
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, update.clone())
            .await;
        info!("addr_change 1");

        // Peer manager receives a request to connect to the other peer.
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("addr_change 2");
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        mock.expect_one_dial_success(other_peer_id, other_addr.clone(), Duration::from_secs(1))
            .await;

        // Send request to connect to other peer at old address. ConnectivityManager should not
        // dial, since we are already connected at the new address.
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, update)
            .await;
        mock.trigger_connectivity_check().await;
        info!("addr_change 3");
        assert_eq!(0, mock.get_dial_queue_size().await);

        // Sending new address of other peer
        let (other_peer_new, other_addr_new) =
            update_peer_with_address(other_peer, "/ip4/127.0.1.1/tcp/8080");
        let update = hashmap! {other_peer_id => other_peer_new};
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, update)
            .await;
        mock.trigger_connectivity_check().await;
        info!("addr_change 4");
        assert_eq!(1, mock.get_connected_size().await);

        // We expect the peer which changed its address to also disconnect. (even if the address doesn't match storage)
        mock.send_lost_peer_await_delivery(other_peer_id, other_addr_new.clone())
            .await;
        info!("addr_change 5");
        assert_eq!(0, mock.get_connected_size().await);

        // We should receive dial request to other peer at new address
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        mock.expect_one_dial_success(other_peer_id, other_addr_new, Duration::from_secs(1))
            .await;

        mock.peers_and_metadata.close_subscribers();
        info!("addr_change done");
    };
    // Runtime::new().unwrap().block_on(future::join(conn_mgr.test_start(), test));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_time()
        .build()
        .unwrap();
    let _enter_context = runtime.enter();
    let conn_mgr_future = conn_mgr.start(runtime.handle().clone());
    runtime.block_on(future::join(conn_mgr_future, test));
}

#[test]
fn lost_connection() {
    setup();
    let (other_peer_id, other_peer, _, other_addr) = test_peer(AccountAddress::ZERO);
    let (mut mock, conn_mgr) = TestHarness::new(HashMap::new());

    let test = async move {
        // Sending address of other peer
        let update = hashmap! {other_peer_id => other_peer};
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, update)
            .await;
        info!("lost_connection 1");

        // Peer manager receives a request to connect to the other peer.
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("lost_connection 2");
        mock.expect_one_dial_success(other_peer_id, other_addr.clone(), Duration::from_secs(1))
            .await;

        // Sending LostPeer event to signal connection loss
        info!("lost_connection 3");
        mock.send_lost_peer_await_delivery(other_peer_id, other_addr.clone())
            .await;

        // Peer manager receives a request to connect to the other peer after loss of
        // connection.
        info!("lost_connection 4");
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("lost_connection 5");
        mock.expect_one_dial_success(other_peer_id, other_addr.clone(), Duration::from_secs(1))
            .await;
        mock.peers_and_metadata.close_subscribers();
        info!("lost_connection done");
    };
    Runtime::new()
        .unwrap()
        .block_on(future::join(conn_mgr.test_start(), test));
}

#[ignore] // TODO: broken until stale-close is fixed
#[test]
fn disconnect() {
    setup();
    let (other_peer_id, other_peer, _, other_addr) = test_peer(AccountAddress::ZERO);
    let (mut mock, conn_mgr) = TestHarness::new(HashMap::new());

    let test = async move {
        // Sending pubkey & address of other peer
        let peers = hashmap! {other_peer_id => other_peer.clone()};
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, peers)
            .await;

        // Waiting to receive dial request
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("disconnect 1");
        mock.expect_one_dial_success(other_peer_id, other_addr.clone(), Duration::from_secs(1))
            .await;

        // Sending request to make other peer ineligible (by dropping the key)
        let mut peer = other_peer;
        peer.keys = HashSet::new();
        peer.addresses = vec![network_address(DEFAULT_BASE_ADDR)];

        let peers = hashmap! {other_peer_id => peer};
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, peers)
            .await;

        // Peer is now ineligible, we should disconnect from them
        mock.trigger_connectivity_check().await;
        info!("disconnect 2");
        mock.trigger_connectivity_check().await;
        mock.trigger_connectivity_check().await;
        mock.expect_disconnect_success(other_peer_id, other_addr)
            .await;

        mock.peers_and_metadata.close_subscribers();
        info!("disconnect done");
    };
    Runtime::new()
        .unwrap()
        .block_on(future::join(conn_mgr.test_start(), test));
}

// Tests that connectivity manager retries dials and disconnects on failure.
#[ignore] // TODO: broken until stale-close is fixed
#[test]
fn retry_on_failure() {
    setup();
    let (other_peer_id, peer, _, other_addr) = test_peer(AccountAddress::ZERO);
    let (mut mock, conn_mgr) = TestHarness::new(HashMap::new());

    let test = async move {
        // Sending pubkey set and addr of other peer
        let peers = hashmap! {other_peer_id => peer.clone()};
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, peers)
            .await;

        // First dial attempt fails
        mock.trigger_connectivity_check().await;
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("retry_on_failure 1");
        mock.expect_one_dial_fail(other_peer_id, other_addr.clone(), Duration::from_secs(1))
            .await;

        // We should retry after the failed attempt; this time, it succeeds.
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("retry_on_failure 2");
        mock.expect_one_dial_success(other_peer_id, other_addr.clone(), Duration::from_secs(1))
            .await;
        info!("retry_on_failure 2.1");

        // Sending request to make other peer ineligible (by removing key)
        let mut peer = peer;
        peer.keys = HashSet::new();
        peer.addresses = vec![network_address(DEFAULT_BASE_ADDR)];
        let peers = hashmap! {other_peer_id => peer};
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, peers)
            .await;

        // Peer manager receives a request to disconnect from the other peer, which fails.
        mock.trigger_connectivity_check().await;
        info!("retry_on_failure 3");
        mock.expect_disconnect_fail(other_peer_id, other_addr.clone())
            .await;

        // Peer manager receives another request to disconnect from the other peer, which now
        // succeeds.
        mock.trigger_connectivity_check().await;
        info!("retry_on_failure 4");
        mock.expect_disconnect_success(other_peer_id, other_addr.clone())
            .await;
        mock.peers_and_metadata.close_subscribers();
        info!("retry_on_failure done");
    };
    Runtime::new()
        .unwrap()
        .block_on(future::join(conn_mgr.test_start(), test));
}

// Tests that if we dial an already connected peer or disconnect from an already disconnected
// peer, connectivity manager does not send any additional dial or disconnect requests.
#[test]
fn no_op_requests() {
    setup();
    let (other_peer_id, peer, _, other_addr) = test_peer(AccountAddress::ZERO);
    let (mut mock, conn_mgr) = TestHarness::new(HashMap::new());

    let test = async move {
        // Sending pubkey set and addr of other peer
        let peers = hashmap! {other_peer_id => peer.clone()};
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, peers)
            .await;

        // Peer manager receives a request to connect to the other peer.
        mock.trigger_connectivity_check().await;
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("no_op_requests 1");
        mock.expect_one_dial_fail(other_peer_id, other_addr.clone(), Duration::from_secs(1))
            .await;

        // Send a delayed NewPeer notification.
        mock.send_new_peer_await_delivery(other_peer_id, other_peer_id, other_addr.clone())
            .await;
        mock.trigger_connectivity_check().await;

        // Send request to make other peer ineligible.
        let mut peer = peer;
        peer.keys = HashSet::new();
        peer.addresses = vec![network_address(DEFAULT_BASE_ADDR)];
        let peers = hashmap! {other_peer_id => peer};
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, peers)
            .await;

        // Peer manager receives a request to disconnect from the other peer, which fails.
        mock.trigger_connectivity_check().await;
        info!("no_op_requests 2");
        mock.expect_disconnect_fail(other_peer_id, other_addr.clone())
            .await;

        // Send delayed LostPeer notification for other peer.
        mock.send_lost_peer_await_delivery(other_peer_id, other_addr)
            .await;

        // Trigger connectivity check again. We don't expect connectivity manager to do
        // anything - if it does, the task should panic. That may not fail the test (right
        // now), but will be easily spotted by someone running the tests locally.
        mock.trigger_connectivity_check().await;
        info!("no_op_requests 3");
        assert_eq!(0, mock.get_connected_size().await);
        assert_eq!(0, mock.get_dial_queue_size().await);
        mock.peers_and_metadata.close_subscribers();
        info!("no_op_requests done");
    };
    Runtime::new()
        .unwrap()
        .block_on(future::join(conn_mgr.test_start(), test));
}

fn generate_account_address(val: usize) -> AccountAddress {
    let mut addr = [0u8; AccountAddress::LENGTH];
    let array = val.to_be_bytes();
    addr[AccountAddress::LENGTH - array.len()..].copy_from_slice(&array);
    AccountAddress::new(addr)
}

#[test]
fn backoff_on_failure() {
    setup();
    let (mut mock, conn_mgr) = TestHarness::new(HashMap::new());

    let test = async move {
        let (peer_id_a, peer_a, _, peer_a_addr) = test_peer(AccountAddress::ONE);
        let (peer_id_b, peer_b, _, peer_b_addr) = test_peer(generate_account_address(2));

        // Sending pubkey set and addr of peers
        let peers = hashmap! {peer_id_a => peer_a, peer_id_b => peer_b};
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, peers)
            .await;

        // Send NewPeer notification for peer_b.
        mock.send_new_peer_await_delivery(peer_id_b, peer_id_b, peer_b_addr)
            .await;

        // We fail 10 attempts. In production, an exponential backoff strategy is used.
        for i in 0..10 {
            // Peer manager receives a request to connect to the seed peer.
            mock.trigger_connectivity_check().await;
            mock.trigger_connectivity_check().await;
            mock.trigger_pending_dials().await;
            info!("backoff_on_failure {}", i);
            mock.expect_one_dial_fail(peer_id_a, peer_a_addr.clone(), Duration::from_secs(1))
                .await;
        }

        // Finally, one dial request succeeds
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("backoff_on_failure -1");
        mock.expect_one_dial_success(peer_id_a, peer_a_addr, Duration::from_secs(1))
            .await;
        mock.peers_and_metadata.close_subscribers();
        info!("backoff_on_failure done");
    };
    Runtime::new()
        .unwrap()
        .block_on(future::join(conn_mgr.test_start(), test));
}

// Test that connectivity manager will still connect to a peer if it advertises
// multiple listen addresses and some of them don't work.
#[test]
fn multiple_addrs_basic() {
    setup();
    let (other_peer_id, mut peer, pubkey, _) = test_peer(AccountAddress::ZERO);
    let (mut mock, conn_mgr) = TestHarness::new(HashMap::new());

    let test = async move {
        // For this test, the peer advertises multiple listen addresses. Assume
        // that the first addr fails to connect while the second addr succeeds.
        let other_addr_1 = network_address_with_pubkey("/ip4/127.0.0.1/tcp/9091", pubkey);
        let other_addr_2 = network_address_with_pubkey("/ip4/127.0.0.1/tcp/9092", pubkey);
        peer.addresses = vec![other_addr_1.clone(), other_addr_2.clone()];

        // Sending address of other peer
        let update = hashmap! {other_peer_id => peer};
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, update)
            .await;

        // Assume that the first listen addr fails to connect.
        mock.trigger_connectivity_check().await;
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("multiple_addrs_basic 1");
        mock.expect_one_dial_fail(other_peer_id, other_addr_1, Duration::from_secs(1))
            .await;

        // Since the last connection attempt failed for other_addr_1, we should
        // attempt the next available listener address. In this case, the call
        // succeeds and we should connect to the peer.
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("multiple_addrs_basic 2");
        mock.expect_one_dial_success(other_peer_id, other_addr_2, Duration::from_secs(1))
            .await;
        mock.peers_and_metadata.close_subscribers();
        info!("multiple_addrs_basic done");
    };
    Runtime::new()
        .unwrap()
        .block_on(future::join(conn_mgr.test_start(), test));
}

// Test that connectivity manager will work with multiple addresses even if we
// retry more times than there are addresses.
#[test]
fn multiple_addrs_wrapping() {
    setup();
    let (other_peer_id, mut peer, pubkey, _) = test_peer(AccountAddress::ZERO);
    let (mut mock, conn_mgr) = TestHarness::new(HashMap::new());

    let test = async move {
        let other_addr_1 = network_address_with_pubkey("/ip4/127.0.0.1/tcp/9091", pubkey);
        let other_addr_2 = network_address_with_pubkey("/ip4/127.0.0.1/tcp/9092", pubkey);
        peer.addresses = vec![other_addr_1.clone(), other_addr_2.clone()];

        // Sending address of other peer
        let update = hashmap! {other_peer_id => peer};
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, update)
            .await;

        // Assume that the first listen addr fails to connect.
        mock.trigger_connectivity_check().await;
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("multiple_addrs_wrapping 1");
        mock.expect_one_dial_fail(other_peer_id, other_addr_1.clone(), Duration::from_secs(1))
            .await;

        // The second attempt also fails.
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("multiple_addrs_wrapping 2");
        mock.expect_one_dial_fail(other_peer_id, other_addr_2, Duration::from_secs(1))
            .await;

        // Our next attempt should wrap around to the first address.
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("multiple_addrs_wrapping 3");
        mock.expect_one_dial_success(other_peer_id, other_addr_1, Duration::from_secs(1))
            .await;
        mock.peers_and_metadata.close_subscribers();
        info!("multiple_addrs_wrapping done");
    };
    Runtime::new()
        .unwrap()
        .block_on(future::join(conn_mgr.test_start(), test));
}

// Test that connectivity manager will still work when dialing a peer with
// multiple listen addrs and then that peer advertises a smaller number of addrs.
#[test]
fn multiple_addrs_shrinking() {
    setup();
    let (other_peer_id, mut peer, pubkey, _) = test_peer(AccountAddress::ZERO);
    let (mut mock, conn_mgr) = TestHarness::new(HashMap::new());

    let test = async move {
        let other_addr_1 = network_address_with_pubkey("/ip4/127.0.0.1/tcp/9091", pubkey);
        let other_addr_2 = network_address_with_pubkey("/ip4/127.0.0.1/tcp/9092", pubkey);
        let other_addr_3 = network_address_with_pubkey("/ip4/127.0.0.1/tcp/9093", pubkey);
        peer.addresses = vec![other_addr_1.clone(), other_addr_2, other_addr_3];

        // Sending address of other peer
        let update = hashmap! {other_peer_id => peer.clone()};
        info!("multiple_addrs_shrinking 1");
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, update)
            .await;

        // Assume that the first listen addr fails to connect.
        mock.trigger_connectivity_check().await;
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("multiple_addrs_shrinking 2");
        mock.expect_one_dial_fail(other_peer_id, other_addr_1, Duration::from_secs(1))
            .await;

        let other_addr_4 = network_address_with_pubkey("/ip4/127.0.0.1/tcp/9094", pubkey);
        let other_addr_5 = network_address_with_pubkey("/ip4/127.0.0.1/tcp/9095", pubkey);
        peer.addresses = vec![other_addr_4.clone(), other_addr_5];

        // The peer issues a new, smaller set of listen addrs.
        let update = hashmap! {other_peer_id => peer};
        info!("multiple_addrs_shrinking 3");
        mock.send_update_discovered_peers(DiscoverySource::OnChainValidatorSet, update)
            .await;
        info!("multiple_addrs_shrinking 3.1");

        // After updating the addresses, we should dial the first new address,
        // other_addr_4 in this case.
        mock.trigger_connectivity_check().await;
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("multiple_addrs_shrinking 4");
        mock.expect_one_dial_success(other_peer_id, other_addr_4, Duration::from_secs(1))
            .await;

        mock.peers_and_metadata.close_subscribers();
        info!("multiple_addrs_shrinking done");
    };
    Runtime::new()
        .unwrap()
        .block_on(future::join(conn_mgr.test_start(), test));
}

#[test]
fn public_connection_limit() {
    setup();
    let mut seeds = HashMap::new();

    for i in 0..=MAX_TEST_CONNECTIONS {
        let (peer_id, peer, _, _) = test_peer(generate_account_address(i));
        seeds.insert(peer_id, peer);
    }
    let num_seeds = seeds.len();

    let (mut mock, conn_mgr) = TestHarness::new(seeds);

    let test = async move {
        // Should receive MAX_TEST_CONNECTIONS for each of the seed peers
        info!("public_connection_limit start");
        mock.trigger_connectivity_check().await;
        mock.trigger_connectivity_check().await;
        mock.trigger_pending_dials().await;
        info!("public_connection_limit 1");
        mock.expect_num_dials(num_seeds, Duration::from_secs(1))
            .await;

        // Should be expected number of connnections
        assert_eq!(num_seeds, mock.get_connected_size().await);
        info!("public_connection_limit 2");

        // Should be no more pending dials, even after a connectivity check.
        mock.trigger_connectivity_check().await;
        assert_eq!(0, mock.get_dial_queue_size().await);
        info!("public_connection_limit 3");

        mock.peers_and_metadata.close_subscribers();
        info!("public_connection_limit done");
    };
    Runtime::new()
        .unwrap()
        .block_on(future::join(conn_mgr.test_start(), test));
}

#[test]
fn basic_update_discovered_peers() {
    setup();
    let mut rng = StdRng::from_seed(TEST_SEED);
    let (mock, mut conn_mgr) = TestHarness::new(HashMap::new());
    let trusted_peers = mock
        .peers_and_metadata
        .get_trusted_peers(&mock.network_context.network_id())
        .unwrap();

    // sample some example data
    let peer_id_a = AccountAddress::ZERO;
    let peer_id_b = AccountAddress::ONE;
    let addr_a = network_address("/ip4/127.0.0.1/tcp/9090");
    let addr_b = network_address("/ip4/127.0.0.1/tcp/9091");
    let pubkey_1 = x25519::PrivateKey::generate(&mut rng).public_key();
    let pubkey_2 = x25519::PrivateKey::generate(&mut rng).public_key();

    let pubkeys_1 = hashset! {pubkey_1};
    let pubkeys_2 = hashset! {pubkey_2};
    let pubkeys_1_2 = hashset! {pubkey_1, pubkey_2};

    let peer_a1 = Peer::new(vec![addr_a.clone()], pubkeys_1.clone(), PeerRole::Validator);
    let peer_a2 = Peer::new(vec![addr_a.clone()], pubkeys_2, PeerRole::Validator);
    let peer_b1 = Peer::new(vec![addr_b], pubkeys_1, PeerRole::Validator);
    let peer_a_1_2 = Peer::new(vec![addr_a], pubkeys_1_2, PeerRole::Validator);

    let peers_empty = PeerSet::new();
    let peers_1 = hashmap! {peer_id_a => peer_a1};
    let peers_2 = hashmap! {peer_id_a => peer_a2, peer_id_b => peer_b1.clone()};
    let peers_1_2 = hashmap! {peer_id_a => peer_a_1_2, peer_id_b => peer_b1};

    // basic one peer one discovery source
    conn_mgr.handle_update_discovered_peers(DiscoverySource::OnChainValidatorSet, peers_1.clone());
    assert_eq!(*trusted_peers.read(), peers_1);

    // same update does nothing
    conn_mgr.handle_update_discovered_peers(DiscoverySource::OnChainValidatorSet, peers_1.clone());
    assert_eq!(*trusted_peers.read(), peers_1);

    // reset
    conn_mgr
        .handle_update_discovered_peers(DiscoverySource::OnChainValidatorSet, peers_empty.clone());
    assert_eq!(*trusted_peers.read(), peers_empty);

    // basic union across multiple sources
    conn_mgr.handle_update_discovered_peers(DiscoverySource::OnChainValidatorSet, peers_1.clone());
    assert_eq!(*trusted_peers.read(), peers_1);
    conn_mgr.handle_update_discovered_peers(DiscoverySource::Config, peers_2);
    assert_eq!(*trusted_peers.read(), peers_1_2);

    // does nothing even if another source has same set
    conn_mgr
        .handle_update_discovered_peers(DiscoverySource::OnChainValidatorSet, peers_1_2.clone());
    assert_eq!(*trusted_peers.read(), peers_1_2);
    conn_mgr.handle_update_discovered_peers(DiscoverySource::Config, peers_1_2.clone());
    assert_eq!(*trusted_peers.read(), peers_1_2);

    // since on-chain and config now contain the same sets, clearing one should do nothing.
    conn_mgr.handle_update_discovered_peers(DiscoverySource::Config, peers_empty.clone());
    assert_eq!(*trusted_peers.read(), peers_1_2);

    // reset
    conn_mgr
        .handle_update_discovered_peers(DiscoverySource::OnChainValidatorSet, peers_empty.clone());
    assert_eq!(*trusted_peers.read(), peers_empty);

    // empty update again does nothing
    conn_mgr.handle_update_discovered_peers(DiscoverySource::Config, peers_empty.clone());
    assert_eq!(*trusted_peers.read(), peers_empty);
}

// deleted tests {test_stale_peers_unknown_inbound,test_stale_peers_vfn_inbound} which tested the behavior of handling a ConnectionNotification::NewPeer message, but no interesting logic lives in that handler anymore
