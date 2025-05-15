use std::sync::Arc;

use async_trait::async_trait;

use rand::Rng;

use crate::{compute_res::ComputeRes, u256_define::{BlockId, TxnHash}, ExecError, ExecTxn, ExternalBlock, ExternalBlockMeta, ExternalPayloadAttr, VerifiedTxn, VerifiedTxnWithAccountSeqNum};