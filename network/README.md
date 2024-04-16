---
id: network
title: Network
custom_edit_url: https://github.com/aptos-labs/aptos-core/edit/main/network/README.md
---

## Overview

For more detailed info, see the [AptosNet Specification](../documentation/specifications/network/README.md).

AptosNet is the primary protocol for communication between any two nodes in the
Aptos ecosystem. It is specifically designed to facilitate the consensus, shared
mempool, and state sync protocols. AptosNet tries to maintain at-most one connection
with each remote peer; the application protocols to that remote peer are then
multiplexed over the single peer connection.

Currently, it provides application protocols with two primary interfaces:

* DirectSend: for fire-and-forget style message delivery.
* RPC: for unary Remote Procedure Calls.

The network component uses:

* TCP for reliable transport.
* [NoiseIK] for authentication and full end-to-end encryption.
* On-chain [`NetworkAddress`](../types/src/network_address/mod.rs) set for discovery, with
  optional seed peers in the [`NetworkConfig`]
  as a fallback.

Validators will only allow connections from other validators. Their identity and
public key information is provided by the [`validator-set-discovery`] protocol,
which updates the eligible member information on each consensus reconfiguration.
Each member of the validator network maintains a full membership view and connects
directly to all other validators in order to maintain a full-mesh network.

In contrast, Validator Full Node (VFNs) servers will only prioritize connections
from more trusted peers in the on-chain discovery set; they will still service
any public clients. Public Full Nodes (PFNs) connecting to VFNs will always
authenticate the VFN server using the available discovery information.

Validator health information, determined using periodic liveness probes, is not
shared between validators; instead, each validator directly monitors its peers
for liveness using the [`HealthChecker`] protocol.

This approach should scale up to a few hundred validators before requiring
partial membership views, sophisticated failure detectors, or network overlays.

The current internal design tries to be _as flat as possible_. There is one queue outbound to each peer with messages to send (actually two in parallel for high priority and low priority, but only one queue of _depth_). There is one queue inbound towards each application handler. That's it. The application assembles an outbound message and it goes on the peer queue(s). The peer code decodes a message and it goes on the app queue.

## Network Design Stories

The design of the Aptos blockchain networking code, as told through a few stories...

### A message comes in from a peer

The TCP stream is wrapped by the [noise](framkework/src/noise/) streaming encryption protocol.

The decrypted stream flows through tokio_util [FramedRead+LengthDelimitedCodec](framework/src/protocols/wire/messaging/v1/mod.rs) which breaks up the stream into messages.

[MultiplexMessageStream](framework/src/protocols/wire/messaging/v1/mod.rs) reads the message blob and decodes the BCS into a Stream of Rust enum MultiplexMessage

[Peer code ReaderContext](framework/src/peer.rs) handles the MultiplexMessage which might be a NetworkMessage enum, or a StreamMessage chunk where many chunks gets reassembled to a NetworkMessage. If the NetworkMessage is an RpcRequest, RpcResponse, or DirectSndMsg, peer code forwards that message to the inbound queue for some application code.

[NetworkEvents code](framework/src/protocols/network/mod.rs), running in application thread, unpacks a blob by a second round of BCS decode into the application’s message struct type. NetworkEvents presents this message to the application in a Stream implementation.

### Application code sends a message to a peer

[NetworkClient](framework/src/application/interface.rs) is the interface for application code to send messages to other peers, either a one way direct message or an RPC request with waiting for reply.

Just under that is the [NetworkSender](framework/src/protocols/network/mod.rs) which starts the fan out into validator-network/vfn-network/pfn-network. NetworkSender encodes the application struct into bytes to put into a NetworkMessage, then puts that NetworkMessage on a queue to the peer writer thread. Some ProtocolId-s are ‘high priority’ and go on a special queue.

[Peer code WriterContext](framework/src/peer.rs) pulls messages from its queues (high priority preferentially, and low priority). Small messages are wrapped in a MultiplexMessage and sent directly, large messages are split into StreamMessage chunks. One large message is held and its chunks are sent out alternating with small messages. If a second large message arrives the first large message gets all of its chunks sent.

Peer code writes into a MultiplexMessageSink object which writes into a tokio_util FramedWrite+LengthDelimitedCodec object that writes into a noise stream that writes to the TCP socket.

### A new peer connects

[PeerListener](builder/src/peer_listener.rs) holds the listening socket bound to some port.

PeerListener::check_new_inbound_connection() does a bunch of checks, if all successful it starts [Peer code](framework/src/peer.rs) which inserts the new peer into the PeersAndMetadata object and starts several tokio tasks to handle the peer.

### Outbound connections to peers

The [ConnectivityManager](framework/src/connectivity_manager/mod.rs) code checks periodically[1] if it has the configured number of outbound connections[2], and if not starts a new one.

ConnectivityManager can also close a connection to an outbound peer if [DiscoveryChangeListener](discovery/src/lib.rs) finds out that a peer should no longer be connected to (e.g. a Validator leaves the set of Validators).

An attempt at an outbound connection should close if a successful inbound connection completes first.

Most of the sequence of connecting to a peer is in ConnectivityManager::queue_dial_peer()

[1] config `connectivity_check_interval_ms`, default 5 seconds

[2] config `max_outbound_connections`, default 4

## Network Applications

 * [Mempool](../mempool/) move pending transactions around
 * [Consensus](../consensus/) decide what goes in a block
 * [JWK](../crates/aptos-jwk-consensus/) advances in consensus
 * [State Sync](../state-sync/) move completed block data around
 * [DKG](../dkg/) on-chain randomness
 * [Peer Monitoring Service](peer-monitoring-service/) reliability and latency monitoring of connected peers
 * [HealthChecker](framework/src/protocols/health_checker/) (probably soon to be replaced by Peer Monitoring Service)
 * [Benchmark](benchmark/) how fast can we push data?

## How is this module organized?
    ../types/src
    ├── network-address            # Network addresses and encryption

    ../aptos-node/src/network2.rs  # where node inits and uses all this

    network
    ├── benchmark                  # Service for measuring how fast we can push data
    ├── builder                    # Builds a network from a NetworkConfig
    ├── discovery                  # Protocols for peer discovery
    ├── memsocket                  # In-memory socket interface for tests
    ├── netcore
    │   └── src
    │       ├── transport          # Composable transport API
    │       └── framing            # Read/write length prefixes to sockets
    ├── peer-monitoring-service    # Measure availability and latency of peers
    └── framework/src
        ├── application            # interfaces for acting on the network
        ├── peer                   # Move data to/from a peer
        ├── connectivity_manager   # Monitor connections and ensure connectivity
        ├── protocols
        │   ├── network            # Application layer interface to network module
        │   ├── direct_send        # Protocol for fire-and-forget style message delivery
        │   ├── health_checker     # Protocol for health probing
        │   ├── rpc                # Protocol for remote procedure calls
        │   └── wire               # Protocol for AptosNet handshakes and messaging
        ├── transport              # The base transport layer for dialing/listening
        └── noise                  # Noise handshaking and wire integration

[`NetworkConfig`]:../config/src/config/network_config.rs
[`ConnectivityManager`]: ./src/connectivity_manager/mod.rs
[`AptosNet Handshake Protocol`]: ../specifications/network/handshake-v1.md
[`ValidatorSet`]: ../types/src/on_chain_config/validator_set.rs
[`AptosTransport`]: ./src/transport/mod.rs
[`HealthChecker`]: ./src/protocols/health_checker/mod.rs
[`Network Interface`]: ./src/protocols/network/mod.rs
[`NetworkMessage`]: ./src/protocols/wire/messaging/v1/mod.rs
[`NoiseIK`]: ../specifications/network/noise.md
[`PeerManager`]: ./src/peer_manager/mod.rs
[`Peer`]: ./src/peer/mod.rs
[`ValidatorConfig`]: ../documentation/specifications/network/onchain-discovery.md#on-chain-config
[`validator-set-discovery`]: discovery/src/lib.rs
[`NetworkClient`]:framework/src/application/interface.rs
[`PeersAndMetadata`]:framework/src/application/storage.rs