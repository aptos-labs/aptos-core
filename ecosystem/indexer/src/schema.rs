// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

table! {
    block_metadata_transactions (hash) {
        hash -> Varchar,
        id -> Varchar,
        round -> Int8,
        previous_block_votes -> Jsonb,
        proposer -> Varchar,
        timestamp -> Timestamp,
        inserted_at -> Timestamp,
        epoch -> Int8,
        previous_block_votes_bitmap -> Jsonb,
    }
}

table! {
    events (key, sequence_number) {
        transaction_hash -> Varchar,
        key -> Varchar,
        sequence_number -> Int8,
        #[sql_name = "type"]
        type_ -> Text,
        data -> Jsonb,
        inserted_at -> Timestamp,
    }
}

table! {
    processor_statuses (name, version) {
        name -> Varchar,
        version -> Int8,
        success -> Bool,
        details -> Nullable<Text>,
        last_updated -> Timestamp,
    }
}

table! {
    transactions (hash) {
        #[sql_name = "type"]
        type_ -> Varchar,
        payload -> Jsonb,
        version -> Int8,
        hash -> Varchar,
        state_root_hash -> Varchar,
        event_root_hash -> Varchar,
        gas_used -> Int8,
        success -> Bool,
        vm_status -> Text,
        accumulator_root_hash -> Varchar,
        inserted_at -> Timestamp,
    }
}

table! {
    user_transactions (hash) {
        hash -> Varchar,
        signature -> Jsonb,
        sender -> Varchar,
        sequence_number -> Int8,
        max_gas_amount -> Int8,
        expiration_timestamp_secs -> Timestamp,
        gas_unit_price -> Int8,
        timestamp -> Timestamp,
        inserted_at -> Timestamp,
    }
}

table! {
    write_set_changes (transaction_hash, hash) {
        transaction_hash -> Varchar,
        hash -> Varchar,
        #[sql_name = "type"]
        type_ -> Text,
        address -> Varchar,
        module -> Jsonb,
        resource -> Jsonb,
        data -> Jsonb,
        inserted_at -> Timestamp,
    }
}

allow_tables_to_appear_in_same_query!(
    block_metadata_transactions,
    events,
    processor_statuses,
    transactions,
    user_transactions,
    write_set_changes,
);
