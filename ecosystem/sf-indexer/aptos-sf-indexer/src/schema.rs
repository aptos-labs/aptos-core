// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

table! {
    block_metadata_transactions (version) {
        version -> Int8,
        block_height -> Int8,
        id -> Varchar,
        round -> Int8,
        epoch -> Int8,
        previous_block_votes_bitvec -> Jsonb,
        proposer -> Varchar,
        failed_proposer_indices -> Jsonb,
        timestamp -> Timestamp,
        inserted_at -> Timestamp,
    }
}

table! {
    collection_datas (creator_address, collection_name, transaction_version) {
        creator_address -> Varchar,
        collection_name -> Text,
        description -> Text,
        transaction_version -> Int8,
        metadata_uri -> Text,
        supply -> Numeric,
        maximum -> Numeric,
        maximum_mutable -> Bool,
        uri_mutable -> Bool,
        description_mutable -> Bool,
        inserted_at -> Timestamp,
    }
}

table! {
    events (key, sequence_number) {
        key -> Varchar,
        sequence_number -> Int8,
        creation_number -> Int8,
        account_address -> Varchar,
        transaction_version -> Int8,
        transaction_block_height -> Int8,
        #[sql_name = "type"]
        type_ -> Text,
        data -> Jsonb,
        inserted_at -> Timestamp,
        type_str -> Text,
    }
}

table! {
    indexer_states (substream_module, block_height) {
        substream_module -> Varchar,
        block_height -> Int8,
        success -> Bool,
        details -> Nullable<Text>,
        last_updated -> Timestamp,
    }
}

table! {
    ledger_infos (chain_id) {
        chain_id -> Int8,
    }
}

table! {
    move_modules (transaction_version, write_set_change_index) {
        transaction_version -> Int8,
        write_set_change_index -> Int8,
        transaction_block_height -> Int8,
        name -> Varchar,
        address -> Varchar,
        bytecode -> Nullable<Bytea>,
        friends -> Nullable<Jsonb>,
        exposed_functions -> Nullable<Jsonb>,
        structs -> Nullable<Jsonb>,
        is_deleted -> Bool,
        inserted_at -> Timestamp,
    }
}

table! {
    move_resources (transaction_version, write_set_change_index) {
        transaction_version -> Int8,
        write_set_change_index -> Int8,
        transaction_block_height -> Int8,
        name -> Varchar,
        address -> Varchar,
        module -> Varchar,
        generic_type_params -> Nullable<Jsonb>,
        data -> Nullable<Jsonb>,
        is_deleted -> Bool,
        inserted_at -> Timestamp,
        type_str -> Text,
    }
}

table! {
    signatures (transaction_version, multi_agent_index, multi_sig_index, is_sender_primary) {
        transaction_version -> Int8,
        multi_agent_index -> Int8,
        multi_sig_index -> Int8,
        transaction_block_height -> Int8,
        signer -> Varchar,
        is_sender_primary -> Bool,
        #[sql_name = "type"]
        type_ -> Varchar,
        public_key -> Varchar,
        threshold -> Int8,
        public_key_indices -> Jsonb,
        inserted_at -> Timestamp,
    }
}

table! {
    table_items (transaction_version, write_set_change_index) {
        key -> Text,
        transaction_version -> Int8,
        write_set_change_index -> Int8,
        transaction_block_height -> Int8,
        table_handle -> Varchar,
        decoded_key -> Jsonb,
        decoded_value -> Nullable<Jsonb>,
        is_deleted -> Bool,
        inserted_at -> Timestamp,
    }
}

table! {
    table_metadatas (handle) {
        handle -> Varchar,
        key_type -> Text,
        value_type -> Text,
        inserted_at -> Timestamp,
    }
}

table! {
    token_datas (creator_address, collection_name, name, transaction_version) {
        creator_address -> Varchar,
        collection_name -> Text,
        name -> Text,
        transaction_version -> Int8,
        maximum -> Numeric,
        supply -> Numeric,
        largest_property_version -> Numeric,
        metadata_uri -> Text,
        royalty_points_numerator -> Numeric,
        royalty_points_denominator -> Numeric,
        maximum_mutable -> Bool,
        uri_mutable -> Bool,
        description_mutable -> Bool,
        properties_mutable -> Bool,
        royalty_mutable -> Bool,
        default_properties -> Jsonb,
        inserted_at -> Timestamp,
    }
}

table! {
    token_ownerships (creator_address, collection_name, name, property_version, transaction_version, table_handle) {
        creator_address -> Varchar,
        collection_name -> Text,
        name -> Text,
        property_version -> Numeric,
        transaction_version -> Int8,
        owner_address -> Nullable<Varchar>,
        amount -> Numeric,
        table_handle -> Varchar,
        table_type -> Nullable<Text>,
        inserted_at -> Timestamp,
    }
}

table! {
    tokens (creator_address, collection_name, name, property_version, transaction_version) {
        creator_address -> Varchar,
        collection_name -> Text,
        name -> Text,
        property_version -> Numeric,
        transaction_version -> Int8,
        token_properties -> Jsonb,
        inserted_at -> Timestamp,
    }
}

table! {
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

table! {
    user_transactions (version) {
        version -> Int8,
        block_height -> Int8,
        parent_signature_type -> Varchar,
        sender -> Varchar,
        sequence_number -> Int8,
        max_gas_amount -> Numeric,
        expiration_timestamp_secs -> Timestamp,
        gas_unit_price -> Numeric,
        timestamp -> Timestamp,
        inserted_at -> Timestamp,
        entry_function_id_str -> Text,
    }
}

table! {
    write_set_changes (transaction_version, index) {
        transaction_version -> Int8,
        index -> Int8,
        hash -> Varchar,
        transaction_block_height -> Int8,
        #[sql_name = "type"]
        type_ -> Text,
        address -> Varchar,
        inserted_at -> Timestamp,
    }
}

allow_tables_to_appear_in_same_query!(
    block_metadata_transactions,
    collection_datas,
    events,
    indexer_states,
    ledger_infos,
    move_modules,
    move_resources,
    signatures,
    table_items,
    table_metadatas,
    token_datas,
    token_ownerships,
    tokens,
    transactions,
    user_transactions,
    write_set_changes,
);
