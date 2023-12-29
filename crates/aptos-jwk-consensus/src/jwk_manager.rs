// Copyright © Aptos Foundation

use std::collections::HashMap;
use tokio::task::AbortHandle;
use aptos_types::jwks::{Issuer, ProviderJWKs};

pub enum ConsensusState {
    NotStarted,
    InProgress { abort_handle: AbortHandle },
    Finished,
}

pub struct PerProviderState {
    on_chain: ProviderJWKs,
    observed: ProviderJWKs,
    consensus_state: ConsensusState,
}

pub struct JWKManager {
    state: HashMap<Issuer, PerProviderState>,
}
