---
id: network
title: Network
custom_edit_url: https://github.com/velor-chain/velor-core/edit/main/network/README.md
---

## Overview

For more detailed info, see the [VelorNet Specification](../documentation/specifications/network/README.md).

VelorNet is the primary protocol for communication between any two nodes in the
Velor ecosystem. It is specifically designed to facilitate the consensus, shared
mempool, and state sync protocols. VelorNet tries to maintain at-most one connection
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

## Implementation Details

### System Architecture

```
                      +-----------+---------+------------+--------+
 Application Modules  | Consensus | Mempool | State Sync | Health |
                      +-----------+---------+------------+--------+
                            ^          ^          ^           ^
   Network Interface        |          |          |           |
                            v          v          v           v
                      +----------------+--------------------------+   +---------------------+
      Network Module  |                 PeerManager               |<->| ConnectivityManager |
                      +----------------------+--------------------+   +---------------------+
                      |        Peer(s)       |                    |
                      +----------------------+                    |
                      |                VelorTransport             |
                      +-------------------------------------------+
```

The network component is implemented in the
[Actor](https://en.wikipedia.org/wiki/Actor_model) model &mdash; it uses
message-passing to communicate between different subcomponents running as
independent "tasks." The [tokio](https://tokio.rs/) framework is used as the task
runtime. The primary subcomponents in the network module are:

* [`Network Interface`] &mdash; The interface provided to application modules
using VelorNet.

* [`PeerManager`] &mdash; Listens for incoming connections, and dials outbound
connections to other peers. Demultiplexes and forwards inbound messages from
[`Peer`]s to appropriate application handlers. Additionally, notifies upstream
components of new or closed connections. Optionally can be connected to
[`ConnectivityManager`] for a network with Discovery.

* [`Peer`] &mdash; Manages a single connection to another peer. It reads and
writes [`NetworkMessage`]es from/to the wire. Currently, it implements the two
protocols: DirectSend and Rpc.

+ [`VelorTransport`] &mdash; A secure, reliable transport. It uses [NoiseIK] over
TCP to negotiate an encrypted and authenticated connection between peers.
The VelorNet version and any Velor-specific application protocols are negotiated
afterward using the [VelorNet Handshake Protocol].

* [`ConnectivityManager`] &mdash; Establishes connections to known peers found
via Discovery. Notifies [`PeerManager`] to make outbound dials, or disconnects based
on updates to known peers via Discovery updates.

* [`validator-set-discovery`] &mdash; Discovers the set of peers to connect to
via on-chain configuration. These are the `validator_network_addresses` and
`fullnode_network_addresses` of each [`ValidatorConfig`] in the
[`ValidatorSet`] set. Notifies the [`ConnectivityManager`] of updates
to the known peer set.

* [`HealthChecker`] &mdash; Performs periodic liveness probes to ensure the
health of a peer/connection. It resets the connection with the peer if a
configurable number of probes fail in succession. Probes currently fail on a
configurable static timeout.

## How is this module organized?
    ../types/src
    ├── network-address            # Network addresses and encryption

    network
    ├── builder                    # Builds a network from a NetworkConfig
    ├── memsocket                  # In-memory socket interface for tests
    ├── netcore
    │   └── src
    │       ├── transport          # Composable transport API
    │       └── framing            # Read/write length prefixes to sockets
    ├── discovery                  # Protocols for peer discovery
    └── src
        ├── application
        ├── peer_manager           # Manage peer connections and messages to/from peers
        ├── peer                   # Handles a single peer connection's state
        ├── connectivity_manager   # Monitor connections and ensure connectivity
        ├── protocols
        │   ├── network            # Application layer interface to network module
        │   ├── direct_send        # Protocol for fire-and-forget style message delivery
        │   ├── health_checker     # Protocol for health probing
        │   ├── rpc                # Protocol for remote procedure calls
        │   └── wire               # Protocol for VelorNet handshakes and messaging
        ├── transport              # The base transport layer for dialing/listening
        └── noise                  # Noise handshaking and wire integration

[`NetworkConfig`]:../config/src/config/network_config.rs
[`ConnectivityManager`]: ./src/connectivity_manager/mod.rs
[`VelorNet Handshake Protocol`]: ../specifications/network/handshake-v1.md
[`ValidatorSet`]: ../types/src/on_chain_config/validator_set.rs
[`VelorTransport`]: ./src/transport/mod.rs
[`HealthChecker`]: ./src/protocols/health_checker/mod.rs
[`Network Interface`]: ./src/protocols/network/mod.rs
[`NetworkMessage`]: ./src/protocols/wire/messaging/v1/mod.rs
[`NoiseIK`]: ../specifications/network/noise.md
[`PeerManager`]: ./src/peer_manager/mod.rs
[`Peer`]: ./src/peer/mod.rs
[`ValidatorConfig`]: ../documentation/specifications/network/onchain-discovery.md#on-chain-config
[`validator-set-discovery`]: discovery/src/lib.rs
