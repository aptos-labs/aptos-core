// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::arc_with_non_send_sync)]

use crate::{
    epoch_change::{EpochChangeProof, Verifier},
    epoch_state::EpochState,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    proof::TransactionAccumulatorSummary,
    state_proof::StateProof,
    transaction::Version,
    waypoint::Waypoint,
};
use anyhow::{bail, ensure, format_err, Result};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

/// `TrustedState` keeps track of light clients' latest, trusted view of the
/// ledger state. Light clients can use proofs from a state proof to "ratchet"
/// their view forward to a newer state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, CryptoHasher, BCSCryptoHash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum TrustedState {
    /// The current trusted state is an epoch waypoint, which is a commitment to
    /// an epoch change ledger info. Most light clients will start here when
    /// syncing for the first time.
    EpochWaypoint(Waypoint),
    /// The current trusted state is inside a verified epoch (which includes the
    /// validator set inside that epoch).
    EpochState {
        /// The current trusted version and a commitment to a ledger info inside
        /// the current trusted epoch.
        waypoint: Waypoint,
        /// The current epoch and validator set inside that epoch.
        epoch_state: EpochState,
    },
}

/// `TrustedStateChange` is the result of attempting to ratchet to a new trusted
/// state. In order to reduce redundant error checking, `TrustedStateChange` also
/// contains references to relevant items used to ratchet us.
#[derive(Clone, Debug)]
pub enum TrustedStateChange<'a> {
    /// We have a newer `TrustedState` but it's still in the same epoch, so only
    /// the latest trusted version changed.
    Version { new_state: TrustedState },
    /// We have a newer `TrustedState` and there was at least one epoch change,
    /// so we have a newer trusted version and a newer trusted validator set.
    Epoch {
        new_state: TrustedState,
        latest_epoch_change_li: &'a LedgerInfoWithSignatures,
    },
    /// The latest ledger info is at the same version as the trusted state and matches the hash.
    NoChange,
}

impl TrustedState {
    /// Create an initial trusted state from a trusted epoch waypoint constructed
    /// from an epoch-change ledger info.
    ///
    /// Note: we can't actually guarantee this waypoint is actually an epoch
    /// waypoint, but the sync will always fail to verify it's not.
    pub fn from_epoch_waypoint(epoch_waypoint: Waypoint) -> Self {
        Self::EpochWaypoint(epoch_waypoint)
    }

    /// Try to create a trusted state from an epoch-change ledger info and an
    /// accumulator summary at the same version.
    ///
    /// Fails if the ledger info is not actually an epoch-change ledger info or
    /// if the accumulator summary is not consistent with the ledger info.
    pub fn try_from_epoch_change_li(
        epoch_change_li: &LedgerInfo,
        accumulator: TransactionAccumulatorSummary,
    ) -> Result<Self> {
        // Ensure the accumulator and ledger info are at the same version/root hash.
        accumulator.verify_consistency(epoch_change_li)?;

        let epoch_state = epoch_change_li.next_epoch_state().cloned().ok_or_else(|| {
            format_err!("No EpochState in LedgerInfo; it must not be on an epoch boundary")
        })?;

        Ok(Self::EpochState {
            waypoint: Waypoint::new_epoch_boundary(epoch_change_li)?,
            epoch_state,
        })
    }

    pub fn is_epoch_waypoint(&self) -> bool {
        matches!(self, Self::EpochWaypoint(_))
    }

    pub fn version(&self) -> Version {
        self.waypoint().version()
    }

    pub fn waypoint(&self) -> Waypoint {
        match self {
            Self::EpochWaypoint(waypoint) => *waypoint,
            Self::EpochState { waypoint, .. } => *waypoint,
        }
    }

    /// Verify and ratchet forward our trusted state using an [`EpochChangeProof`]
    /// (that moves us into the latest epoch), a [`LedgerInfoWithSignatures`]
    /// inside that epoch, and an [`crate::proof::AccumulatorConsistencyProof`] from our current
    /// version to that last verifiable ledger info.
    ///
    /// If our current trusted state doesn't have an accumulator summary yet
    /// (for example, a client may be starting with an epoch waypoint), then an
    /// initial accumulator summary must be provided.
    ///
    /// For example, a client sends a `GetStateProof` request to an upstream
    /// FullNode and receives some epoch change proof along with a latest
    /// ledger info inside the `StateProof` response. This function
    /// verifies the change proof and ratchets the trusted state version forward
    /// if the response successfully moves us into a new epoch or a new latest
    /// ledger info within our current epoch.
    ///
    /// + If there was a validation error, e.g., the epoch change proof was
    ///   invalid, we return an `Err`.
    ///
    /// + If the message was well formed but stale (i.e., the returned latest
    ///   ledger is behind our trusted version), we also return an `Err` since
    ///   stale responses should always be rejected.
    ///
    /// + If the response is fresh and there is no epoch change, we just ratchet
    ///   our trusted version to the latest ledger info and return
    ///   `Ok(TrustedStateChange::Version { .. })`.
    ///
    /// + If there is a new epoch and the server provides a correct proof, we
    ///   ratchet our trusted version forward, update our verifier to contain
    ///   the new validator set, and return `Ok(TrustedStateChange::Epoch { .. })`.
    pub fn verify_and_ratchet<'a>(
        &self,
        state_proof: &'a StateProof,
    ) -> Result<TrustedStateChange<'a>> {
        self.verify_and_ratchet_inner(
            state_proof.latest_ledger_info_w_sigs(),
            state_proof.epoch_changes(),
        )
    }

    pub fn verify_and_ratchet_inner<'a>(
        &self,
        latest_li: &'a LedgerInfoWithSignatures,
        epoch_change_proof: &'a EpochChangeProof,
    ) -> Result<TrustedStateChange<'a>> {
        // Abort early if the response is stale.
        let curr_version = self.version();
        let target_version = latest_li.ledger_info().version();
        ensure!(
            target_version >= curr_version,
            "The target latest ledger info version is stale ({}) and behind our current trusted version ({})",
            target_version, curr_version,
        );

        if self.epoch_change_verification_required(latest_li.ledger_info().next_block_epoch()) {
            // Verify the EpochChangeProof to move us into the latest epoch.
            let epoch_change_li = epoch_change_proof.verify(self)?;
            let new_epoch_state = epoch_change_li
                .ledger_info()
                .next_epoch_state()
                .cloned()
                .ok_or_else(|| {
                    format_err!(
                        "A valid EpochChangeProof will never return a non-epoch change ledger info"
                    )
                })?;

            // If the latest ledger info is in the same epoch as the new verifier, verify it and
            // use it as latest state, otherwise fallback to the epoch change ledger info.
            let new_epoch = new_epoch_state.epoch;

            let verified_ledger_info = if epoch_change_li == latest_li {
                latest_li
            } else if latest_li.ledger_info().epoch() == new_epoch {
                new_epoch_state.verify(latest_li)?;
                latest_li
            } else if latest_li.ledger_info().epoch() > new_epoch && epoch_change_proof.more {
                epoch_change_li
            } else {
                bail!("Inconsistent epoch change proof and latest ledger info");
            };
            let new_waypoint = Waypoint::new_any(verified_ledger_info.ledger_info());

            let new_state = TrustedState::EpochState {
                waypoint: new_waypoint,
                epoch_state: new_epoch_state,
            };

            Ok(TrustedStateChange::Epoch {
                new_state,
                latest_epoch_change_li: epoch_change_li,
            })
        } else {
            let (curr_waypoint, curr_epoch_state) = match self {
                Self::EpochWaypoint(_) => {
                    bail!("EpochWaypoint can only verify an epoch change ledger info")
                },
                Self::EpochState {
                    waypoint,
                    epoch_state,
                    ..
                } => (waypoint, epoch_state),
            };

            // The EpochChangeProof is empty, stale, or only gets us into our
            // current epoch. We then try to verify that the latest ledger info
            // is inside this epoch.
            let new_waypoint = Waypoint::new_any(latest_li.ledger_info());
            if new_waypoint.version() == curr_waypoint.version() {
                ensure!(
                    &new_waypoint == curr_waypoint,
                    "LedgerInfo doesn't match verified state"
                );
                Ok(TrustedStateChange::NoChange)
            } else {
                // Verify the target ledger info, which should be inside the current epoch.
                curr_epoch_state.verify(latest_li)?;

                let new_state = Self::EpochState {
                    waypoint: new_waypoint,
                    epoch_state: curr_epoch_state.clone(),
                };

                Ok(TrustedStateChange::Version { new_state })
            }
        }
    }
}

impl Verifier for TrustedState {
    fn verify(&self, ledger_info: &LedgerInfoWithSignatures) -> Result<()> {
        match self {
            Self::EpochWaypoint(waypoint) => Verifier::verify(waypoint, ledger_info),
            Self::EpochState { epoch_state, .. } => Verifier::verify(epoch_state, ledger_info),
        }
    }

    fn epoch_change_verification_required(&self, epoch: u64) -> bool {
        match self {
            Self::EpochWaypoint(waypoint) => {
                Verifier::epoch_change_verification_required(waypoint, epoch)
            },
            Self::EpochState { epoch_state, .. } => {
                Verifier::epoch_change_verification_required(epoch_state, epoch)
            },
        }
    }

    fn is_ledger_info_stale(&self, ledger_info: &LedgerInfo) -> bool {
        match self {
            Self::EpochWaypoint(waypoint) => Verifier::is_ledger_info_stale(waypoint, ledger_info),
            Self::EpochState { epoch_state, .. } => {
                Verifier::is_ledger_info_stale(epoch_state, ledger_info)
            },
        }
    }
}

impl TrustedStateChange<'_> {
    pub fn new_state(self) -> Option<TrustedState> {
        match self {
            Self::Version { new_state } | Self::Epoch { new_state, .. } => Some(new_state),
            Self::NoChange => None,
        }
    }

    pub fn is_epoch_change(&self) -> bool {
        matches!(self, Self::Epoch { .. })
    }

    pub fn is_no_change(&self) -> bool {
        matches!(self, Self::NoChange)
    }
}
