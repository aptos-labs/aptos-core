module example_addr::multisig_managed_coin {
    use velor_framework::multisig_account;
    use velor_framework::object;
    use velor_framework::object::ObjectCore;
    use std::signer;
    use std::string::{Self, String};
    use example_addr::managed_fungible_asset;

    public entry fun initialize(
        creator: &signer,
        additional_owners: vector<address>,
        num_signature_required: u64,
        metadata_keys: vector<String>,
        metadata_values: vector<vector<u8>>,
        maximum_supply: u128,
        name: String,
        symbol: String,
        decimals: u8,
        icon_uri: String,
        project_uri: String,
        ref_flags: vector<bool>,
    ) {
        let multisig_address = multisig_account::get_next_multisig_account_address(signer::address_of(creator));
        // Customize those arguments as needed.
        multisig_account::create_with_owners(
            creator,
            additional_owners,
            num_signature_required,
            metadata_keys,
            metadata_values
        );

        // The ideal way is to get the multisig account signer but it is unavailable right now. So the pattern is to
        // create the metadata object by creator and transfer it to multisig account.
        let constructor_ref = &object::create_named_object(creator, *string::bytes(&symbol));
        object::transfer(creator, object::object_from_constructor_ref<ObjectCore>(constructor_ref), multisig_address);

        // Customize those arguments as needed.
        managed_fungible_asset::initialize(
            constructor_ref,
            maximum_supply,
            name,
            symbol,
            decimals,
            icon_uri,
            project_uri,
            ref_flags
        );
    }
}
