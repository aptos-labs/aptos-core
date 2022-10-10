// @generated automatically by Diesel CLI.

diesel::table! {
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

diesel::table! {
    collection_datas (creator_address, collection_name_hash, transaction_version) {
        creator_address -> Varchar,
        collection_name_hash -> Varchar,
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
        table_handle -> Varchar,
    }
}

diesel::table! {
    current_ans_lookup (domain, subdomain) {
        domain -> Varchar,
        subdomain -> Varchar,
        registered_address -> Nullable<Varchar>,
        expiration_timestamp -> Timestamp,
        last_transaction_version -> Int8,
        inserted_at -> Timestamp,
    }
}

diesel::table! {
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
        table_handle -> Varchar,
    }
}

diesel::table! {
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
        collection_data_id_hash -> Varchar,
    }
}

diesel::table! {
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
        collection_data_id_hash -> Varchar,
        table_type -> Text,
    }
}

diesel::table! {
    current_token_pending_claims (token_data_id_hash, property_version, from_address, to_address) {
        token_data_id_hash -> Varchar,
        property_version -> Numeric,
        from_address -> Varchar,
        to_address -> Varchar,
        collection_data_id_hash -> Varchar,
        creator_address -> Varchar,
        collection_name -> Varchar,
        name -> Varchar,
        amount -> Numeric,
        table_handle -> Varchar,
        last_transaction_version -> Int8,
        inserted_at -> Timestamp,
    }
}

diesel::table! {
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

diesel::table! {
    ledger_infos (chain_id) {
        chain_id -> Int8,
    }
}

diesel::table! {
    marketplace_bids (creator_address, collection_name) {
        creator_address -> Varchar,
        collection_name -> Text,
        token_name -> Text,
        property_version -> Int2,
        price -> Int8,
        maker -> Varchar,
        timestamp -> Timestamp,
    }
}

diesel::table! {
    marketplace_collections (creator_address, collection_name) {
        creator_address -> Varchar,
        collection_name -> Text,
        creation_timestamp -> Timestamp,
    }
}

diesel::table! {
    marketplace_offers (creator_address, collection_name) {
        creator_address -> Varchar,
        collection_name -> Text,
        token_name -> Text,
        property_version -> Int2,
        price -> Int8,
        seller -> Varchar,
        timestamp -> Timestamp,
    }
}

diesel::table! {
    marketplace_orders (creator_address, collection_name) {
        creator_address -> Varchar,
        collection_name -> Text,
        token_name -> Text,
        property_version -> Int2,
        price -> Int8,
        quantity -> Int8,
        maker -> Varchar,
        timestamp -> Timestamp,
    }
}

diesel::table! {
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

diesel::table! {
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

diesel::table! {
    processor_statuses (name, version) {
        name -> Varchar,
        version -> Int8,
        success -> Bool,
        details -> Nullable<Text>,
        last_updated -> Timestamp,
    }
}

diesel::table! {
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

diesel::table! {
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

diesel::table! {
    table_metadatas (handle) {
        handle -> Varchar,
        key_type -> Text,
        value_type -> Text,
        inserted_at -> Timestamp,
    }
}

diesel::table! {
    token_activities (transaction_version, event_account_address, event_creation_number, event_sequence_number) {
        transaction_version -> Int8,
        event_account_address -> Varchar,
        event_creation_number -> Int8,
        event_sequence_number -> Int8,
        collection_data_id_hash -> Varchar,
        token_data_id_hash -> Varchar,
        property_version -> Numeric,
        creator_address -> Varchar,
        collection_name -> Varchar,
        name -> Varchar,
        transfer_type -> Varchar,
        from_address -> Nullable<Varchar>,
        to_address -> Nullable<Varchar>,
        token_amount -> Numeric,
        coin_type -> Nullable<Text>,
        coin_amount -> Nullable<Numeric>,
        inserted_at -> Timestamp,
    }
}

diesel::table! {
    token_datas (creator_address, collection_name_hash, name_hash, transaction_version) {
        creator_address -> Varchar,
        collection_name_hash -> Varchar,
        name_hash -> Varchar,
        collection_name -> Text,
        name -> Text,
        transaction_version -> Int8,
        maximum -> Numeric,
        supply -> Numeric,
        largest_property_version -> Numeric,
        metadata_uri -> Text,
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
        collection_data_id_hash -> Varchar,
    }
}

diesel::table! {
    token_ownerships (creator_address, collection_name_hash, name_hash, property_version, transaction_version, table_handle) {
        creator_address -> Varchar,
        collection_name_hash -> Varchar,
        name_hash -> Varchar,
        collection_name -> Text,
        name -> Text,
        property_version -> Numeric,
        transaction_version -> Int8,
        owner_address -> Nullable<Varchar>,
        amount -> Numeric,
        table_handle -> Varchar,
        table_type -> Nullable<Text>,
        inserted_at -> Timestamp,
        collection_data_id_hash -> Varchar,
    }
}

diesel::table! {
    tokens (creator_address, collection_name_hash, name_hash, property_version, transaction_version) {
        creator_address -> Varchar,
        collection_name_hash -> Varchar,
        name_hash -> Varchar,
        collection_name -> Text,
        name -> Text,
        property_version -> Numeric,
        transaction_version -> Int8,
        token_properties -> Jsonb,
        inserted_at -> Timestamp,
        collection_data_id_hash -> Varchar,
    }
}

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

diesel::table! {
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

diesel::table! {
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

diesel::allow_tables_to_appear_in_same_query!(
    block_metadata_transactions,
    collection_datas,
    current_ans_lookup,
    current_collection_datas,
    current_token_datas,
    current_token_ownerships,
    current_token_pending_claims,
    events,
    ledger_infos,
    marketplace_bids,
    marketplace_collections,
    marketplace_offers,
    marketplace_orders,
    move_modules,
    move_resources,
    processor_statuses,
    signatures,
    table_items,
    table_metadatas,
    token_activities,
    token_datas,
    token_ownerships,
    tokens,
    transactions,
    user_transactions,
    write_set_changes,
);
