# Phase 1: Quick Wins Implementation Guide

**Timeline**: 1-2 months  
**Focus**: Latency and throughput improvements with minimal architectural changes  
**Priority**: High-impact, low-to-medium effort changes

---

## Table of Contents

1. [Priority Message Queues](#1-priority-message-queues)
2. [Multiple Concurrent Inbound Streams](#2-multiple-concurrent-inbound-streams)
3. [Enhanced Health Monitoring](#3-enhanced-health-monitoring)
4. [Serialization Optimization](#4-serialization-optimization)

---

## 1. Priority Message Queues

### Problem Statement

Currently, all messages share the same queue regardless of urgency. During high load (e.g., state sync downloading large chunks), time-sensitive consensus messages can be delayed.

**Current Flow**:
```
All Messages → Single KLAST Queue (1024) → Writer Task → TCP
```

**Impact**: Consensus votes delayed by 10-100ms during heavy state sync activity.

### Proposed Solution

Implement a priority-based message queue system with separate channels for different priority classes.

### Implementation Details

#### 1.1 Define Priority Levels

**File**: `network/framework/src/protocols/wire/messaging/v1/mod.rs`

```rust
/// Message priority levels (higher value = higher priority)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessagePriority {
    /// Lowest priority: telemetry, metrics, non-critical data
    Low = 0,
    /// Normal priority: state sync data chunks, general queries
    Normal = 1,
    /// High priority: mempool transactions, block proposals
    High = 2,
    /// Critical priority: consensus votes, timeout certificates
    Critical = 3,
}

impl Default for MessagePriority {
    fn default() -> Self {
        MessagePriority::Normal
    }
}

impl MessagePriority {
    /// Returns the queue capacity for this priority level
    pub fn queue_capacity(&self) -> usize {
        match self {
            MessagePriority::Critical => 256,   // Small, fast queue
            MessagePriority::High => 512,
            MessagePriority::Normal => 1024,
            MessagePriority::Low => 256,
        }
    }
}
```

#### 1.2 Map ProtocolIds to Priorities

**File**: `network/framework/src/protocols/wire/handshake/v1/mod.rs`

Add a method to `ProtocolId` to determine its priority:

```rust
impl ProtocolId {
    /// Returns the message priority for this protocol
    pub fn priority(&self) -> MessagePriority {
        match self {
            // Critical: Consensus protocol messages
            ProtocolId::ConsensusRpcBcs |
            ProtocolId::ConsensusDirectSendBcs |
            ProtocolId::ConsensusDirectSendCompressedBcs => MessagePriority::Critical,
            
            // High: Mempool and block-related
            ProtocolId::MempoolDirectSend |
            ProtocolId::MempoolDirectSendCompressedBcs => MessagePriority::High,
            
            // Normal: State sync and general data transfer
            ProtocolId::StateSyncDirectSend |
            ProtocolId::StorageServiceRpc => MessagePriority::Normal,
            
            // Low: Health checks, discovery, telemetry
            ProtocolId::HealthCheckerRpc => MessagePriority::Low,
            
            // Default to Normal for unknown protocols
            _ => MessagePriority::Normal,
        }
    }
}
```

#### 1.3 Create Priority Queue Structure

**File**: `network/framework/src/peer/priority_queue.rs` (new file)

```rust
//! Priority-based message queue for the Peer actor.
//!
//! This module provides a multi-priority queue that ensures high-priority
//! messages (e.g., consensus votes) are sent before lower-priority messages
//! (e.g., state sync chunks).

use crate::protocols::wire::messaging::v1::{MessagePriority, NetworkMessage};
use aptos_channels::aptos_channel;
use futures::stream::{FusedStream, Stream, StreamExt};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// A priority-aware message queue that processes higher priority messages first.
pub struct PriorityMessageQueue {
    /// Separate queues for each priority level
    critical_rx: aptos_channel::Receiver<(), NetworkMessage>,
    high_rx: aptos_channel::Receiver<(), NetworkMessage>,
    normal_rx: aptos_channel::Receiver<(), NetworkMessage>,
    low_rx: aptos_channel::Receiver<(), NetworkMessage>,
    
    /// Senders for each priority level
    critical_tx: aptos_channel::Sender<(), NetworkMessage>,
    high_tx: aptos_channel::Sender<(), NetworkMessage>,
    normal_tx: aptos_channel::Sender<(), NetworkMessage>,
    low_tx: aptos_channel::Sender<(), NetworkMessage>,
}

impl PriorityMessageQueue {
    pub fn new() -> Self {
        let (critical_tx, critical_rx) = aptos_channel::new(
            aptos_channel::QueueStyle::KLAST,
            MessagePriority::Critical.queue_capacity(),
            None, // Add metrics later
        );
        let (high_tx, high_rx) = aptos_channel::new(
            aptos_channel::QueueStyle::KLAST,
            MessagePriority::High.queue_capacity(),
            None,
        );
        let (normal_tx, normal_rx) = aptos_channel::new(
            aptos_channel::QueueStyle::KLAST,
            MessagePriority::Normal.queue_capacity(),
            None,
        );
        let (low_tx, low_rx) = aptos_channel::new(
            aptos_channel::QueueStyle::KLAST,
            MessagePriority::Low.queue_capacity(),
            None,
        );

        Self {
            critical_rx,
            high_rx,
            normal_rx,
            low_rx,
            critical_tx,
            high_tx,
            normal_tx,
            low_tx,
        }
    }

    /// Push a message to the appropriate priority queue
    pub fn push(&self, priority: MessagePriority, message: NetworkMessage) -> Result<(), NetworkMessage> {
        let result = match priority {
            MessagePriority::Critical => self.critical_tx.push((), message),
            MessagePriority::High => self.high_tx.push((), message),
            MessagePriority::Normal => self.normal_tx.push((), message),
            MessagePriority::Low => self.low_tx.push((), message),
        };
        result.map_err(|(_, msg)| msg)
    }

    /// Get a sender handle for a specific priority
    pub fn sender(&self, priority: MessagePriority) -> aptos_channel::Sender<(), NetworkMessage> {
        match priority {
            MessagePriority::Critical => self.critical_tx.clone(),
            MessagePriority::High => self.high_tx.clone(),
            MessagePriority::Normal => self.normal_tx.clone(),
            MessagePriority::Low => self.low_tx.clone(),
        }
    }
}

impl Stream for PriorityMessageQueue {
    type Item = NetworkMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Check queues in priority order (highest first)
        // This ensures critical messages are always processed first
        
        // 1. Check critical queue
        if let Poll::Ready(Some(msg)) = self.critical_rx.poll_next_unpin(cx) {
            return Poll::Ready(Some(msg));
        }
        
        // 2. Check high queue
        if let Poll::Ready(Some(msg)) = self.high_rx.poll_next_unpin(cx) {
            return Poll::Ready(Some(msg));
        }
        
        // 3. Check normal queue
        if let Poll::Ready(Some(msg)) = self.normal_rx.poll_next_unpin(cx) {
            return Poll::Ready(Some(msg));
        }
        
        // 4. Check low queue
        if let Poll::Ready(Some(msg)) = self.low_rx.poll_next_unpin(cx) {
            return Poll::Ready(Some(msg));
        }
        
        // No messages available, return Pending
        Poll::Pending
    }
}

impl FusedStream for PriorityMessageQueue {
    fn is_terminated(&self) -> bool {
        self.critical_rx.is_terminated()
            && self.high_rx.is_terminated()
            && self.normal_rx.is_terminated()
            && self.low_rx.is_terminated()
    }
}
```

#### 1.4 Integrate into Peer Actor

**File**: `network/framework/src/peer/mod.rs`

Modify the writer task to use the priority queue:

```rust
// In start_writer_task(), replace the single write_reqs channel with PriorityMessageQueue

fn start_writer_task(
    executor: &Handle,
    time_service: TimeService,
    connection_metadata: ConnectionMetadata,
    network_context: NetworkContext,
    mut writer: MultiplexMessageSink<impl AsyncWrite + Unpin + Send + 'static>,
    max_frame_size: usize,
    max_message_size: usize,
) -> (PriorityMessageQueue, oneshot::Sender<()>) {
    let priority_queue = PriorityMessageQueue::new();
    let mut queue_rx = priority_queue.clone(); // Need to make PriorityMessageQueue cloneable
    
    // ... rest of implementation uses queue_rx.next() instead of write_reqs_rx.next()
}
```

#### 1.5 Update Message Sending

**File**: `network/framework/src/peer/mod.rs`

When handling outbound requests, use the protocol's priority:

```rust
fn handle_outbound_request(
    &mut self,
    request: PeerRequest,
    priority_queue: &PriorityMessageQueue,
) {
    match request {
        PeerRequest::SendDirectSend(message) => {
            let priority = message.protocol_id.priority();
            let network_message = NetworkMessage::DirectSendMsg(DirectSendMsg {
                protocol_id: message.protocol_id,
                priority: priority as u8,
                raw_msg: Vec::from(message.mdata.as_ref()),
            });
            
            if let Err(msg) = priority_queue.push(priority, network_message) {
                // Handle queue full - log and drop
                counters::direct_send_messages(&self.network_context, FAILED_LABEL).inc();
            }
        }
        // Similar for RPC...
    }
}
```

### Testing Strategy

1. **Unit Tests**: Test priority queue ordering guarantees
2. **Integration Tests**: Verify consensus messages are prioritized during simulated load
3. **Performance Tests**: Measure latency improvement for consensus under heavy state sync

### Metrics to Add

```rust
// In counters.rs
pub static PRIORITY_QUEUE_DEPTH: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_network_priority_queue_depth",
        "Current depth of priority message queues",
        &["network_id", "priority"]
    )
    .unwrap()
});

pub static PRIORITY_QUEUE_DROPS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_network_priority_queue_drops",
        "Messages dropped due to full priority queues",
        &["network_id", "priority"]
    )
    .unwrap()
});
```

### Expected Impact

| Metric | Before | After (Expected) |
|--------|--------|------------------|
| Consensus vote latency (p99) under load | 50-150ms | 10-30ms |
| Vote drops during state sync | Occasional | Rare |

---

## 2. Multiple Concurrent Inbound Streams

### Problem Statement

The current `InboundStreamBuffer` only supports **one** concurrent inbound stream per peer. If a peer sends two large messages simultaneously, the second stream is discarded.

**Current Code** (`network/framework/src/protocols/stream/mod.rs`):
```rust
pub struct InboundStreamBuffer {
    stream: Option<InboundStream>,  // Only ONE stream!
    max_fragments: usize,
}
```

### Proposed Solution

Replace single `Option<InboundStream>` with a `HashMap<RequestId, InboundStream>` to support multiple concurrent streams.

### Implementation Details

#### 2.1 Modify InboundStreamBuffer

**File**: `network/framework/src/protocols/stream/mod.rs`

```rust
use std::collections::HashMap;

/// Maximum number of concurrent inbound streams per peer
const MAX_CONCURRENT_INBOUND_STREAMS: usize = 4;

/// A buffer for multiple concurrent inbound fragment streams
pub struct InboundStreamBuffer {
    /// Active streams indexed by request_id
    streams: HashMap<u32, InboundStream>,
    /// Maximum fragments per stream
    max_fragments: usize,
    /// Maximum concurrent streams
    max_concurrent_streams: usize,
}

impl InboundStreamBuffer {
    pub fn new(max_fragments: usize) -> Self {
        Self::with_max_streams(max_fragments, MAX_CONCURRENT_INBOUND_STREAMS)
    }

    pub fn with_max_streams(max_fragments: usize, max_concurrent_streams: usize) -> Self {
        Self {
            streams: HashMap::with_capacity(max_concurrent_streams),
            max_fragments,
            max_concurrent_streams,
        }
    }

    /// Start a new inbound stream
    pub fn new_stream(&mut self, header: StreamHeader) -> anyhow::Result<()> {
        let request_id = header.request_id;
        
        // Check if we already have a stream with this request_id
        if self.streams.contains_key(&request_id) {
            bail!(
                "Stream with request_id {} already exists",
                request_id
            );
        }
        
        // Check if we've hit the concurrent stream limit
        if self.streams.len() >= self.max_concurrent_streams {
            // Find and remove the oldest stream (could also use LRU)
            // For now, reject the new stream
            bail!(
                "Maximum concurrent streams ({}) reached, rejecting stream {}",
                self.max_concurrent_streams,
                request_id
            );
        }
        
        let inbound_stream = InboundStream::new(header, self.max_fragments)?;
        self.streams.insert(request_id, inbound_stream);
        Ok(())
    }

    /// Append a fragment to an existing stream
    pub fn append_fragment(
        &mut self,
        fragment: StreamFragment,
    ) -> anyhow::Result<Option<NetworkMessage>> {
        let request_id = fragment.request_id;
        
        // Find the stream for this fragment
        let stream = self
            .streams
            .get_mut(&request_id)
            .ok_or_else(|| anyhow::anyhow!(
                "No stream exists for request_id {}",
                request_id
            ))?;
        
        // Append the fragment
        let stream_complete = stream.append_fragment(fragment)?;
        
        // If complete, remove and return the message
        if stream_complete {
            let completed_stream = self.streams.remove(&request_id).unwrap();
            Ok(Some(completed_stream.into_message()))
        } else {
            Ok(None)
        }
    }

    /// Get the number of active streams
    pub fn active_stream_count(&self) -> usize {
        self.streams.len()
    }

    /// Clean up stale streams (streams that haven't received fragments recently)
    pub fn cleanup_stale_streams(&mut self, max_age: std::time::Duration) {
        let now = std::time::Instant::now();
        self.streams.retain(|_, stream| {
            stream.last_fragment_time.elapsed() < max_age
        });
    }
}
```

#### 2.2 Add Timestamp Tracking to InboundStream

**File**: `network/framework/src/protocols/stream/mod.rs`

```rust
pub struct InboundStream {
    request_id: u32,
    num_fragments: u8,
    received_fragment_id: u8,
    message: NetworkMessage,
    /// Timestamp of last received fragment (for stale stream cleanup)
    last_fragment_time: std::time::Instant,
}

impl InboundStream {
    fn new(header: StreamHeader, max_fragments: usize) -> anyhow::Result<Self> {
        // ... existing validation ...
        
        Ok(Self {
            request_id: header.request_id,
            num_fragments: header.num_fragments,
            received_fragment_id: 0,
            message: header.message,
            last_fragment_time: std::time::Instant::now(),
        })
    }

    fn append_fragment(&mut self, fragment: StreamFragment) -> anyhow::Result<bool> {
        // Update timestamp
        self.last_fragment_time = std::time::Instant::now();
        
        // ... existing logic ...
    }

    /// Consume the stream and return the completed message
    fn into_message(self) -> NetworkMessage {
        self.message
    }
}
```

#### 2.3 Add Periodic Cleanup in Peer Actor

**File**: `network/framework/src/peer/mod.rs`

Add a periodic cleanup task to remove stale streams:

```rust
// In the Peer::start() event loop, add a cleanup interval
let cleanup_interval = self.time_service.interval(Duration::from_secs(30));
tokio::pin!(cleanup_interval);

loop {
    futures::select! {
        // ... existing handlers ...
        
        _ = cleanup_interval.select_next_some() => {
            // Clean up streams that haven't received data in 60 seconds
            self.inbound_stream.cleanup_stale_streams(Duration::from_secs(60));
        }
    }
}
```

### Testing Strategy

1. **Unit Tests**: 
   - Multiple concurrent streams complete successfully
   - Exceeding max streams is handled gracefully
   - Stale stream cleanup works correctly

2. **Integration Tests**:
   - Two large state sync responses received concurrently
   - Interleaved fragments from different streams

### Metrics to Add

```rust
pub static CONCURRENT_INBOUND_STREAMS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_network_concurrent_inbound_streams",
        "Number of concurrent inbound message streams",
        &["network_id", "peer_id"]
    )
    .unwrap()
});

pub static STREAM_REJECTIONS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_network_stream_rejections",
        "Streams rejected due to concurrent limit",
        &["network_id"]
    )
    .unwrap()
});
```

### Expected Impact

| Metric | Before | After |
|--------|--------|-------|
| Concurrent large message handling | 1 per peer | 4 per peer |
| Message drops during parallel transfers | Common | Rare |

---

## 3. Enhanced Health Monitoring

### Problem Statement

The current `HealthChecker` only performs network-level ping/pong. It cannot detect:
- Application-layer issues (blocked consensus handler)
- Protocol-specific degradation (slow RPC responses)
- Gradual performance degradation

### Proposed Solution

Implement comprehensive health scoring that tracks per-protocol metrics.

### Implementation Details

#### 3.1 Define Health Data Structure

**File**: `network/framework/src/protocols/health_checker/health_data.rs` (new file)

```rust
use crate::ProtocolId;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Rolling window size for metric calculations
const METRIC_WINDOW_SIZE: usize = 100;

/// Health data for a single peer
#[derive(Debug, Clone)]
pub struct PeerHealthData {
    /// Network-level ping RTT (milliseconds)
    ping_rtt: RollingAverage,
    
    /// Per-protocol RPC success rate
    rpc_success_rates: HashMap<ProtocolId, RollingAverage>,
    
    /// Per-protocol RPC latency (milliseconds)
    rpc_latencies: HashMap<ProtocolId, RollingAverage>,
    
    /// Last time we received any message from this peer
    last_message_time: Option<Instant>,
    
    /// Consecutive health check failures
    consecutive_failures: u32,
    
    /// Computed health score (0.0 = unhealthy, 1.0 = healthy)
    health_score: f64,
}

impl PeerHealthData {
    pub fn new() -> Self {
        Self {
            ping_rtt: RollingAverage::new(METRIC_WINDOW_SIZE),
            rpc_success_rates: HashMap::new(),
            rpc_latencies: HashMap::new(),
            last_message_time: None,
            consecutive_failures: 0,
            health_score: 1.0,
        }
    }

    /// Record a successful ping with the given RTT
    pub fn record_ping_success(&mut self, rtt_ms: f64) {
        self.ping_rtt.add(rtt_ms);
        self.consecutive_failures = 0;
        self.recalculate_health_score();
    }

    /// Record a ping failure
    pub fn record_ping_failure(&mut self) {
        self.consecutive_failures += 1;
        self.recalculate_health_score();
    }

    /// Record an RPC completion (success or failure)
    pub fn record_rpc_completion(
        &mut self,
        protocol_id: ProtocolId,
        success: bool,
        latency_ms: f64,
    ) {
        // Update success rate
        let success_rate = self
            .rpc_success_rates
            .entry(protocol_id)
            .or_insert_with(|| RollingAverage::new(METRIC_WINDOW_SIZE));
        success_rate.add(if success { 1.0 } else { 0.0 });

        // Update latency (only for successful RPCs)
        if success {
            let latency = self
                .rpc_latencies
                .entry(protocol_id)
                .or_insert_with(|| RollingAverage::new(METRIC_WINDOW_SIZE));
            latency.add(latency_ms);
        }

        self.recalculate_health_score();
    }

    /// Record receipt of any message
    pub fn record_message_received(&mut self) {
        self.last_message_time = Some(Instant::now());
    }

    /// Get the current health score
    pub fn health_score(&self) -> f64 {
        self.health_score
    }

    /// Check if the peer should be considered unhealthy
    pub fn is_unhealthy(&self, failure_threshold: u32) -> bool {
        self.consecutive_failures > failure_threshold || self.health_score < 0.3
    }

    /// Recalculate the composite health score
    fn recalculate_health_score(&mut self) {
        let mut score = 1.0;

        // Factor 1: Ping RTT (weight: 30%)
        // Penalize high latency: >500ms is bad, >1000ms is very bad
        if let Some(avg_rtt) = self.ping_rtt.average() {
            let rtt_score = if avg_rtt < 100.0 {
                1.0
            } else if avg_rtt < 500.0 {
                1.0 - (avg_rtt - 100.0) / 800.0
            } else {
                0.5 - (avg_rtt - 500.0) / 1000.0
            };
            score *= 0.7 + 0.3 * rtt_score.max(0.0);
        }

        // Factor 2: Consecutive failures (weight: 40%)
        let failure_score = match self.consecutive_failures {
            0 => 1.0,
            1 => 0.9,
            2 => 0.7,
            3 => 0.5,
            _ => 0.2,
        };
        score *= 0.6 + 0.4 * failure_score;

        // Factor 3: RPC success rate (weight: 30%)
        if !self.rpc_success_rates.is_empty() {
            let avg_success_rate: f64 = self
                .rpc_success_rates
                .values()
                .filter_map(|r| r.average())
                .sum::<f64>()
                / self.rpc_success_rates.len() as f64;
            score *= 0.7 + 0.3 * avg_success_rate;
        }

        self.health_score = score.clamp(0.0, 1.0);
    }

    /// Get average ping RTT in milliseconds
    pub fn avg_ping_rtt_ms(&self) -> Option<f64> {
        self.ping_rtt.average()
    }

    /// Get RPC success rate for a specific protocol
    pub fn rpc_success_rate(&self, protocol_id: &ProtocolId) -> Option<f64> {
        self.rpc_success_rates.get(protocol_id).and_then(|r| r.average())
    }
}

/// A simple rolling average calculator
#[derive(Debug, Clone)]
struct RollingAverage {
    values: Vec<f64>,
    capacity: usize,
    index: usize,
    count: usize,
}

impl RollingAverage {
    fn new(capacity: usize) -> Self {
        Self {
            values: vec![0.0; capacity],
            capacity,
            index: 0,
            count: 0,
        }
    }

    fn add(&mut self, value: f64) {
        self.values[self.index] = value;
        self.index = (self.index + 1) % self.capacity;
        if self.count < self.capacity {
            self.count += 1;
        }
    }

    fn average(&self) -> Option<f64> {
        if self.count == 0 {
            None
        } else {
            let sum: f64 = self.values[..self.count].iter().sum();
            Some(sum / self.count as f64)
        }
    }
}
```

#### 3.2 Integrate with PeersAndMetadata

**File**: `network/framework/src/application/storage.rs`

Add health data to the peer metadata storage:

```rust
use crate::protocols::health_checker::health_data::PeerHealthData;

impl PeersAndMetadata {
    /// Get or create health data for a peer
    pub fn get_peer_health_data(&self, peer: PeerNetworkId) -> Option<PeerHealthData> {
        // Implementation depends on existing structure
    }

    /// Update health data for a peer
    pub fn update_peer_health_data(
        &self,
        peer: PeerNetworkId,
        update_fn: impl FnOnce(&mut PeerHealthData),
    ) -> Result<(), Error> {
        // Implementation
    }
    
    /// Get peers sorted by health score (healthiest first)
    pub fn get_peers_by_health(&self, network_id: &NetworkId) -> Vec<PeerNetworkId> {
        // Sort connected peers by health_score descending
    }
}
```

#### 3.3 Update HealthChecker to Use New Metrics

**File**: `network/framework/src/protocols/health_checker/mod.rs`

```rust
async fn handle_ping_response(
    &mut self,
    peer_id: PeerId,
    round: u64,
    req_nonce: u32,
    ping_result: Result<Pong, RpcError>,
    start_time: Instant,
) {
    let peer_network_id = PeerNetworkId::new(self.network_context.network_id(), peer_id);
    
    match ping_result {
        Ok(pong) if pong.0 == req_nonce => {
            let rtt_ms = start_time.elapsed().as_secs_f64() * 1000.0;
            
            // Update health data
            if let Err(e) = self.network_interface.get_peers_and_metadata()
                .update_peer_health_data(peer_network_id, |health| {
                    health.record_ping_success(rtt_ms);
                })
            {
                warn!("Failed to update health data: {:?}", e);
            }
            
            // Existing success handling...
        }
        _ => {
            // Update health data for failure
            if let Err(e) = self.network_interface.get_peers_and_metadata()
                .update_peer_health_data(peer_network_id, |health| {
                    health.record_ping_failure();
                })
            {
                warn!("Failed to update health data: {:?}", e);
            }
            
            // Check if peer should be disconnected based on health score
            if let Some(health) = self.network_interface.get_peers_and_metadata()
                .get_peer_health_data(peer_network_id)
            {
                if health.is_unhealthy(self.ping_failures_tolerated as u32) {
                    // Disconnect unhealthy peer
                    self.disconnect_peer(peer_id).await;
                }
            }
        }
    }
}
```

### Metrics to Export

```rust
pub static PEER_HEALTH_SCORE: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "aptos_network_peer_health_score",
        "Health score for connected peers (0.0-1.0)",
        &["network_id", "peer_id"]
    )
    .unwrap()
});

pub static PEER_RPC_SUCCESS_RATE: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "aptos_network_peer_rpc_success_rate",
        "RPC success rate per peer per protocol",
        &["network_id", "peer_id", "protocol_id"]
    )
    .unwrap()
});
```

### Expected Impact

| Metric | Before | After |
|--------|--------|-------|
| Degraded peer detection time | Minutes (after N ping failures) | Seconds (continuous scoring) |
| False positive disconnections | Higher | Lower (multi-factor scoring) |
| Peer selection quality | Random | Health-aware |

---

## 4. Serialization Optimization

### Problem Statement

Every message goes through BCS serialization/deserialization. For high-frequency messages (consensus votes), this adds CPU overhead.

### Proposed Solution

1. Add zero-copy deserialization where possible
2. Cache serialized bytes for repeated sends
3. Profile and optimize hot paths

### Implementation Details

#### 4.1 Zero-Copy Deserialization for Direct Send

**File**: `network/framework/src/protocols/wire/messaging/v1/mod.rs`

Use `bcs::from_bytes_with_limit` with borrowed data where possible:

```rust
/// Deserialize a NetworkMessage with zero-copy optimization
pub fn deserialize_network_message(bytes: &[u8]) -> Result<NetworkMessage, bcs::Error> {
    // Use the existing BCS deserialization but ensure we're not copying unnecessarily
    bcs::from_bytes(bytes)
}

/// For messages that will be forwarded without modification,
/// keep the raw bytes to avoid re-serialization
#[derive(Debug)]
pub struct RawNetworkMessage {
    /// The deserialized message (for routing decisions)
    pub message: NetworkMessage,
    /// The original serialized bytes (for forwarding)
    pub raw_bytes: bytes::Bytes,
}

impl RawNetworkMessage {
    pub fn from_bytes(bytes: bytes::Bytes) -> Result<Self, bcs::Error> {
        let message = bcs::from_bytes(&bytes)?;
        Ok(Self { message, raw_bytes: bytes })
    }
}
```

#### 4.2 Message Serialization Cache

**File**: `network/framework/src/protocols/network/mod.rs`

For broadcast scenarios (sending same message to many peers):

```rust
use bytes::Bytes;
use std::sync::Arc;

/// A message that has been pre-serialized for efficient multi-peer sending
pub struct PreserializedMessage<T> {
    /// The original message (for local handling if needed)
    pub message: Arc<T>,
    /// Pre-serialized bytes per protocol
    serialized: HashMap<ProtocolId, Bytes>,
}

impl<T: serde::Serialize> PreserializedMessage<T> {
    pub fn new(message: T, protocols: &[ProtocolId]) -> Result<Self, bcs::Error> {
        let message = Arc::new(message);
        let mut serialized = HashMap::new();
        
        for protocol in protocols {
            let bytes = protocol.to_bytes(&*message)?;
            serialized.insert(*protocol, Bytes::from(bytes));
        }
        
        Ok(Self { message, serialized })
    }

    pub fn get_bytes(&self, protocol: &ProtocolId) -> Option<&Bytes> {
        self.serialized.get(protocol)
    }
}
```

#### 4.3 Optimize Hot Path: Consensus Vote Handling

Profile the consensus vote path and add targeted optimizations:

```rust
// In the consensus vote handling path, avoid unnecessary clones
impl NetworkSender<ConsensusMsg> {
    /// Optimized broadcast for consensus votes
    pub fn broadcast_vote_optimized(
        &self,
        vote: Vote,
        peers: &[PeerId],
    ) -> Result<(), NetworkError> {
        // Pre-serialize once
        let msg = ConsensusMsg::VoteMsg(Box::new(VoteMsg::new(vote, ...)));
        let preserialized = PreserializedMessage::new(
            msg,
            &[ProtocolId::ConsensusDirectSendBcs],
        )?;
        
        // Send to all peers using pre-serialized bytes
        for peer in peers {
            self.send_to_raw(
                *peer,
                ProtocolId::ConsensusDirectSendBcs,
                preserialized.get_bytes(&ProtocolId::ConsensusDirectSendBcs)
                    .unwrap()
                    .clone(),
            )?;
        }
        
        Ok(())
    }
}
```

### Profiling Recommendations

Add timing metrics around serialization:

```rust
pub fn serialize_with_metrics<T: serde::Serialize>(
    protocol_id: ProtocolId,
    message: &T,
) -> Result<Vec<u8>, bcs::Error> {
    let timer = counters::start_serialization_timer(protocol_id, SERIALIZATION_LABEL);
    let result = bcs::to_bytes(message);
    timer.stop_and_record();
    result
}
```

### Expected Impact

| Metric | Before | After |
|--------|--------|-------|
| CPU usage for vote broadcast (100 peers) | 100x serialization | 1x serialization |
| Serialization overhead per message | ~10-100μs | ~10-100μs (but fewer total) |

---

## Summary: Phase 1 Checklist

### Week 1-2: Priority Message Queues
- [ ] Define `MessagePriority` enum
- [ ] Add `priority()` method to `ProtocolId`
- [ ] Implement `PriorityMessageQueue`
- [ ] Integrate into Peer actor
- [ ] Add metrics
- [ ] Write unit tests
- [ ] Integration testing under load

### Week 3-4: Multiple Concurrent Streams
- [ ] Modify `InboundStreamBuffer` to use HashMap
- [ ] Add timestamp tracking for stale stream cleanup
- [ ] Add cleanup task to Peer actor
- [ ] Add metrics
- [ ] Write unit tests
- [ ] Test concurrent large message handling

### Week 5-6: Enhanced Health Monitoring
- [ ] Implement `PeerHealthData` structure
- [ ] Integrate with `PeersAndMetadata`
- [ ] Update `HealthChecker` to use new scoring
- [ ] Add health-based peer selection
- [ ] Export metrics
- [ ] Integration testing

### Week 7-8: Serialization Optimization
- [ ] Profile current serialization hot spots
- [ ] Implement `PreserializedMessage` for broadcasts
- [ ] Add serialization metrics
- [ ] Optimize consensus vote path
- [ ] Benchmark improvements

---

## Success Metrics

At the end of Phase 1, measure:

1. **Consensus Latency**: p50, p99 vote propagation time
2. **State Sync Throughput**: MB/s during catch-up
3. **CPU Usage**: Per-validator during peak load
4. **Message Drop Rate**: Especially for critical messages
5. **Peer Health Detection**: Time to detect and disconnect unhealthy peers

Target improvements:
- 30-50% reduction in consensus message latency under load
- Zero message drops for critical priority messages
- 2-4x improvement in concurrent large message handling
- 50% faster detection of degraded peers

---

*Document Version: 1.0*  
*Last Updated: January 27, 2026*
