// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Gas parameters for the native position subsystem.
//!
//! All reads during block execution are served from the in-memory
//! `PositionInMemory` store, never RocksDB. Gas reflects in-memory cost +
//! serialization, not disk I/O. Params are gated on a sufficiently-new
//! gas-schedule version so older validators parse the schedule cleanly.

use crate::gas_schedule::NativeGasParameters;
use aptos_gas_algebra::{InternalGas, InternalGasPerByte};

crate::gas_schedule::macros::define_gas_parameters!(
    PositionGasParameters,
    "position",
    NativeGasParameters => .position,
    [
        // Base I/O — in-memory only during execution.
        [load_base: InternalGas, { 30.. => "load.base" }, 500],
        [load_per_byte: InternalGasPerByte, { 30.. => "load.per_byte" }, 50],
        [load_failure: InternalGas, { 30.. => "load.failure" }, 100],
        [write_base: InternalGas, { 30.. => "write.base" }, 800],
        [write_per_byte: InternalGasPerByte, { 30.. => "write.per_byte" }, 50],
        [delete_base: InternalGas, { 30.. => "delete.base" }, 800],

        // Lifecycle / capability.
        [register_base: InternalGas, { 30.. => "register.base" }, 20000],
        [unregister_base: InternalGas, { 30.. => "unregister.base" }, 2000],
        [deny_base: InternalGas, { 30.. => "deny.base" }, 5000],
        [update_ceiling_base: InternalGas, { 30.. => "update_ceiling.base" }, 5000],

        // Queries.
        [user_exists_base: InternalGas, { 30.. => "user_exists.base" }, 500],
        [has_position_base: InternalGas, { 30.. => "has_position.base" }, 500],
        [has_any_position_base: InternalGas, { 30.. => "has_any_position.base" }, 500],
        [get_position_base: InternalGas, { 30.. => "get_position.base" }, 800],
        [get_position_per_byte: InternalGasPerByte, { 30.. => "get_position.per_byte" }, 50],
        [get_position_info_base: InternalGas, { 30.. => "get_position_info.base" }, 500],
        [get_user_markets_base: InternalGas, { 30.. => "get_user_markets.base" }, 800],
        [get_user_markets_per_market: InternalGas, { 30.. => "get_user_markets.per_market" }, 100],
        [get_account_positions_base: InternalGas, { 30.. => "get_account_positions.base" }, 1000],
        [get_account_positions_per_position: InternalGas, { 30.. => "get_account_positions.per_position" }, 500],

        // Mutations.
        [create_position_base: InternalGas, { 30.. => "create_position.base" }, 5000],
        [create_position_per_byte: InternalGasPerByte, { 30.. => "create_position.per_byte" }, 50],
        [update_position_base: InternalGas, { 30.. => "update_position.base" }, 1500],
        [update_position_per_byte: InternalGasPerByte, { 30.. => "update_position.per_byte" }, 50],
        [remove_position_base: InternalGas, { 30.. => "remove_position.base" }, 3000],
        [remove_position_per_byte: InternalGasPerByte, { 30.. => "remove_position.per_byte" }, 50],

        // Computation natives.
        [compute_pnl_base: InternalGas, { 30.. => "compute_pnl.base" }, 1000],
        [compute_funding_cost_base: InternalGas, { 30.. => "compute_funding_cost.base" }, 1000],
        [compute_margin_required_base: InternalGas, { 30.. => "compute_margin_required.base" }, 1000],
        [apply_trade_base: InternalGas, { 30.. => "apply_trade.base" }, 2000],
        [compute_cross_margin_status_base: InternalGas, { 30.. => "compute_cross_margin_status.base" }, 2000],
        [compute_cross_margin_status_per_position: InternalGas, { 30.. => "compute_cross_margin_status.per_position" }, 800],

        // Storage fees (apply to Position only; UserMarkets is in-memory).
        [storage_slot_base: InternalGas, { 30.. => "storage_slot.base" }, 200000],
        [storage_byte_base: InternalGasPerByte, { 30.. => "storage_byte.base" }, 100],
    ]
);
