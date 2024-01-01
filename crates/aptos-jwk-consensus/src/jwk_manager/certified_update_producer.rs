// Copyright © Aptos Foundation

use aptos_channels::aptos_channel;
use aptos_types::{
    epoch_state::EpochState,
    jwks::{ProviderJWKs, QuorumCertifiedUpdate},
};
use futures_util::future::AbortHandle;

/// A sub-process of the whole JWK consensus process.
/// Once invoked by `JWKConsensusManager` to `start_produce`,
/// it starts producing a `QuorumCertifiedUpdate` and returns an abort handle.
/// Once an `QuorumCertifiedUpdate` is available, it is sent back via a channel given earlier.
pub trait CertifiedUpdateProducer: Send + Sync {
    fn start_produce(
        &self,
        epoch_state: EpochState,
        payload: ProviderJWKs,
        agg_node_tx: Option<aptos_channel::Sender<(), QuorumCertifiedUpdate>>,
    ) -> AbortHandle;
}
