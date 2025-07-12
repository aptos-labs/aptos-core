// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core_mempool::{CoreMempool, TimelineState},
    network::{BroadcastPeerPriority, MempoolSyncMsg},
    shared_mempool::{tasks, types::SharedMempool},
};
use aptos_config::{
    config::{NodeConfig, NodeType},
    network_id::NetworkId,
};
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

impl Arbitrary for BroadcastPeerPriority {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        prop_oneof![
            Just(BroadcastPeerPriority::Primary),
            Just(BroadcastPeerPriority::Failover),
        ]
        .boxed()
    }
}

pub fn mempool_incoming_transactions_strategy() -> impl Strategy<
    Value = (
        Vec<(
            SignedTransaction,
            Option<u64>,
            Option<BroadcastPeerPriority>,
        )>,
        TimelineState,
    ),
> {
    (
        proptest::collection::vec(
            any::<(
                SignedTransaction,
                Option<u64>,
                Option<BroadcastPeerPriority>,
            )>(),
            0..100,
        ),
        prop_oneof![
            Just(TimelineState::NotReady),
            Just(TimelineState::NonQualified)
        ],
    )
}

pub fn test_mempool_process_incoming_transactions_impl(
    txns: Vec<(
        SignedTransaction,
        Option<u64>,
        Option<BroadcastPeerPriority>,
    )>,
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
    let transaction_filter_config = config.transaction_filters.mempool_filter.clone();
    let smp: SharedMempool<NetworkClient<MempoolSyncMsg>, MockVMValidator> = SharedMempool::new(
        Arc::new(Mutex::new(CoreMempool::new(&config))),
        config.mempool.clone(),
        transaction_filter_config,
        network_client,
        Arc::new(mock_db),
        vm_validator,
        vec![],
        NodeType::extract_from_config(&config),
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
