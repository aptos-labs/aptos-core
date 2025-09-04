// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{mode::TConsensusMode, types::ObservedKeyLevelUpdateRequest};
use anyhow::Context;
use velor_logger::info;
use velor_types::jwks::{Issuer, KeyLevelUpdate, ProviderJWKs, QuorumCertifiedUpdate, KID};

pub struct PerKeyMode {}

impl TConsensusMode for PerKeyMode {
    type ConsensusSessionKey = (Issuer, KID);
    type ReliableBroadcastRequest = ObservedKeyLevelUpdateRequest;

    fn log_certify_start(epoch: u64, payload: &ProviderJWKs) {
        let KeyLevelUpdate {
            issuer,
            base_version,
            kid,
            ..
        } = KeyLevelUpdate::try_from_issuer_level_repr(payload)
            .unwrap_or_else(|_| KeyLevelUpdate::unknown());
        info!(
            epoch = epoch,
            issuer = String::from_utf8(issuer).ok(),
            kid = String::from_utf8(kid).ok(),
            base_version = base_version,
            "Start certifying key-level update."
        );
    }

    fn new_rb_request(
        epoch: u64,
        payload: &ProviderJWKs,
    ) -> anyhow::Result<ObservedKeyLevelUpdateRequest> {
        let KeyLevelUpdate { issuer, kid, .. } =
            KeyLevelUpdate::try_from_issuer_level_repr(payload)
                .context("new_rb_request failed with repr translation")?;
        Ok(ObservedKeyLevelUpdateRequest { epoch, issuer, kid })
    }

    fn log_certify_done(epoch: u64, qc: &QuorumCertifiedUpdate) {
        let KeyLevelUpdate {
            issuer,
            base_version,
            kid,
            ..
        } = KeyLevelUpdate::try_from_issuer_level_repr(&qc.update)
            .unwrap_or_else(|_| KeyLevelUpdate::unknown());
        info!(
            epoch = epoch,
            issuer = String::from_utf8(issuer).ok(),
            base_version = base_version,
            kid = String::from_utf8(kid).ok(),
            "Certified key-level update obtained."
        );
    }

    fn session_key_from_qc(qc: &QuorumCertifiedUpdate) -> anyhow::Result<(Issuer, KID)> {
        let KeyLevelUpdate { issuer, kid, .. } =
            KeyLevelUpdate::try_from_issuer_level_repr(&qc.update)
                .context("session_key_from_qc failed with repr translation")?;
        Ok((issuer, kid))
    }
}
