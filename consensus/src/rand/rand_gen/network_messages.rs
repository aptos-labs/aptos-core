// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::rand::rand_gen::types::{
    AugData, AugDataSignature, AugmentedData, CertifiedAugData, CertifiedAugDataAck, Proof,
    RandShare, Share, ShareAck,
};
use aptos_reliable_broadcast::RBMessage;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum RandMessage<S, P, D> {
    Share(RandShare<S>),
    ShareAck(ShareAck<P>),
    AugData(AugData<D>),
    AugDataSignature(AugDataSignature),
    CertifiedAugData(CertifiedAugData<D>),
    CertifiedAugDataAck(CertifiedAugDataAck),
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> TryFrom<RandMessage<S, P, D>>
    for RandShare<S>
{
    type Error = anyhow::Error;

    fn try_from(value: RandMessage<S, P, D>) -> Result<Self, Self::Error> {
        match value {
            RandMessage::Share(value) => Ok(value),
            _ => Err(anyhow::anyhow!("Invalid RandMessages type")),
        }
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> TryFrom<RandMessage<S, P, D>>
    for ShareAck<P>
{
    type Error = anyhow::Error;

    fn try_from(value: RandMessage<S, P, D>) -> Result<Self, Self::Error> {
        match value {
            RandMessage::ShareAck(value) => Ok(value),
            _ => Err(anyhow::anyhow!("Invalid RandMessages type")),
        }
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> TryFrom<RandMessage<S, P, D>> for AugData<D> {
    type Error = anyhow::Error;

    fn try_from(value: RandMessage<S, P, D>) -> Result<Self, Self::Error> {
        match value {
            RandMessage::AugData(value) => Ok(value),
            _ => Err(anyhow::anyhow!("Invalid RandMessages type")),
        }
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> TryFrom<RandMessage<S, P, D>>
    for AugDataSignature
{
    type Error = anyhow::Error;

    fn try_from(value: RandMessage<S, P, D>) -> Result<Self, Self::Error> {
        match value {
            RandMessage::AugDataSignature(value) => Ok(value),
            _ => Err(anyhow::anyhow!("Invalid RandMessages type")),
        }
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> TryFrom<RandMessage<S, P, D>>
    for CertifiedAugData<D>
{
    type Error = anyhow::Error;

    fn try_from(value: RandMessage<S, P, D>) -> Result<Self, Self::Error> {
        match value {
            RandMessage::CertifiedAugData(value) => Ok(value),
            _ => Err(anyhow::anyhow!("Invalid RandMessages type")),
        }
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> TryFrom<RandMessage<S, P, D>>
    for CertifiedAugDataAck
{
    type Error = anyhow::Error;

    fn try_from(value: RandMessage<S, P, D>) -> Result<Self, Self::Error> {
        match value {
            RandMessage::CertifiedAugDataAck(value) => Ok(value),
            _ => Err(anyhow::anyhow!("Invalid RandMessages type")),
        }
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> From<RandShare<S>> for RandMessage<S, P, D> {
    fn from(value: RandShare<S>) -> Self {
        Self::Share(value)
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> From<ShareAck<P>> for RandMessage<S, P, D> {
    fn from(value: ShareAck<P>) -> Self {
        Self::ShareAck(value)
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> From<AugData<D>> for RandMessage<S, P, D> {
    fn from(value: AugData<D>) -> Self {
        Self::AugData(value)
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> From<AugDataSignature>
    for RandMessage<S, P, D>
{
    fn from(value: AugDataSignature) -> Self {
        Self::AugDataSignature(value)
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> From<CertifiedAugData<D>>
    for RandMessage<S, P, D>
{
    fn from(value: CertifiedAugData<D>) -> Self {
        Self::CertifiedAugData(value)
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> From<CertifiedAugDataAck>
    for RandMessage<S, P, D>
{
    fn from(value: CertifiedAugDataAck) -> Self {
        Self::CertifiedAugDataAck(value)
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> RBMessage for RandMessage<S, P, D> {}
