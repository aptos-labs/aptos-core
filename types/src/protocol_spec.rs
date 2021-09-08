// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::{TransactionInfo, TransactionInfoTrait};
#[cfg(any(test, feature = "fuzzing"))]
use proptest::arbitrary::Arbitrary;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub trait ProtocolSpec: Clone + Debug + Sync + Send {
    const NAME: &'static str;
    #[cfg(not(any(test, feature = "fuzzing")))]
    type TransactionInfo: TransactionInfoTrait;
    #[cfg(any(test, feature = "fuzzing"))]
    type TransactionInfo: TransactionInfoTrait + Arbitrary;
}

/// Default Protocol Spec
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct DpnProto;

impl ProtocolSpec for DpnProto {
    const NAME: &'static str = "Default Protocol";
    type TransactionInfo = TransactionInfo;
}
