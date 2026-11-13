// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native implementations for `aptos_experimental::native_order_book`: the
//! `NativeOrderContext` session extension and the write-staging natives. The
//! order value types live in `aptos-types` (`state_store::native_order`),
//! keyed by `TradingNativeKey::{Order, BulkOrder}`.

pub mod context;
pub mod natives;

pub use context::NativeOrderContext;
pub use natives::all_natives;
