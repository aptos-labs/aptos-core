// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::ledger_info::LedgerInfoWithSignatures;
use aptos_types::proof::TransactionInfoListWithProof;

pub struct ChunkProof {
    pub txn_infos_with_proof: TransactionInfoListWithProof,
    pub verified_target_li: LedgerInfoWithSignatures,
    pub epoch_change_li: Option<LedgerInfoWithSignatures>,
}
