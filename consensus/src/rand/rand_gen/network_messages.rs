// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::rand::rand_gen::types::{
    AugData, AugDataSignature, AugmentedData, CertifiedAugData, CertifiedAugDataAck, Proof,
    RandShare, Share, ShareAck,
};
use aptos_enum_conversion_derive::EnumConversion;
use aptos_reliable_broadcast::RBMessage;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, EnumConversion)]
pub enum RandMessage<S, P, D> {
    Share(RandShare<S>),
    ShareAck(ShareAck<P>),
    AugData(AugData<D>),
    AugDataSignature(AugDataSignature),
    CertifiedAugData(CertifiedAugData<D>),
    CertifiedAugDataAck(CertifiedAugDataAck),
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> RBMessage for RandMessage<S, P, D> {}
