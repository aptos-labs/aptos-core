// Copyright © Aptos Foundation

use aptos_types::system_txn::SystemTransaction;

pub trait SysTxnProvider: Send + Sync {
    fn get(&self) -> Option<SystemTransaction>;
}
