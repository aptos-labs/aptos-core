// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{mode::TConsensusMode, types::ObservedUpdateRequest};
use velor_logger::info;
use velor_types::jwks::{Issuer, ProviderJWKs, QuorumCertifiedUpdate};

pub struct PerIssuerMode {}

impl TConsensusMode for PerIssuerMode {
    type ConsensusSessionKey = Issuer;
    type ReliableBroadcastRequest = ObservedUpdateRequest;

    fn log_certify_start(epoch: u64, payload: &ProviderJWKs) {
        info!(
            epoch = epoch,
            issuer = String::from_utf8(payload.issuer.clone()).ok(),
            version = payload.version,
            "Start certifying update."
        );
    }

    fn new_rb_request(epoch: u64, payload: &ProviderJWKs) -> anyhow::Result<ObservedUpdateRequest> {
        Ok(ObservedUpdateRequest {
            epoch,
            issuer: payload.issuer.clone(),
        })
    }

    fn log_certify_done(epoch: u64, qc: &QuorumCertifiedUpdate) {
        info!(
            epoch = epoch,
            issuer = String::from_utf8(qc.update.issuer.clone()).ok(),
            version = qc.update.version,
            "Certified update obtained."
        );
    }

    fn session_key_from_qc(qc: &QuorumCertifiedUpdate) -> anyhow::Result<Issuer> {
        Ok(qc.update.issuer.clone())
    }
}
