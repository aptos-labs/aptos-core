// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Crate-wide flag for the native-trading subsystem (positions today;
//! orders and collateral land later). Gates whether
//! `AptosDB::open_internal` attaches the position DBs, runs the
//! native commit applier, and exposes the in-memory `UserPositions`.

/// Flip to `true` once order/collateral land.
pub(crate) const ENABLE_TRADING_NATIVE: bool = false;
