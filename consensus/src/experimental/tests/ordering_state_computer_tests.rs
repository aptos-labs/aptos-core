// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use consensus_types::{block::Block, quorum_cert::QuorumCert};
use diem_types::validator_signer::ValidatorSigner;
use rand::Rng;

pub fn prepare_ordering_state_computer(_channel_size: usize) {
    // TODO
}

pub fn random_empty_block(signer: &ValidatorSigner, qc: QuorumCert) -> Block {
    let mut rng = rand::thread_rng();
    Block::new_proposal(vec![], rng.gen::<u64>(), rng.gen::<u64>(), qc, signer)
}

#[test]
fn test_ordering_state_computer() {
    // TODO: after changing the ordering state computer
}
