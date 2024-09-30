// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_storage_interface::state_delta::StateDelta;

pub struct SpeculativeState {
    state: StateDelta,
}
