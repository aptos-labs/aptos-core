// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::common::{Author, Round};

pub trait AnchorElection {
    fn get_anchor(&self, round: Round) -> Author;

    fn commit(&mut self, round: Round);
}
