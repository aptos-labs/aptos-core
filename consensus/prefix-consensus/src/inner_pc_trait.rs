// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Trait definition for an inner Prefix Consensus algorithm.
//!
//! The manager creates one instance per view and drives it by calling `start()`
//! then feeding incoming messages via `process_message()`. The implementation
//! handles round transitions, signature verification, and QC formation internally.

use crate::types::{PrefixConsensusInput, PrefixConsensusOutput};
use anyhow::Result;
use aptos_types::{
    account_address::AccountAddress,
    validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
};
use async_trait::async_trait;
use std::sync::Arc;

/// Type alias for party identity (matches the rest of the crate).
pub type Author = AccountAddress;

/// Trait for an inner Prefix Consensus algorithm used by managers.
///
/// The manager creates one instance per view and drives it by calling [`start()`]
/// then feeding incoming messages via [`process_message()`]. The implementation
/// handles round transitions, signature verification, and QC formation internally.
///
/// # Security Contract
///
/// The `author` parameter in [`process_message()`] is the **network-authenticated**
/// sender identity. The trait implementation **MUST** verify that the message's
/// claimed author matches this value (author mismatch check). This prevents
/// impersonation attacks where a Byzantine party claims to be someone else.
///
/// # Message Type
///
/// The associated `Message` type must currently be `PrefixConsensusMsg` for
/// compatibility with `StrongPrefixConsensusMsg::InnerPC { view, msg }`.
#[async_trait]
pub trait InnerPCAlgorithm: Send + Sync {
    /// The network message type this algorithm sends/receives.
    type Message: Clone + Send + Sync;

    /// Create a new instance for a view.
    fn new_for_view(
        input: PrefixConsensusInput,
        verifier: Arc<ValidatorVerifier>,
    ) -> Self
    where
        Self: Sized;

    /// Start the algorithm.
    ///
    /// Creates the initial vote, processes the self-vote, and cascades through
    /// any rounds that complete immediately (early QC pattern). Returns all
    /// outbound messages to broadcast (may be multiple if cascading occurs)
    /// and an optional completion output.
    async fn start(
        &mut self,
        signer: &ValidatorSigner,
    ) -> Result<(Vec<Self::Message>, Option<PrefixConsensusOutput>)>;

    /// Process an incoming message from a peer.
    ///
    /// The `author` parameter is the network-authenticated sender identity.
    /// The implementation **MUST** verify that the message's claimed author
    /// matches this value. It also performs signature verification, vote
    /// processing, and round transitions internally.
    ///
    /// Returns all outbound messages to broadcast (may be multiple if a QC
    /// triggers a round transition that itself produces a QC) and an optional
    /// completion output.
    ///
    /// Author mismatch and signature verification failures return
    /// `Ok((vec![], None))` (silently drop the bad message). Protocol-level
    /// errors from vote processing propagate as `Err`.
    async fn process_message(
        &mut self,
        author: Author,
        msg: Self::Message,
        signer: &ValidatorSigner,
    ) -> Result<(Vec<Self::Message>, Option<PrefixConsensusOutput>)>;
}
