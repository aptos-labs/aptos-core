// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{format_err, Result};
use aptos_types::{chain_id::ChainId, PeerId};
use once_cell::sync::OnceCell;
use std::sync::Arc;

/// The global [AptosNodeIdentity]
static APTOS_NODE_IDENTITY: OnceCell<Arc<AptosNodeIdentity>> = OnceCell::new();

/// Structure that holds information related to a node's identity
pub struct AptosNodeIdentity {
    pub chain_id: OnceCell<ChainId>,
    pub peer_id: Option<PeerId>,
    // Holds Peer ID as String to reduce overhead for frequent lookups.
    pub peer_id_str: Option<String>,
    pub git_hash: OnceCell<&'static str>,
}

/// Initializes the [AptosNodeIdentity] using the provided [PeerId] and
/// sets it globally exactly once.
pub fn init(peer_id: Option<PeerId>) -> Result<()> {
    let identity = AptosNodeIdentity {
        chain_id: OnceCell::new(),
        peer_id,
        peer_id_str: peer_id.map(|id| id.to_string()),
        git_hash: OnceCell::new(),
    };

    APTOS_NODE_IDENTITY
        .set(Arc::new(identity))
        .map_err(|_| format_err!("APTOS_NODE_IDENTITY was already set"))
}

/// Sets the [ChainId] in the global [AptosNodeIdentity], returning an error
/// if [init] was not called already.
pub fn set_chain_id(chain_id: ChainId) -> Result<()> {
    match APTOS_NODE_IDENTITY.get() {
        Some(identity) => identity
            .chain_id
            .set(chain_id)
            .map_err(|_| format_err!("chain_id was already set.")),
        None => Err(format_err!("APTOS_NODE_IDENTITY has not been set yet")),
    }
}

/// Sets the [GitHash] in the global [AptosNodeIdentity], returning an error
/// if [init] was not called already.
pub fn set_git_hash(git_hash: String) -> Result<()> {
    let short_git_hash = if git_hash.len() >= 7 {
        git_hash[..7].to_string()
    } else {
        git_hash.to_string()
    };
    // Convert to &'static str by leaking the String
    let static_hash = Box::leak(short_git_hash.into_boxed_str());
    match APTOS_NODE_IDENTITY.get() {
        Some(identity) => identity
            .git_hash
            .set(static_hash)
            .map_err(|_| format_err!("git_hash was already set.")),
        None => Err(format_err!("APTOS_NODE_IDENTITY has not been set yet")),
    }
}

/// Returns the [PeerId] from the global `APTOS_NODE_IDENTITY`
pub fn peer_id() -> Option<PeerId> {
    APTOS_NODE_IDENTITY
        .get()
        .and_then(|identity| identity.peer_id)
}

/// Returns the [PeerId] as [str] from the global `APTOS_NODE_IDENTITY`
pub fn peer_id_as_str() -> Option<&'static str> {
    APTOS_NODE_IDENTITY
        .get()
        .and_then(|identity| identity.peer_id_str.as_deref())
}

/// Returns the [ChainId] from the global `APTOS_NODE_IDENTITY`
pub fn chain_id() -> Option<ChainId> {
    APTOS_NODE_IDENTITY
        .get()
        .and_then(|identity| identity.chain_id.get().cloned())
}

/// Returns the [GitHash] as [str] from the global `APTOS_NODE_IDENTITY`
pub fn git_hash() -> Option<&'static str> {
    APTOS_NODE_IDENTITY
        .get()
        .and_then(|identity| identity.git_hash.get().copied())
}

#[cfg(test)]
mod tests {
    use aptos_types::{chain_id::ChainId, PeerId};
    use claims::{assert_err, assert_ok};

    #[test]
    fn test_aptos_node_identity() {
        // Should return None before init is called
        assert_eq!(super::peer_id(), None);
        assert_eq!(super::chain_id(), None);

        // Init with peer_id
        let peer_id = PeerId::random();
        assert_ok!(super::init(Some(peer_id)));

        assert_eq!(super::peer_id(), Some(peer_id));
        assert_eq!(
            super::peer_id_as_str(),
            Some(peer_id.to_string()).as_deref()
        );

        // Calling init again should error
        assert_err!(super::init(None));

        // Init chain_id
        let chain_id = ChainId::test();
        assert_ok!(super::set_chain_id(chain_id));
        assert_eq!(super::chain_id(), Some(chain_id));

        // Calling set chain ID again should error
        assert_err!(super::set_chain_id(chain_id));
    }
}
