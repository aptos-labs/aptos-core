// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native implementations for `aptos_experimental::native_collateral`: the
//! `NativeCollateralContext` session extension and the write-staging natives.
//! The collateral value type lives in `aptos-types`
//! (`state_store::native_collateral`), keyed by `TradingNativeKey::Collateral`.

pub mod context;
pub mod natives;

pub use context::NativeCollateralContext;
pub use natives::all_natives;
