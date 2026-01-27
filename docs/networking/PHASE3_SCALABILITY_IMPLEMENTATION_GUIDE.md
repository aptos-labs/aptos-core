# Phase 3: Scalability Implementation Guide

**Timeline**: 6-12 months  
**Focus**: Scale beyond 140 validators to 500-1000+ nodes  
**Priority**: Future-proofing the network for growth

---

## Table of Contents

1. [Scalability Challenges](#1-scalability-challenges)
2. [Gossip Protocol Design](#2-gossip-protocol-design)
3. [Structured Overlay Network](#3-structured-overlay-network)
4. [Adaptive Topology](#4-adaptive-topology)
5. [Implementation Plan](#5-implementation-plan)
6. [Testing at Scale](#6-testing-at-scale)
7. [Migration Strategy](#7-migration-strategy)

---

## 1. Scalability Challenges

### 1.1 Current Full Mesh Limitations

```
Full Mesh Connection Count:
  n = 140 validators
  connections = n × (n-1) / 2 = 9,730 total connections
  per-node = 139 connections each

Scaling Analysis:
  n = 500  → 124,750 connections (499 per node)
  n = 1000 → 499,500 connections (999 per node)
  n = 2000 → 1,999,000 connections (1999 per node)

Resource Usage per Node (estimated):
  - Memory: ~1MB per connection × 999 = ~1GB just for connections
  - File descriptors: 999 (may hit OS limits)
  - CPU for health checks: O(n) pings per interval
  - Bandwidth for broadcasts: O(n) copies per message
```

### 1.2 Bandwidth Analysis

```
Consensus Message Broadcast (current):

  Block proposal size: ~100KB (variable)
  Vote size: ~200 bytes
  Validators: 140
  
  Per proposal round:
    - Proposer sends: 100KB × 139 = 13.9 MB
    - Each vote broadcast: 200B × 139 × 140 = 3.9 MB total
  
  At 1000 validators:
    - Proposer sends: 100KB × 999 = 99.9 MB
    - Each vote broadcast: 200B × 999 × 1000 = 199.8 MB total
    
  Problem: Bandwidth grows O(n²) for broadcasts!
```

### 1.3 Target Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        HYBRID NETWORK TOPOLOGY                               │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                    Consensus Critical Path                              │ │
│  │                      (Direct Connections)                               │ │
│  │                                                                         │ │
│  │    ┌───┐     ┌───┐     ┌───┐     ┌───┐                                 │ │
│  │    │V1 │◄───►│V2 │◄───►│V3 │◄───►│V4 │  ... (partial mesh)            │ │
│  │    └───┘     └───┘     └───┘     └───┘                                 │ │
│  │       │         │         │         │                                   │ │
│  └───────┼─────────┼─────────┼─────────┼───────────────────────────────────┘ │
│          │         │         │         │                                     │
│  ┌───────▼─────────▼─────────▼─────────▼───────────────────────────────────┐ │
│  │                     Gossip Layer (Epidemic)                             │ │
│  │                                                                         │ │
│  │  • Mempool transactions                                                 │ │
│  │  • State sync availability                                              │ │
│  │  • Peer discovery                                                       │ │
│  │                                                                         │ │
│  │    Each node: 20-50 gossip peers (configurable)                        │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                    Structured Overlay (DHT)                             ││
│  │                                                                         ││
│  │  • State sync chunk discovery                                           ││
│  │  • Historical data queries                                              ││
│  │  • O(log n) routing                                                     ││
│  │                                                                         ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Gossip Protocol Design

### 2.1 Gossip Protocol Selection

| Protocol | Pros | Cons | Use Case |
|----------|------|------|----------|
| **Epidemic (Push)** | Simple, fast | Redundant messages | Mempool txns |
| **Epidemic (Pull)** | Efficient, no duplicates | Higher latency | State sync |
| **CRDS (Solana-style)** | Efficient, proven | Complex | Peer discovery |
| **HyParView** | Reliable, adaptive | More connections | Membership |

**Recommendation**: Hybrid approach
- **Push gossip** for time-sensitive, small messages (mempool)
- **Pull gossip** for large data (state sync chunks)
- **CRDS** for peer/metadata propagation

### 2.2 Push Gossip Implementation

**File**: `network/framework/src/gossip/push.rs` (new)

```rust
//! Push-based gossip for fast message propagation.
//!
//! Each node randomly selects `fanout` peers to forward messages to.
//! Messages propagate in O(log n) hops with high probability.

use crate::application::storage::PeersAndMetadata;
use aptos_config::network_id::PeerNetworkId;
use aptos_crypto::HashValue;
use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Configuration for push gossip
#[derive(Clone, Debug)]
pub struct PushGossipConfig {
    /// Number of peers to forward each message to
    pub fanout: usize,
    /// How long to remember seen messages (prevent reprocessing)
    pub seen_cache_ttl: Duration,
    /// Maximum seen cache size
    pub max_seen_cache_size: usize,
    /// Interval to prune seen cache
    pub prune_interval: Duration,
}

impl Default for PushGossipConfig {
    fn default() -> Self {
        Self {
            fanout: 8,  // sqrt(n) is optimal for 64 nodes, adjust for larger
            seen_cache_ttl: Duration::from_secs(60),
            max_seen_cache_size: 100_000,
            prune_interval: Duration::from_secs(10),
        }
    }
}

impl PushGossipConfig {
    /// Config optimized for large networks (500+ nodes)
    pub fn large_network() -> Self {
        Self {
            fanout: 12,  // Higher fanout for faster propagation
            seen_cache_ttl: Duration::from_secs(120),
            max_seen_cache_size: 500_000,
            prune_interval: Duration::from_secs(30),
        }
    }
}

/// Push gossip protocol handler
pub struct PushGossip<M> {
    /// Configuration
    config: PushGossipConfig,
    /// Peers and metadata for peer selection
    peers_and_metadata: Arc<PeersAndMetadata>,
    /// Cache of seen message hashes to prevent reprocessing
    seen_cache: Arc<RwLock<SeenCache>>,
    /// Network sender for forwarding messages
    network_sender: NetworkSender<M>,
    /// Metrics
    metrics: PushGossipMetrics,
}

impl<M: GossipMessage> PushGossip<M> {
    pub fn new(
        config: PushGossipConfig,
        peers_and_metadata: Arc<PeersAndMetadata>,
        network_sender: NetworkSender<M>,
    ) -> Self {
        Self {
            config,
            peers_and_metadata,
            seen_cache: Arc::new(RwLock::new(SeenCache::new())),
            network_sender,
            metrics: PushGossipMetrics::new(),
        }
    }

    /// Process an incoming gossip message
    pub async fn handle_message(
        &self,
        sender: PeerNetworkId,
        message: M,
    ) -> Result<bool, GossipError> {
        let message_hash = message.hash();
        
        // Check if we've seen this message before
        {
            let seen = self.seen_cache.read().await;
            if seen.contains(&message_hash) {
                self.metrics.duplicate_messages.inc();
                return Ok(false);  // Already processed
            }
        }
        
        // Mark as seen
        {
            let mut seen = self.seen_cache.write().await;
            seen.insert(message_hash, Instant::now());
        }
        
        // Forward to random peers (excluding sender)
        self.forward_message(&message, Some(sender)).await?;
        
        self.metrics.processed_messages.inc();
        Ok(true)  // New message, should be processed by application
    }

    /// Broadcast a new message originating from this node
    pub async fn broadcast(&self, message: M) -> Result<(), GossipError> {
        let message_hash = message.hash();
        
        // Mark as seen (we originated it)
        {
            let mut seen = self.seen_cache.write().await;
            seen.insert(message_hash, Instant::now());
        }
        
        // Forward to random peers
        self.forward_message(&message, None).await?;
        
        self.metrics.originated_messages.inc();
        Ok(())
    }

    /// Forward a message to random peers
    async fn forward_message(
        &self,
        message: &M,
        exclude: Option<PeerNetworkId>,
    ) -> Result<(), GossipError> {
        // Get all connected peers
        let mut peers = self.peers_and_metadata
            .get_connected_peers()
            .map_err(|e| GossipError::PeerSelection(e.to_string()))?;
        
        // Exclude the sender if provided
        if let Some(excluded) = exclude {
            peers.retain(|p| *p != excluded);
        }
        
        // Randomly select fanout peers
        let mut rng = rand::thread_rng();
        peers.shuffle(&mut rng);
        let selected: Vec<_> = peers.into_iter().take(self.config.fanout).collect();
        
        // Send to selected peers
        for peer in selected {
            if let Err(e) = self.network_sender.send_to(peer, message.clone()) {
                self.metrics.send_failures.inc();
                // Log but don't fail - gossip is best-effort
                warn!("Failed to forward gossip to {:?}: {:?}", peer, e);
            }
        }
        
        self.metrics.forwarded_messages.inc_by(self.config.fanout as u64);
        Ok(())
    }

    /// Prune old entries from the seen cache
    pub async fn prune_seen_cache(&self) {
        let mut seen = self.seen_cache.write().await;
        let now = Instant::now();
        let ttl = self.config.seen_cache_ttl;
        
        seen.retain(|_, inserted| now.duration_since(*inserted) < ttl);
        
        // If still too large, remove oldest entries
        if seen.len() > self.config.max_seen_cache_size {
            let to_remove = seen.len() - self.config.max_seen_cache_size;
            // Remove oldest entries (simplified - could use LRU)
            let keys: Vec<_> = seen.keys().take(to_remove).cloned().collect();
            for key in keys {
                seen.remove(&key);
            }
        }
    }
}

/// Cache of seen message hashes
struct SeenCache {
    entries: HashMap<HashValue, Instant>,
}

impl SeenCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn contains(&self, hash: &HashValue) -> bool {
        self.entries.contains_key(hash)
    }

    fn insert(&mut self, hash: HashValue, time: Instant) {
        self.entries.insert(hash, time);
    }

    fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&HashValue, &mut Instant) -> bool,
    {
        self.entries.retain(f);
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn keys(&self) -> impl Iterator<Item = &HashValue> {
        self.entries.keys()
    }

    fn remove(&mut self, key: &HashValue) -> Option<Instant> {
        self.entries.remove(key)
    }
}

/// Trait for messages that can be gossiped
pub trait GossipMessage: Clone + Send + Sync + 'static {
    /// Compute a hash for deduplication
    fn hash(&self) -> HashValue;
    
    /// Get the message size for metrics
    fn size(&self) -> usize;
}
```

### 2.3 Pull Gossip for Large Data

**File**: `network/framework/src/gossip/pull.rs` (new)

```rust
//! Pull-based gossip for efficient large data distribution.
//!
//! Nodes periodically request what they're missing from peers.
//! More efficient than push for large data as it avoids duplicates.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Configuration for pull gossip
#[derive(Clone, Debug)]
pub struct PullGossipConfig {
    /// How often to pull from peers
    pub pull_interval: Duration,
    /// Number of peers to pull from each interval
    pub pull_fanout: usize,
    /// Maximum items to request per pull
    pub max_items_per_pull: usize,
    /// Timeout for pull requests
    pub pull_timeout: Duration,
}

impl Default for PullGossipConfig {
    fn default() -> Self {
        Self {
            pull_interval: Duration::from_secs(1),
            pull_fanout: 3,
            max_items_per_pull: 100,
            pull_timeout: Duration::from_secs(5),
        }
    }
}

/// Pull gossip protocol handler
pub struct PullGossip<K, V> {
    /// Configuration
    config: PullGossipConfig,
    /// Local data store
    local_data: Arc<RwLock<HashMap<K, V>>>,
    /// Bloom filter of what we have (for efficient syncing)
    local_bloom: Arc<RwLock<BloomFilter>>,
    /// Pending pull requests
    pending_pulls: Arc<RwLock<HashMap<PeerId, PullRequest<K>>>>,
}

impl<K: DataKey, V: DataValue> PullGossip<K, V> {
    /// Handle a pull request from a peer
    pub async fn handle_pull_request(
        &self,
        peer: PeerNetworkId,
        request: PullRequest<K>,
    ) -> PullResponse<K, V> {
        let local = self.local_data.read().await;
        
        // Find items we have that the peer is missing
        let mut items = Vec::new();
        
        for key in &request.wanted_keys {
            if let Some(value) = local.get(key) {
                if items.len() < self.config.max_items_per_pull {
                    items.push((key.clone(), value.clone()));
                }
            }
        }
        
        PullResponse { items }
    }

    /// Handle a pull response from a peer
    pub async fn handle_pull_response(
        &self,
        peer: PeerNetworkId,
        response: PullResponse<K, V>,
    ) {
        let mut local = self.local_data.write().await;
        let mut bloom = self.local_bloom.write().await;
        
        for (key, value) in response.items {
            if !local.contains_key(&key) {
                local.insert(key.clone(), value);
                bloom.insert(&key);
            }
        }
    }

    /// Periodic pull from random peers
    pub async fn pull_tick(&self) {
        // Select random peers
        let peers = self.select_pull_peers().await;
        
        // For each peer, send a pull request
        for peer in peers {
            let wanted = self.compute_wanted_keys(&peer).await;
            if !wanted.is_empty() {
                let request = PullRequest { wanted_keys: wanted };
                // Send request (async)
                self.send_pull_request(peer, request).await;
            }
        }
    }

    /// Compute what keys we want from a peer
    async fn compute_wanted_keys(&self, peer: &PeerNetworkId) -> Vec<K> {
        // This would typically involve exchanging bloom filters
        // or using set reconciliation (e.g., IBLT)
        vec![]  // Placeholder
    }
}

/// Efficient set reconciliation using Invertible Bloom Lookup Tables
pub struct SetReconciliation {
    iblt: InvertibleBloomLookupTable,
}

impl SetReconciliation {
    /// Compute the symmetric difference between local and remote sets
    pub fn compute_difference(
        &self,
        local_keys: &HashSet<HashValue>,
        remote_iblt: &InvertibleBloomLookupTable,
    ) -> (Vec<HashValue>, Vec<HashValue>) {
        // Keys we have that remote doesn't
        let mut to_send = Vec::new();
        // Keys remote has that we don't
        let mut to_request = Vec::new();
        
        // XOR the IBLTs and decode differences
        let diff = self.iblt.subtract(remote_iblt);
        
        for entry in diff.decode() {
            match entry {
                DiffEntry::OnlyLocal(key) => to_send.push(key),
                DiffEntry::OnlyRemote(key) => to_request.push(key),
            }
        }
        
        (to_send, to_request)
    }
}
```

### 2.4 Gossip Integration with Mempool

**File**: `mempool/src/gossip_mempool.rs` (new)

```rust
//! Gossip-based mempool transaction dissemination.
//!
//! Replaces direct broadcast with epidemic gossip for O(n log n) scalability.

use crate::core_mempool::CoreMempool;
use aptos_network::gossip::{GossipMessage, PushGossip, PushGossipConfig};

/// Mempool transaction wrapper for gossip
#[derive(Clone)]
pub struct GossipTransaction {
    pub transaction: SignedTransaction,
    pub received_at: Instant,
}

impl GossipMessage for GossipTransaction {
    fn hash(&self) -> HashValue {
        self.transaction.committed_hash()
    }

    fn size(&self) -> usize {
        bcs::serialized_size(&self.transaction).unwrap_or(0)
    }
}

/// Gossip-enabled mempool
pub struct GossipMempool {
    /// Core mempool
    core: CoreMempool,
    /// Gossip protocol handler
    gossip: PushGossip<GossipTransaction>,
    /// Configuration
    config: GossipMempoolConfig,
}

impl GossipMempool {
    /// Submit a new transaction (local submission)
    pub async fn submit_transaction(
        &self,
        txn: SignedTransaction,
    ) -> Result<(), MempoolError> {
        // Add to local mempool
        self.core.add_transaction(txn.clone())?;
        
        // Gossip to network
        let gossip_txn = GossipTransaction {
            transaction: txn,
            received_at: Instant::now(),
        };
        self.gossip.broadcast(gossip_txn).await?;
        
        Ok(())
    }

    /// Handle a gossiped transaction from the network
    pub async fn handle_gossip(
        &self,
        sender: PeerNetworkId,
        gossip_txn: GossipTransaction,
    ) -> Result<(), MempoolError> {
        // Check if we should process (deduplication happens in gossip layer)
        let is_new = self.gossip.handle_message(sender, gossip_txn.clone()).await?;
        
        if is_new {
            // Validate and add to local mempool
            self.core.add_transaction(gossip_txn.transaction)?;
        }
        
        Ok(())
    }
}
```

---

## 3. Structured Overlay Network

### 3.1 Kademlia-based DHT

For efficient O(log n) routing to find specific data:

**File**: `network/framework/src/overlay/kademlia.rs` (new)

```rust
//! Kademlia-style distributed hash table for structured routing.
//!
//! Provides O(log n) lookup for any key in the network.

use aptos_crypto::HashValue;
use aptos_types::PeerId;
use std::collections::BTreeMap;

/// Number of bits in peer IDs (256 for SHA-256 based IDs)
const KEY_BITS: usize = 256;

/// K-bucket size (number of peers per bucket)
const K: usize = 20;

/// Alpha: number of parallel lookups
const ALPHA: usize = 3;

/// Kademlia routing table
pub struct KademliaRoutingTable {
    /// Our own peer ID
    local_id: PeerId,
    /// K-buckets indexed by distance (0 = closest, 255 = farthest)
    buckets: [KBucket; KEY_BITS],
}

impl KademliaRoutingTable {
    pub fn new(local_id: PeerId) -> Self {
        Self {
            local_id,
            buckets: std::array::from_fn(|_| KBucket::new()),
        }
    }

    /// Add a peer to the routing table
    pub fn add_peer(&mut self, peer_id: PeerId, addr: NetworkAddress) {
        let distance = self.xor_distance(&peer_id);
        let bucket_idx = self.bucket_index(distance);
        
        self.buckets[bucket_idx].add(peer_id, addr);
    }

    /// Remove a peer from the routing table
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        let distance = self.xor_distance(peer_id);
        let bucket_idx = self.bucket_index(distance);
        
        self.buckets[bucket_idx].remove(peer_id);
    }

    /// Find the K closest peers to a target
    pub fn find_closest(&self, target: &HashValue, count: usize) -> Vec<(PeerId, NetworkAddress)> {
        let mut candidates: Vec<_> = self.buckets
            .iter()
            .flat_map(|b| b.peers())
            .map(|(id, addr)| {
                let dist = xor_distance_hash(&id.to_hash(), target);
                (dist, *id, addr.clone())
            })
            .collect();
        
        candidates.sort_by_key(|(dist, _, _)| *dist);
        candidates.into_iter()
            .take(count)
            .map(|(_, id, addr)| (id, addr))
            .collect()
    }

    /// XOR distance between local ID and another peer
    fn xor_distance(&self, other: &PeerId) -> HashValue {
        xor_distance_hash(&self.local_id.to_hash(), &other.to_hash())
    }

    /// Get the bucket index for a given distance
    fn bucket_index(&self, distance: HashValue) -> usize {
        // Count leading zeros to determine bucket
        let bytes = distance.as_ref();
        let mut idx = 0;
        for byte in bytes {
            if *byte == 0 {
                idx += 8;
            } else {
                idx += byte.leading_zeros() as usize;
                break;
            }
        }
        idx.min(KEY_BITS - 1)
    }
}

/// A single K-bucket
struct KBucket {
    /// Peers in this bucket, ordered by last seen (most recent first)
    peers: BTreeMap<PeerId, KBucketEntry>,
}

struct KBucketEntry {
    addr: NetworkAddress,
    last_seen: Instant,
}

impl KBucket {
    fn new() -> Self {
        Self {
            peers: BTreeMap::new(),
        }
    }

    fn add(&mut self, peer_id: PeerId, addr: NetworkAddress) {
        if self.peers.len() < K {
            self.peers.insert(peer_id, KBucketEntry {
                addr,
                last_seen: Instant::now(),
            });
        } else {
            // Bucket full - ping least recently seen, replace if unresponsive
            // (Simplified - full implementation would ping asynchronously)
        }
    }

    fn remove(&mut self, peer_id: &PeerId) {
        self.peers.remove(peer_id);
    }

    fn peers(&self) -> impl Iterator<Item = (&PeerId, &NetworkAddress)> {
        self.peers.iter().map(|(id, entry)| (id, &entry.addr))
    }
}

/// XOR distance between two hashes
fn xor_distance_hash(a: &HashValue, b: &HashValue) -> HashValue {
    let a_bytes = a.as_ref();
    let b_bytes = b.as_ref();
    let mut result = [0u8; 32];
    for i in 0..32 {
        result[i] = a_bytes[i] ^ b_bytes[i];
    }
    HashValue::new(result)
}
```

### 3.2 DHT-based State Sync Discovery

**File**: `state-sync/src/dht_discovery.rs` (new)

```rust
//! DHT-based discovery for state sync data.
//!
//! Allows nodes to efficiently find peers that have specific data.

use crate::overlay::kademlia::KademliaRoutingTable;

/// Announcement of available data
#[derive(Clone, Debug)]
pub struct DataAnnouncement {
    /// The data key (e.g., state version, chunk hash)
    pub key: HashValue,
    /// The peer providing this data
    pub provider: PeerId,
    /// TTL for this announcement
    pub ttl: Duration,
    /// When the announcement was created
    pub timestamp: u64,
}

/// DHT for state sync data discovery
pub struct StateSyncDHT {
    /// Kademlia routing table
    routing_table: KademliaRoutingTable,
    /// Local storage of announcements
    announcements: HashMap<HashValue, Vec<DataAnnouncement>>,
    /// Network client for DHT operations
    network: NetworkClient,
}

impl StateSyncDHT {
    /// Announce that we have data for a key
    pub async fn announce(&self, key: HashValue) -> Result<(), DHTError> {
        // Store locally
        self.store_announcement(key, self.local_peer_id).await;
        
        // Publish to K closest peers to the key
        let closest = self.routing_table.find_closest(&key, K);
        for (peer_id, _) in closest {
            self.send_store_request(peer_id, key).await?;
        }
        
        Ok(())
    }

    /// Find peers that have data for a key
    pub async fn find_providers(&self, key: HashValue) -> Result<Vec<PeerId>, DHTError> {
        // Check local cache first
        if let Some(announcements) = self.announcements.get(&key) {
            if !announcements.is_empty() {
                return Ok(announcements.iter().map(|a| a.provider).collect());
            }
        }
        
        // Iterative lookup
        let mut queried = HashSet::new();
        let mut to_query: Vec<_> = self.routing_table
            .find_closest(&key, ALPHA)
            .into_iter()
            .collect();
        
        while !to_query.is_empty() {
            let batch: Vec<_> = to_query.drain(..ALPHA.min(to_query.len())).collect();
            
            for (peer_id, _) in batch {
                if queried.insert(peer_id) {
                    match self.send_find_request(peer_id, key).await {
                        Ok(response) => {
                            if !response.providers.is_empty() {
                                return Ok(response.providers);
                            }
                            // Add closer peers to query list
                            for (closer_id, addr) in response.closer_peers {
                                if !queried.contains(&closer_id) {
                                    to_query.push((closer_id, addr));
                                }
                            }
                        }
                        Err(e) => {
                            warn!("DHT query to {:?} failed: {:?}", peer_id, e);
                        }
                    }
                }
            }
            
            // Sort by distance to key
            to_query.sort_by_key(|(id, _)| {
                xor_distance_hash(&id.to_hash(), &key)
            });
        }
        
        Ok(vec![])  // No providers found
    }
}
```

---

## 4. Adaptive Topology

### 4.1 Topology Manager

**File**: `network/framework/src/topology/manager.rs` (new)

```rust
//! Adaptive topology manager that adjusts connections based on network conditions.

use std::collections::HashMap;

/// Topology configuration
#[derive(Clone, Debug)]
pub struct TopologyConfig {
    /// Minimum direct connections to maintain
    pub min_direct_connections: usize,
    /// Maximum direct connections
    pub max_direct_connections: usize,
    /// Target gossip fanout
    pub gossip_fanout: usize,
    /// Connection budget per role
    pub role_budgets: HashMap<PeerRole, usize>,
}

impl Default for TopologyConfig {
    fn default() -> Self {
        let mut role_budgets = HashMap::new();
        role_budgets.insert(PeerRole::Validator, 50);       // Direct consensus peers
        role_budgets.insert(PeerRole::ValidatorFullNode, 20); // VFN connections
        role_budgets.insert(PeerRole::FullNode, 10);        // Public FN connections
        
        Self {
            min_direct_connections: 20,
            max_direct_connections: 100,
            gossip_fanout: 8,
            role_budgets,
        }
    }
}

/// Manages network topology
pub struct TopologyManager {
    config: TopologyConfig,
    /// Current connection state
    connections: Arc<RwLock<ConnectionState>>,
    /// Peer scoring
    peer_scores: Arc<RwLock<PeerScores>>,
    /// Network metrics
    metrics: Arc<TopologyMetrics>,
}

impl TopologyManager {
    /// Decide whether to accept an inbound connection
    pub async fn should_accept_connection(
        &self,
        peer_id: PeerId,
        role: PeerRole,
    ) -> ConnectionDecision {
        let connections = self.connections.read().await;
        let current_count = connections.count_by_role(&role);
        let budget = self.config.role_budgets.get(&role).copied().unwrap_or(0);
        
        if current_count >= budget {
            // At budget - check if this peer is better than existing ones
            let scores = self.peer_scores.read().await;
            let peer_score = scores.get(&peer_id).unwrap_or(&0.5);
            
            if let Some(worst_peer) = connections.worst_peer_by_role(&role, &scores) {
                let worst_score = scores.get(&worst_peer).unwrap_or(&0.5);
                if peer_score > worst_score {
                    return ConnectionDecision::AcceptAndEvict(worst_peer);
                }
            }
            
            ConnectionDecision::Reject(RejectReason::AtBudget)
        } else {
            ConnectionDecision::Accept
        }
    }

    /// Periodically optimize topology
    pub async fn optimize_topology(&self) {
        // 1. Score all peers
        self.update_peer_scores().await;
        
        // 2. Identify underperforming connections
        let to_disconnect = self.find_underperforming_peers().await;
        
        // 3. Identify high-value peers we should connect to
        let to_connect = self.find_valuable_peers().await;
        
        // 4. Execute changes
        for peer in to_disconnect {
            self.disconnect_peer(peer).await;
        }
        for (peer, addr) in to_connect {
            self.connect_to_peer(peer, addr).await;
        }
    }

    /// Update peer scores based on recent performance
    async fn update_peer_scores(&self) {
        let mut scores = self.peer_scores.write().await;
        let connections = self.connections.read().await;
        
        for peer_id in connections.all_peers() {
            let score = self.compute_peer_score(&peer_id).await;
            scores.insert(peer_id, score);
        }
    }

    /// Compute a peer's score (0.0 - 1.0)
    async fn compute_peer_score(&self, peer_id: &PeerId) -> f64 {
        let mut score = 0.5;  // Base score
        
        // Factor 1: Latency (lower is better)
        if let Some(latency) = self.metrics.get_peer_latency(peer_id) {
            score += match latency.as_millis() {
                0..=50 => 0.2,
                51..=100 => 0.1,
                101..=200 => 0.0,
                201..=500 => -0.1,
                _ => -0.2,
            };
        }
        
        // Factor 2: Message delivery rate
        if let Some(delivery_rate) = self.metrics.get_delivery_rate(peer_id) {
            score += (delivery_rate - 0.9) * 2.0;  // Boost for >90%, penalty for <90%
        }
        
        // Factor 3: Uptime
        if let Some(uptime) = self.metrics.get_peer_uptime(peer_id) {
            if uptime > Duration::from_hours(24) {
                score += 0.1;  // Bonus for stable peers
            }
        }
        
        score.clamp(0.0, 1.0)
    }
}

/// Decision for incoming connection
pub enum ConnectionDecision {
    Accept,
    AcceptAndEvict(PeerId),
    Reject(RejectReason),
}

pub enum RejectReason {
    AtBudget,
    Blacklisted,
    TooManyFromSameSubnet,
}
```

### 4.2 Geographic-Aware Peer Selection

**File**: `network/framework/src/topology/geographic.rs` (new)

```rust
//! Geographic-aware peer selection for optimal latency.

use std::collections::HashMap;

/// Geographic regions
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum Region {
    NorthAmerica,
    SouthAmerica,
    Europe,
    Asia,
    Africa,
    Oceania,
}

/// Geographic peer selector
pub struct GeographicSelector {
    /// Our region
    local_region: Region,
    /// Peers by region
    peers_by_region: HashMap<Region, Vec<PeerId>>,
    /// Target distribution
    target_distribution: HashMap<Region, f64>,
}

impl GeographicSelector {
    pub fn new(local_region: Region) -> Self {
        // Default: prioritize local and nearby regions
        let mut target = HashMap::new();
        match local_region {
            Region::NorthAmerica => {
                target.insert(Region::NorthAmerica, 0.4);
                target.insert(Region::Europe, 0.25);
                target.insert(Region::Asia, 0.2);
                target.insert(Region::SouthAmerica, 0.1);
                target.insert(Region::Oceania, 0.05);
            }
            Region::Europe => {
                target.insert(Region::Europe, 0.4);
                target.insert(Region::NorthAmerica, 0.25);
                target.insert(Region::Asia, 0.2);
                target.insert(Region::Africa, 0.1);
                target.insert(Region::Oceania, 0.05);
            }
            // ... other regions
            _ => {
                // Equal distribution as fallback
                for region in [Region::NorthAmerica, Region::Europe, Region::Asia, 
                               Region::SouthAmerica, Region::Africa, Region::Oceania] {
                    target.insert(region, 1.0 / 6.0);
                }
            }
        }
        
        Self {
            local_region,
            peers_by_region: HashMap::new(),
            target_distribution: target,
        }
    }

    /// Select peers maintaining geographic distribution
    pub fn select_peers(&self, count: usize) -> Vec<PeerId> {
        let mut selected = Vec::new();
        
        // Calculate how many from each region
        for (region, target_ratio) in &self.target_distribution {
            let target_count = (count as f64 * target_ratio).ceil() as usize;
            if let Some(region_peers) = self.peers_by_region.get(region) {
                let to_take = target_count.min(region_peers.len());
                selected.extend(region_peers.iter().take(to_take).cloned());
            }
        }
        
        // Truncate to requested count
        selected.truncate(count);
        selected
    }
}
```

---

## 5. Implementation Plan

### Phase 3a: Gossip Layer (Weeks 1-8)

| Week | Milestone |
|------|-----------|
| 1-2 | Push gossip implementation |
| 3-4 | Pull gossip and set reconciliation |
| 5-6 | Mempool gossip integration |
| 7-8 | Testing and benchmarking |

### Phase 3b: Structured Overlay (Weeks 9-16)

| Week | Milestone |
|------|-----------|
| 9-10 | Kademlia routing table |
| 11-12 | DHT operations (store, find) |
| 13-14 | State sync DHT integration |
| 15-16 | Testing and optimization |

### Phase 3c: Adaptive Topology (Weeks 17-24)

| Week | Milestone |
|------|-----------|
| 17-18 | Topology manager core |
| 19-20 | Peer scoring system |
| 21-22 | Geographic-aware selection |
| 23-24 | Integration and testing |

---

## 6. Testing at Scale

### 6.1 Simulation Framework

```rust
/// Network simulator for testing at scale
pub struct NetworkSimulator {
    /// Simulated nodes
    nodes: Vec<SimulatedNode>,
    /// Network latency matrix
    latencies: LatencyMatrix,
    /// Packet loss rates
    loss_rates: HashMap<(NodeId, NodeId), f64>,
}

impl NetworkSimulator {
    /// Create a network with N nodes
    pub fn new(node_count: usize) -> Self {
        // Create nodes with realistic geographic distribution
        let nodes = (0..node_count)
            .map(|i| SimulatedNode::new(i, random_region()))
            .collect();
        
        // Generate latency matrix based on geography
        let latencies = LatencyMatrix::from_geographic(&nodes);
        
        Self {
            nodes,
            latencies,
            loss_rates: HashMap::new(),
        }
    }

    /// Simulate gossip propagation
    pub async fn simulate_gossip(
        &self,
        message: GossipMessage,
        origin: NodeId,
    ) -> GossipStats {
        let mut stats = GossipStats::new();
        let mut received: HashSet<NodeId> = HashSet::new();
        let mut pending: VecDeque<(NodeId, Instant)> = VecDeque::new();
        
        received.insert(origin);
        pending.push_back((origin, Instant::now()));
        
        while let Some((node_id, receive_time)) = pending.pop_front() {
            let node = &self.nodes[node_id];
            let targets = node.gossip_targets();
            
            for target in targets {
                if !received.contains(&target) {
                    // Calculate delivery time
                    let latency = self.latencies.get(node_id, target);
                    let deliver_time = receive_time + latency;
                    
                    // Check for packet loss
                    if !self.is_lost(node_id, target) {
                        received.insert(target);
                        pending.push_back((target, deliver_time));
                        stats.record_delivery(deliver_time - stats.start_time);
                    }
                }
            }
        }
        
        stats.total_nodes = self.nodes.len();
        stats.reached_nodes = received.len();
        stats
    }
}
```

### 6.2 Scale Testing Scenarios

```rust
#[tokio::test]
async fn test_gossip_500_nodes() {
    let sim = NetworkSimulator::new(500);
    let msg = GossipMessage::random();
    let stats = sim.simulate_gossip(msg, 0).await;
    
    assert!(stats.reached_nodes >= 495);  // 99% delivery
    assert!(stats.p99_latency < Duration::from_secs(2));  // Fast propagation
}

#[tokio::test]
async fn test_gossip_1000_nodes() {
    let sim = NetworkSimulator::new(1000);
    let msg = GossipMessage::random();
    let stats = sim.simulate_gossip(msg, 0).await;
    
    assert!(stats.reached_nodes >= 990);  // 99% delivery
    assert!(stats.p99_latency < Duration::from_secs(3));
}

#[tokio::test]
async fn test_dht_lookup_1000_nodes() {
    let sim = NetworkSimulator::new(1000);
    
    // Node 0 stores data
    let key = HashValue::random();
    sim.nodes[0].dht_store(key).await;
    
    // Node 999 looks up data
    let result = sim.nodes[999].dht_find(key).await;
    
    assert!(result.is_some());
    assert!(result.unwrap().hops <= 10);  // O(log n) hops
}
```

### 6.3 Chaos Testing

```rust
/// Chaos testing scenarios
pub struct ChaosTests;

impl ChaosTests {
    /// Test gossip under 5% packet loss
    pub async fn test_gossip_packet_loss() {
        let mut sim = NetworkSimulator::new(500);
        sim.set_global_loss_rate(0.05);
        
        // Gossip should still reach most nodes
        let stats = sim.simulate_gossip(GossipMessage::random(), 0).await;
        assert!(stats.reached_nodes >= 475);  // 95% minimum
    }

    /// Test with node churn (nodes joining/leaving)
    pub async fn test_node_churn() {
        let sim = NetworkSimulator::new(500);
        
        // Simulate 10% churn per minute
        for _ in 0..10 {
            sim.remove_random_nodes(25);  // 5% leave
            sim.add_random_nodes(25);     // 5% join
            
            // System should remain functional
            let stats = sim.simulate_gossip(GossipMessage::random(), 0).await;
            assert!(stats.reached_nodes as f64 / sim.node_count() as f64 >= 0.90);
        }
    }

    /// Test partition tolerance
    pub async fn test_network_partition() {
        let sim = NetworkSimulator::new(500);
        
        // Create two partitions
        sim.partition(0..250, 250..500);
        
        // Gossip within partition should work
        let stats_a = sim.simulate_gossip(GossipMessage::random(), 0).await;
        assert!(stats_a.reached_nodes >= 245);  // Most of partition A
        
        // Heal partition
        sim.heal_partition();
        
        // Full propagation should resume
        let stats_full = sim.simulate_gossip(GossipMessage::random(), 0).await;
        assert!(stats_full.reached_nodes >= 495);
    }
}
```

---

## 7. Migration Strategy

### 7.1 Phased Rollout

```
Phase 3a: Gossip (Optional)
├── Week 1-4: Deploy gossip alongside direct broadcast
├── Week 5-8: Monitor metrics, tune fanout
└── Week 9-12: Gradually shift traffic to gossip

Phase 3b: DHT (Parallel)  
├── Week 1-8: Deploy DHT for state sync discovery only
├── Week 9-12: Expand to other data types
└── Week 13-16: Full DHT integration

Phase 3c: Topology (Gradual)
├── Week 1-8: Deploy topology manager (passive mode)
├── Week 9-16: Enable connection optimization
└── Week 17-24: Full adaptive topology
```

### 7.2 Feature Flags

```toml
[features]
default = []

# Gossip features
gossip = []
gossip-mempool = ["gossip"]
gossip-only-mempool = ["gossip-mempool"]  # Disable direct mempool broadcast

# DHT features
dht = []
dht-state-sync = ["dht"]

# Topology features
adaptive-topology = []
geographic-selection = ["adaptive-topology"]
```

### 7.3 Backwards Compatibility

```rust
/// Compatibility layer for mixed network
pub struct CompatibilityLayer {
    /// Legacy direct broadcast
    legacy_broadcast: DirectBroadcast,
    /// New gossip layer
    gossip: Option<PushGossip>,
    /// Feature detection for peers
    peer_features: HashMap<PeerId, PeerFeatures>,
}

impl CompatibilityLayer {
    /// Broadcast a message using the best available method
    pub async fn broadcast(&self, message: Message) {
        // Use gossip for gossip-capable peers
        if let Some(ref gossip) = self.gossip {
            let gossip_peers: Vec<_> = self.peer_features
                .iter()
                .filter(|(_, f)| f.supports_gossip)
                .map(|(id, _)| *id)
                .collect();
            
            if !gossip_peers.is_empty() {
                gossip.broadcast_to(message.clone(), gossip_peers).await;
            }
        }
        
        // Use direct broadcast for legacy peers
        let legacy_peers: Vec<_> = self.peer_features
            .iter()
            .filter(|(_, f)| !f.supports_gossip)
            .map(|(id, _)| *id)
            .collect();
        
        if !legacy_peers.is_empty() {
            self.legacy_broadcast.send_to_all(message, legacy_peers).await;
        }
    }
}
```

---

## 8. Success Metrics

### Target Performance at Scale

| Metric | 140 Nodes | 500 Nodes | 1000 Nodes |
|--------|-----------|-----------|------------|
| Connections per node | 139 (full mesh) | 50-80 | 50-100 |
| Gossip propagation (p99) | N/A | < 2s | < 3s |
| DHT lookup (p99) | N/A | < 500ms | < 1s |
| Bandwidth per node | O(n) | O(log n) | O(log n) |
| Memory per node | ~150MB | ~100MB | ~150MB |

### Monitoring Dashboard

Key metrics to track:
- Gossip reach rate (% of nodes receiving message)
- Gossip propagation latency (p50, p99)
- DHT lookup success rate
- DHT lookup latency
- Connection count distribution
- Peer score distribution
- Geographic distribution of connections

---

## Appendix: Key Files to Create

| File | Purpose |
|------|---------|
| `gossip/mod.rs` | Gossip module entry |
| `gossip/push.rs` | Push gossip implementation |
| `gossip/pull.rs` | Pull gossip implementation |
| `gossip/message.rs` | Gossip message types |
| `overlay/mod.rs` | Overlay network module |
| `overlay/kademlia.rs` | Kademlia DHT |
| `overlay/dht.rs` | DHT operations |
| `topology/mod.rs` | Topology management |
| `topology/manager.rs` | Topology manager |
| `topology/geographic.rs` | Geographic selection |
| `topology/scoring.rs` | Peer scoring |

---

*Document Version: 1.0*  
*Last Updated: January 27, 2026*
