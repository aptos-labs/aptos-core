// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native implementations for `aptos_trading::native_position`: the
//! `NativePositionContext` session extension and the write-staging
//! natives. The `NativePosition` value type lives in `aptos-types`
//! (`state_store::native_position`), keyed by `TradingNativeKey::Position`.

pub mod context;
pub mod natives;

pub use context::{NativePositionContext, PositionTxCache};
pub use natives::all_natives;
