// Copyright © Aptos Foundation

use aptos_types::system_txn::SystemTransaction;
use std::sync::Arc;

pub trait SysTxnProvider: Send + Sync {
    fn get(&self) -> Option<Arc<SystemTransaction>>;
}
