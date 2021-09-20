// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::{TransactionInfo, TransactionInfoTrait};

pub trait ProtocolSpec: Send + Sync {
    const NAME: &'static str;
    type TransactionInfo: TransactionInfoTrait;
}

/// Default Protocol Spec
#[derive(Clone, Debug)]
pub struct DpnProto;

impl ProtocolSpec for DpnProto {
    const NAME: &'static str = "Default Protocol";
    type TransactionInfo = TransactionInfo;
}
