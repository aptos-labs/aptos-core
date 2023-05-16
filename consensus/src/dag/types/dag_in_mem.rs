// Copyright © Aptos Foundation

use move_core_types::account_address::AccountAddress as PeerId;
use crate::dag::dag_storage::ItemId;
use crate::dag::types::dag_round_list::DagRoundList;
use crate::dag::types::week_link_creator::WeakLinksCreator;
use serde::{Deserialize, Serialize};
use crate::dag::types::missing_node_status_map::MissingNodeStatusMap;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagInMem_Key {
    pub(crate) my_id: PeerId,
    pub(crate) epoch: u64,
}

/// The part of the DAG data that should be persisted.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagInMem {
    pub(crate) my_id: PeerId,
    pub(crate) epoch: u64,
    pub(crate) current_round: u64,
    // starts from 0, which is genesys
    pub(crate) front: WeakLinksCreator,
    pub(crate) dag: DagRoundList,
    // TODO: protect from DDoS - currently validators can add unbounded number of entries
    pub(crate) missing_nodes: MissingNodeStatusMap,
}

impl DagInMem {
    pub(crate) fn get_dag(&self) -> &DagRoundList {
        &self.dag
    }

    pub(crate) fn get_dag_mut(&mut self) -> &mut DagRoundList {
        &mut self.dag
    }

    pub(crate) fn get_front(&self) -> &WeakLinksCreator {
        &self.front
    }

    pub(crate) fn get_front_mut(&mut self) -> &mut WeakLinksCreator {
        &mut self.front
    }

    pub(crate) fn get_missing_nodes(&self) -> &MissingNodeStatusMap {
        &self.missing_nodes
    }

    pub(crate) fn get_missing_nodes_mut(&mut self) -> &mut MissingNodeStatusMap {
        &mut self.missing_nodes
    }
}

/// The part of the DAG data that should be persisted.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagInMem_Brief {
    pub(crate) current_round: u64,
    pub(crate) front: ItemId,
    pub(crate) dag: ItemId,
    pub(crate) missing_nodes: ItemId,
}
