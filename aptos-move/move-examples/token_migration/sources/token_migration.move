/// This provides an example for migrating an NFT collection and its tokens from the token v1 standard to token v2.
/// The migrated tokens will contain the original token, so no tokens will be burnt.
///
/// WARNING: This example does not apply to fungible or semi-fungible token collections.
///
/// For more context on the token v1 standard, see https://aptos.dev/standards/aptos-token.
/// For token v2, see https://aptos.dev/standards/aptos-token-v2.
///
/// The migration flow is as follows:
/// 1. Create a new collection using the token v2 standard by calling `create_migrated_collection`.
/// Currently only the creator can call this function.
/// 2. Wrap each token from the original token v1 collection by calling `migrate_token`. The creator needs to own the
/// token to be migrated.
///
/// Note that this example uses AptosToken which is the convenient no code solution built on top of token v2. Developers
/// can use their custom implementation if desired.
module token_migration::token_migration {
    use aptos_framework::object::{Self, ExtendRef};
    use aptos_token::token as tokenv1;
    use aptos_token_objects::aptos_token as tokenv2;
    use aptos_token_objects::token as base_tokenv2;
    use aptos_token_objects::royalty as tokenv2_royalty;
    use std::option::{Self, Option};
    use std::string::String;
    use std::signer;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Storage state for managing the no-code Token.
    struct MigratedToken has key {
        /// Token to wrap
        token: tokenv1::Token,
        /// Used to burn.
        burn_ref: Option<base_tokenv2::BurnRef>,
        /// Used to control freeze.
        transfer_ref: Option<object::TransferRef>,
        /// Used to mutate fields
        mutator_ref: Option<base_tokenv2::MutatorRef>,
        /// Used to mutate fields
        extend_ref: Option<ExtendRef>,
    }

    /// Create a collection based on the previous collection
    public entry fun create_migrated_collection(creator: &signer, collection_name: String) {
        let creator_address = signer::address_of(creator);

        let collection_description = tokenv1::get_collection_description(creator_address, collection_name);
        let collection_uri = tokenv1::get_collection_uri(creator_address, collection_name);
        let max_supply = tokenv1::get_collection_maximum(creator_address, collection_name);
        let collection_mutability_config = tokenv1::get_collection_mutability_config(creator_address, collection_name);

        // For now, allow mutable everything, TODO: Make this configurable / use previous state
        tokenv2::create_collection(
            creator,
            collection_description,
            max_supply,
            collection_name,
            collection_uri,
            tokenv1::get_collection_mutability_description(&collection_mutability_config),
            true,
            tokenv1::get_collection_mutability_uri(&collection_mutability_config),
            // In v1, token mutability configs is set on the token instead of collection.
            // This example defaults to true here but the creator can deploy custom code built on token v2 to maintain
            // the same control setup as token v1 if desired.
            true, /* mutable_token_description */
            true, /* mutable_token_name */
            true, /* mutable_token_properties */
            true, /* mutable_token_uri */
            true, /* tokens_burnable_by_creator */
            true, /* tokens_freezable_by_creator */
            // Royalty is set on the tokens in v1, so we'll maintain the same setup during migration. This sets
            // the royalty on collections, so we'll set it to 0.
            0,
            1
        );
    }

    /// This can only called by the creator, who also needs to own the token being migrated.
    /// If creators want to let the token owner migrate their own tokens, they would need to deploy custom code.
    public fun migrate_token(creator: &signer, receiver: address, collection_name: String, token_name: String) {
        let creator_address = signer::address_of(creator);
        let original_token_data_id = tokenv1::create_token_data_id(creator_address, collection, token_name);
        let original_token_id = tokenv1::create_token_id(data_id, 0);
        let original_token_properties = token_v1::get_property_map(creator_address, original_token_id);
        let original_token = tokenv1::withdraw_token(creator, token_id, 1);

        let token_description = tokenv1::get_tokendata_description(original_token_data_id);
        let token_uri = tokenv1::get_tokendata_uri(creator_address, original_token_data_id);
        let token_constructor = tokenv2::mint(
            creator,
            collection_name,
            token_description,
            token_name,
            token_uri,
        );

        // Initialize royalty config on the token to respect the v1 royalty config.
        let original_royalty = tokenv1::get_tokendata_royalty(original_token_data_id);
        let numerator = tokenv1::get_royalty_numerator(&original_royalty);
        let denominator = tokenv1::get_royalty_denominator(&original_royalty);
        let payee = tokenv1::get_royalty_payee(&original_royalty);
        let royalty = tokenv2_royalty::create(numerator, denominator, payee);
        tokenv2_royalty::init(&token_constructor, royalty);

        let signer = object::generate_signer(&token_constructor);
        let migrated_token_resource = MigratedToken {
            token,
            burn_ref: option::some(burn_ref),
            transfer_ref: option::some(transfer_ref),
            mutator_ref: option::some(mutator_ref),
            extend_ref: option::some(extend_ref)
        };
        move_to(&signer, migrated_token_resource);
        let obj = object_from_constructor_ref<MigratedToken>(&token_constructor);
        transfer(creator, obj, receiver);
    }
}
