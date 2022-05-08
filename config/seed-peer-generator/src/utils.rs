// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::Error;
use aptos_config::config::{Peer, PeerRole, PeerSet};
use aptos_logger::prelude::*;
use aptos_rest_client::Client;
use aptos_types::{
    account_config::aptos_root_address,
    network_address::NetworkAddress,
    on_chain_config::{access_path_for_config, OnChainConfig, ValidatorSet},
    validator_info::ValidatorInfo,
    PeerId,
};

/// Retrieve the Fullnode seed peers from JSON-RPC
pub fn gen_validator_full_node_seed_peer_config(
    client_endpoint: String,
) -> anyhow::Result<PeerSet> {
    let validator_set = get_validator_set(client_endpoint)?;

    gen_seed_peers(
        &validator_set,
        PeerRole::ValidatorFullNode,
        to_fullnode_addresses,
    )
}

pub(crate) fn to_fullnode_addresses(
    validator_info: &ValidatorInfo,
) -> Result<Vec<NetworkAddress>, bcs::Error> {
    validator_info.config().fullnode_network_addresses()
}

/// Retrieve the validator set from a JSON RPC endpoint
fn get_validator_set(client_endpoint: String) -> anyhow::Result<ValidatorSet> {
    let client = Client::new(url::Url::parse(&client_endpoint)?);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let validator_set_response = rt.block_on(client.get_resource::<ValidatorSet>(
        aptos_root_address(),
        std::str::from_utf8(&access_path_for_config(ValidatorSet::CONFIG_ID).path)?,
    ))?;
    Ok(validator_set_response.inner().clone())
}

// TODO: Merge with OnchainDiscovery
pub(crate) fn gen_seed_peers<
    ToAddresses: Fn(&ValidatorInfo) -> Result<Vec<NetworkAddress>, bcs::Error>,
>(
    validator_set: &ValidatorSet,
    role: PeerRole,
    to_addresses: ToAddresses,
) -> anyhow::Result<PeerSet> {
    let set = validator_set
        .payload()
        .filter_map(|validator_info| {
            to_seed_peer(validator_info, role, &to_addresses).map_or_else(
                |error| {
                    warn!(
                        "Unable to generate seed for validator {} {}",
                        validator_info.account_address(),
                        error
                    );
                    None
                },
                Some,
            )
        })
        .collect::<PeerSet>();

    if set.is_empty() {
        Err(Error::msg("No seed peers were generated"))
    } else {
        Ok(set)
    }
}

/// Convert ValidatorInfo to a seed peer
fn to_seed_peer<T: Fn(&ValidatorInfo) -> Result<Vec<NetworkAddress>, bcs::Error>>(
    validator_info: &ValidatorInfo,
    role: PeerRole,
    to_addresses: &T,
) -> Result<(PeerId, Peer), bcs::Error> {
    let peer_id = *validator_info.account_address();
    let addrs = to_addresses(validator_info)?;
    Ok((peer_id, Peer::from_addrs(role, addrs)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_config::config::HANDSHAKE_VERSION;
    use aptos_crypto::{ed25519::Ed25519PrivateKey, x25519, PrivateKey as PK, Uniform};
    use aptos_types::validator_config::ValidatorConfig;
    use rand::{prelude::StdRng, SeedableRng};

    fn validator_set(
        peer_id: PeerId,
        fullnode_network_addresses: &[NetworkAddress],
    ) -> ValidatorSet {
        let consensus_private_key = Ed25519PrivateKey::generate_for_testing();
        let consensus_pubkey = consensus_private_key.public_key();
        let validator_config = ValidatorConfig::new(
            consensus_pubkey,
            vec![],
            bcs::to_bytes(fullnode_network_addresses).unwrap(),
        );
        let validator_info = ValidatorInfo::new(peer_id, 0, validator_config);
        ValidatorSet::new(vec![validator_info])
    }

    fn generate_network_addresses(num: u64) -> Vec<NetworkAddress> {
        let mut addresses = Vec::new();
        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        for _ in 0..num {
            let private_key = x25519::PrivateKey::generate(&mut rng);
            let pubkey = private_key.public_key();
            let network_address =
                NetworkAddress::mock().append_prod_protos(pubkey, HANDSHAKE_VERSION);
            addresses.push(network_address);
        }

        addresses
    }

    #[test]
    fn validator_fullnode_test() {
        let role = PeerRole::ValidatorFullNode;
        let peer_id = PeerId::random();
        let fullnode_addresses = generate_network_addresses(3);
        let validator_set = validator_set(peer_id, &fullnode_addresses);
        let result = gen_seed_peers(&validator_set, role, to_fullnode_addresses).unwrap();

        let mut expected_peers = PeerSet::new();
        expected_peers.insert(peer_id, Peer::from_addrs(role, fullnode_addresses));
        assert_eq!(expected_peers, result);
    }
}
