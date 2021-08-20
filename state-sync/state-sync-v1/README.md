---
id: state_sync_v1
title: State Sync v1
custom_edit_url: https://github.com/diem/diem/edit/main/state-sync/state-sync-v1/README.md
---

*** **Note: there are plans to build and deploy a new version of state sync. As
such, this version (v1) will be deprecated in the near future. See this
[issue](https://github.com/diem/diem/issues/8906) for more information.** ***

# State synchronizer (State sync)

State sync is a component that helps Diem nodes advance local blockchain ledger
state by requesting and sharing transactions between peers. This helps nodes
to synchronize with the most up-to-date state of the blockchain (e.g., if they
fall behind or are freshly deployed).

## Overview

Refer to the [State Sync Specification](../../specifications/state_sync) for a
high-level overview and description of State Sync.

## Implementation details

This crate contains a state sync implementation as described in the
specification mentioned above. The files of note in this crate are:
- `bootstrapper.rs`: the wrapper struct for creating state sync instances and
local clients (`client.rs`) to those instances.
- `chunk_request.rs` & `chunk_response.rs`: the definitions of the messages sent
between Diem nodes when making state sync requests and responses.
- `coordinator.rs`: the primary state sync runtime that processes messages (e.g.,
from other Diem nodes) and reacts appropriately.
- `executor_proxy.rs`: the interface between the state sync coordinator and
both storage and execution.
- `request_manager.rs`: the actor that manages the network requests and responses
 between peers.

## How is this module organized?
```
state-sync
|- src                         # Source code and unit tests
|- tests/                      # Integration tests
```
