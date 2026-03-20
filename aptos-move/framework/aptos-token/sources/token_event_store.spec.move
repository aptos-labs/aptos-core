spec aptos_token::token_event_store {
    spec module {
        pragma verify = false;
        pragma aborts_if_is_strict;
    }

    spec schema TokenEventStoreAbortsIf {
        use aptos_framework::account::{Account};
        creator: &signer;
        let addr = signer::address_of(creator);
        let account = global<Account>(addr);
        aborts_if !exists<Account>(addr);
        aborts_if account.guid_creation_num + 9 >= account::MAX_GUID_CREATION_NUM;
        aborts_if account.guid_creation_num + 9 > MAX_U64;
    }

    spec emit_collection_uri_mutate_event(
        creator: &signer, collection: String, old_uri: String, new_uri: String
    ) {}

    spec emit_collection_description_mutate_event(
        creator: &signer,
        collection: String,
        old_description: String,
        new_description: String
    ) {}

    spec emit_collection_maximum_mutate_event(
        creator: &signer, collection: String, old_maximum: u64, new_maximum: u64
    ) {}

    spec emit_token_opt_in_event(account: &signer, opt_in: bool) {}

    spec emit_token_uri_mutate_event(
        creator: &signer,
        collection: String,
        token: String,
        old_uri: String,
        new_uri: String
    ) {}

    spec emit_default_property_mutate_event(
        creator: &signer,
        collection: String,
        token: String,
        keys: vector<String>,
        old_values: vector<Option<PropertyValue>>,
        new_values: vector<PropertyValue>
    ) {}

    spec emit_token_descrition_mutate_event(
        creator: &signer,
        collection: String,
        token: String,
        old_description: String,
        new_description: String
    ) {}

    spec emit_token_royalty_mutate_event(
        creator: &signer,
        collection: String,
        token: String,
        old_royalty_numerator: u64,
        old_royalty_denominator: u64,
        old_royalty_payee_addr: address,
        new_royalty_numerator: u64,
        new_royalty_denominator: u64,
        new_royalty_payee_addr: address
    ) {}

    spec emit_token_maximum_mutate_event(
        creator: &signer,
        collection: String,
        token: String,
        old_maximum: u64,
        new_maximum: u64
    ) {}
}
