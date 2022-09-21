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
    collection_datas (collection_data_id_hash, transaction_version) {
        collection_data_id_hash -> Varchar,
        transaction_version -> Int8,
        creator_address -> Varchar,
        collection_name -> Varchar,
        description -> Text,
        metadata_uri -> Varchar,
        supply -> Numeric,
        maximum -> Numeric,
        maximum_mutable -> Bool,
        uri_mutable -> Bool,
        description_mutable -> Bool,
        inserted_at -> Timestamp,
    }
}

table! {
    current_collection_datas (collection_data_id_hash) {
        collection_data_id_hash -> Varchar,
        creator_address -> Varchar,
        collection_name -> Varchar,
        description -> Text,
        metadata_uri -> Varchar,
        supply -> Numeric,
        maximum -> Numeric,
        maximum_mutable -> Bool,
        uri_mutable -> Bool,
        description_mutable -> Bool,
        last_transaction_version -> Int8,
        inserted_at -> Timestamp,
    }
}

table! {
    current_token_datas (token_data_id_hash) {
        token_data_id_hash -> Varchar,
        creator_address -> Varchar,
        collection_name -> Varchar,
        name -> Varchar,
        maximum -> Numeric,
        supply -> Numeric,
        largest_property_version -> Numeric,
        metadata_uri -> Varchar,
        payee_address -> Varchar,
        royalty_points_numerator -> Numeric,
        royalty_points_denominator -> Numeric,
        maximum_mutable -> Bool,
        uri_mutable -> Bool,
        description_mutable -> Bool,
        properties_mutable -> Bool,
        royalty_mutable -> Bool,
        default_properties -> Jsonb,
        last_transaction_version -> Int8,
        inserted_at -> Timestamp,
    }
}

table! {
    current_token_ownerships (token_data_id_hash, property_version, owner_address) {
        token_data_id_hash -> Varchar,
        property_version -> Numeric,
        owner_address -> Varchar,
        creator_address -> Varchar,
        collection_name -> Varchar,
        name -> Varchar,
        amount -> Numeric,
        token_properties -> Jsonb,
        last_transaction_version -> Int8,
        inserted_at -> Timestamp,
    }
}

table! {
    events (account_address, creation_number, sequence_number) {
        sequence_number -> Int8,
        creation_number -> Int8,
        account_address -> Varchar,
        transaction_version -> Int8,
        transaction_block_height -> Int8,
        #[sql_name = "type"]
        type_ -> Text,
        data -> Jsonb,
        inserted_at -> Timestamp,
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
        name -> Text,
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
        name -> Text,
        address -> Varchar,
        #[sql_name = "type"]
        type_ -> Text,
        module -> Text,
        generic_type_params -> Nullable<Jsonb>,
        data -> Nullable<Jsonb>,
        is_deleted -> Bool,
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
        signature -> Varchar,
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
    token_datas (token_data_id_hash, transaction_version) {
        token_data_id_hash -> Varchar,
        transaction_version -> Int8,
        creator_address -> Varchar,
        collection_name -> Varchar,
        name -> Varchar,
        maximum -> Numeric,
        supply -> Numeric,
        largest_property_version -> Numeric,
        metadata_uri -> Varchar,
        payee_address -> Varchar,
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
    token_ownerships (token_data_id_hash, property_version, transaction_version, table_handle) {
        token_data_id_hash -> Varchar,
        property_version -> Numeric,
        transaction_version -> Int8,
        table_handle -> Varchar,
        creator_address -> Varchar,
        collection_name -> Varchar,
        name -> Varchar,
        owner_address -> Nullable<Varchar>,
        amount -> Numeric,
        table_type -> Nullable<Text>,
        inserted_at -> Timestamp,
    }
}

table! {
    tokens (token_data_id_hash, property_version, transaction_version) {
        token_data_id_hash -> Varchar,
        property_version -> Numeric,
        transaction_version -> Int8,
        creator_address -> Varchar,
        collection_name -> Varchar,
        name -> Varchar,
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
        entry_function_id_str -> Text,
        inserted_at -> Timestamp,
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
    current_collection_datas,
    current_token_datas,
    current_token_ownerships,
    events,
    ledger_infos,
    move_modules,
    move_resources,
    processor_statuses,
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
