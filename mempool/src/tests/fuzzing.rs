// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core_mempool::{CoreMempool, TimelineState},
    network::MempoolSyncMsg,
    shared_mempool::{tasks, types::SharedMempool},
};
use aptos_config::{config::NodeConfig, network_id::NetworkId};
use aptos_infallible::{Mutex, RwLock};
use aptos_network::{
    application::{interface::NetworkClient, storage::PeersAndMetadata},
    protocols::wire::handshake::v1::ProtocolId::MempoolDirectSend,
};
use aptos_storage_interface::mock::MockDbReaderWriter;
use aptos_types::transaction::SignedTransaction;
use aptos_vm_validator::mocks::mock_vm_validator::MockVMValidator;
use proptest::{
    arbitrary::any,
    prelude::*,
    strategy::{Just, Strategy},
};
use std::{collections::HashMap, sync::Arc};

pub fn mempool_incoming_transactions_strategy(
) -> impl Strategy<Value = (Vec<SignedTransaction>, TimelineState)> {
    (
        proptest::collection::vec(any::<SignedTransaction>(), 0..100),
        prop_oneof![
            Just(TimelineState::NotReady),
            Just(TimelineState::NonQualified)
        ],
    )
}

pub fn test_mempool_process_incoming_transactions_impl(
    txns: Vec<SignedTransaction>,
    timeline_state: TimelineState,
) {
    let config = NodeConfig::default();
    let mock_db = MockDbReaderWriter;
    let vm_validator = Arc::new(RwLock::new(MockVMValidator));
    let network_client = NetworkClient::new(
        vec![MempoolDirectSend],
        vec![],
        HashMap::new(),
        PeersAndMetadata::new(&[NetworkId::Validator]),
    );
    let smp: SharedMempool<NetworkClient<MempoolSyncMsg>, MockVMValidator> = SharedMempool::new(
        Arc::new(Mutex::new(CoreMempool::new(&config))),
        config.mempool.clone(),
        network_client,
        Arc::new(mock_db),
        vm_validator,
        vec![],
        config.base.role,
    );

    let _ = tasks::process_incoming_transactions(&smp, txns, timeline_state, false);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_mempool_process_incoming_transactions((txns, timeline_state) in mempool_incoming_transactions_strategy()) {
        test_mempool_process_incoming_transactions_impl(txns, timeline_state);
    }
}
