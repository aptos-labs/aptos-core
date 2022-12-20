// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// @generated automatically by Diesel CLI.

diesel::table! {
    transactions (version) {
        version -> Int8,
        block_height -> Int8,
        hash -> Varchar,
        #[sql_name = "type"]
        type_ -> Varchar,
        payload -> Nullable<Jsonb>,
        state_change_hash -> Varchar,
        event_root_hash -> Varchar,
        state_checkpoint_hash -> Nullable<Varchar>,
        gas_used -> Numeric,
        success -> Bool,
        vm_status -> Text,
        accumulator_root_hash -> Varchar,
        num_events -> Int8,
        num_write_set_changes -> Int8,
        inserted_at -> Timestamp,
    }
}
