// Copyright © Aptos Foundation

use aptos_types::block_info::Round;
use aptos_types::validator_verifier::ValidatorVerifier;
use move_core_types::account_address::AccountAddress as PeerId;
use std::collections::HashSet;
use aptos_consensus_types::node::NodeMetaData;
use crate::dag::dag_storage::{DagStorageItem, DagStoreWriteBatch, ItemId};
use crate::dag::types::peer_index_map::PeerIndexMap;
use crate::dag::types::peer_status_list::{PeerStatusList, PeerStatusListItem};
use crate::dag::types::PeerStatus;
use serde::{Deserialize, Serialize};
use aptos_logger::info;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct WeakLinksCreator_Brief {
    pub(crate) my_id: PeerId,
    pub(crate) latest_nodes_metadata: ItemId,
    pub(crate) address_to_validator_index: ItemId,
}

///keeps track of weak links. None indicates that a (strong or weak) link was already added.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct WeakLinksCreator {
    pub(crate) id: ItemId,
    pub(crate) my_id: PeerId,
    pub(crate) latest_nodes_metadata: PeerStatusList,
    pub(crate) address_to_validator_index: PeerIndexMap,
}

impl WeakLinksCreator {
    pub fn new(my_id: PeerId, verifier: &ValidatorVerifier) -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            my_id,
            latest_nodes_metadata: PeerStatusList::new(verifier
                .address_to_validator_index()
                .iter()
                .map(|_| None)
                .collect()),
            address_to_validator_index: PeerIndexMap::new(verifier.address_to_validator_index().clone()),
        }
    }

    pub fn get_weak_links(&mut self, new_round: Round) -> HashSet<NodeMetaData> {
        self.latest_nodes_metadata
            .iter_mut()
            .filter(|node_status| {
                node_status.is_some()
                    && node_status.as_ref().unwrap().not_linked()
                    && node_status.as_ref().unwrap().round() < new_round - 1
            })
            .map(|node_status| node_status.as_mut().unwrap().mark_linked().unwrap())
            .collect()
    }

    pub fn update_peer_latest_node(&mut self, node_meta_data: NodeMetaData, storage_diff: &mut Box<dyn DagStoreWriteBatch>) {
        let peer_index = self
            .address_to_validator_index
            .get(&node_meta_data.source())
            .expect("invalid peer_id node metadata");

        let need_to_update = match &self.latest_nodes_metadata.get(*peer_index).unwrap() {
            Some(status) => status.round() < node_meta_data.round(),
            None => true,
        };
        if need_to_update {
            info!(
                "DAG: updating peer latest node: my_id {}, round {} peer_index {}",
                self.my_id,
                node_meta_data.round(),
                *peer_index
            );
            let new_status = Some(PeerStatus::NotLinked(node_meta_data));
            *self.latest_nodes_metadata.get_mut(*peer_index).unwrap() = new_status.clone();
            let list_item = PeerStatusListItem {
                list_id: self.latest_nodes_metadata.id,
                index: *peer_index,
                content: new_status.clone(),
            };
            list_item.deep_save(storage_diff).unwrap();
        } else {
            info!("DAG: not updating peer latest node: my_id {},", self.my_id);
        }
    }

    pub fn update_with_strong_links(&mut self, round: Round, strong_links: Vec<PeerId>, storage_diff: &mut Box<dyn DagStoreWriteBatch>) {
        for peer_id in strong_links {
            let index = self.address_to_validator_index.get(&peer_id).unwrap();
            debug_assert!(self.latest_nodes_metadata.get(*index).unwrap().as_ref().unwrap().round() >= round);
            if self.latest_nodes_metadata.get(*index).unwrap().as_ref().unwrap().round() == round {
                debug_assert!(self.latest_nodes_metadata.get(*index).unwrap()
                    .as_ref()
                    .unwrap()
                    .not_linked());
                self.latest_nodes_metadata.get_mut(*index)
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .mark_linked();
                let list_item = PeerStatusListItem{
                    list_id: self.latest_nodes_metadata.id,
                    index: *index,
                    content: self.latest_nodes_metadata.get(*index).unwrap().clone(),
                };
                list_item.deep_save(storage_diff).unwrap();
            }
        }
    }
}
