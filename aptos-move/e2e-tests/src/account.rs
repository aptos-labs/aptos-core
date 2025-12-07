// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Note: These types have been moved to crate `aptos-transactions-simulation`.
//       Reimporting for backward compatibility.
pub use aptos_transaction_simulation::{
    Account, AccountData, AccountPublicKey, CoinStore, FungibleStore, TransactionBuilder,
};
