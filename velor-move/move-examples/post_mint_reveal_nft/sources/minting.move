// This module is an example of how to create a post-mint reveal NFT minting flow.
// In this example, we have two collections:
// - Source collection: the tokens in this collection will all have the same token_data_id. It will act as a certificate
// for users to later exchange this certificate token to a destination token.
// - Destination collection: this collection holds the art NFTs. Users can use their certificate token to exchange
// for an art NFT in the destination collection after reveal time. The exchanged destination token will be pseudo-randomized
// at the time of exchange, but its name will be based on the property version of the specified source certificate token.
//
// When minting starts, the user will mint a source certificate token from the source collection and pay for the certificate (
// you can think of it as paying for the right to exchange to a destination token later).
// When the reveal time starts, the user can then exchange their source certificate token to a destination token.
// The actual NFTs that have art/value are the destination tokens. The source token simply acts as a certificate
// to exchange for the destination token.
//
// Note that a post-mint reveal NFT flow can be done in many ways: we can lock the token in a locker that will only unlock after
// certain time, mutate the token uri, etc.
// We choose to implement the two-collection method as a tutorial because it's simple and hands-off (you don't have to manually update
// anything after setting up the collection).
//
// Now let's go over the flow using a 2-stage whitelist NFT project as an example:
// We will use an admin account to set up the collection. All admin functions are restricted to the deployer's account.
// ##### Setup #####
// 1. Publish this contract
// velor move publish --named-addresses post_mint_reveal_nft=[address of your admin_account]
// 2. Use the admin_account to set up the collection's treasury address (where the minting fee will go)
// set_treasury(&admin_account, signer::address_of(treasury_account));
// 3. Set up the source and destination collection configs using the admin account. Note that here we are setting up both
// the source and destination collection configs at the same time.
// set_collection_config_and_create_collection()
// The source token name will be "{source_token_name_base}: {counter}" and the destination token name will be "{destination_token_name_base}: {counter}".
// 4. Set up the public minting and reveal config
// set_public_minting_and_reveal_config()
// 5. (Optional, do this if your project has a whitelist)
// Set up the whitelist config in order. Because we're setting up a 2-stage whitelist project,
// we want to call set_up_whitelist_stage() and add_to_whitelist() two times - first time
// to set up the first stage, and the second time to set up the second stage.
// set_up_whitelist_stage(&admin_account, start_time, end_time, price, 0); // setting up the first stage with 0 being the stage index (stage is 0-indexed)
// add_to_whitelist(&admin_account, whitelist_address, mint_limit, 0); // adding whitelisted addresses to the first stage
// set_up_whitelist_stage(&admin_account, start_time, end_time, price, 1); // setting up the second stage with 1 being the stage index
// add_to_whitelist(&admin_account, whitelist_address, mint_limit, 1); // adding whitelisted addresses to the second stage
// Currently, there's no way to reset the whitelist config.
// 6. Add token uris for the destination collection
// add_tokens()
//
// ##### Minting #####
// 1. Mint a source certificate token from the source collection
// mint_source_certificate()
// 2. Exchange the source certificate token to a destination token
// exchange()
module post_mint_reveal_nft::minting {
    use velor_framework::account::{Self, SignerCapability, create_signer_with_capability};
    use velor_framework::velor_coin::VelorCoin;
    use velor_framework::coin;
    use velor_framework::event;
    use velor_framework::timestamp;
    use velor_token::token::{
        Self,
        TokenMutabilityConfig,
        create_token_mutability_config,
        create_collection,
        create_tokendata,
        TokenId
    };
    use post_mint_reveal_nft::big_vector::{Self, BigVector};
    use post_mint_reveal_nft::bucket_table::{Self, BucketTable};
    use post_mint_reveal_nft::whitelist;

    use std::bcs;
    use std::error;
    use std::signer;
    use std::string::{Self, String, utf8};
    use std::vector;

    /// NFTMintConfig stores relevant information and events of this module.
    struct NFTMintConfig has key {
        treasury: address,
        signer_cap: SignerCapability,
    }

    /// CollectionConfig stores information about the destination collection and token.
    struct CollectionConfig has key {
        destination_collection_name: String,
        destination_collection_description: String,
        destination_collection_maximum: u64,
        destination_collection_uri: String,
        destination_collection_mutate_config: vector<bool>,
        // this is the base name for the destination token. when exchanging from a certificate token to a destination token,
        // we will generate the destination token name as {destination_token_name_base} {counter}
        destination_token_name_base: String,
        royalty_payee_address: address,
        token_description: String,
        token_maximum: u64,
        token_mutate_config: TokenMutabilityConfig,
        royalty_points_den: u64,
        royalty_points_num: u64,
        tokens: BigVector<TokenAsset>,
        // Use a bucket table to check if there is any duplicate in tokens.
        // This is to prevent the same token from being added twice.
        // The `key` is the uri of the token, and the `value` is true (the value doesn't matter in this case,
        // since we're using a bucket_table as a hash set here).
        deduped_tokens: BucketTable<String, bool>,
        // The maximum amount of tokens a non-whitelisted address can mint. 0 indicates that there is no maximum and
        // any address can mint any amount of tokens within the limit of the collection maximum.
        public_mint_limit: u64,
    }

    struct TokenAsset has drop, store {
        token_uri: String,
        property_keys: vector<String>,
        property_values: vector<vector<u8>>,
        property_types: vector<String>,
    }

    /// PublicMintConfig stores information about public minting.
    struct PublicMintConfig has key {
        public_minting_addresses: BucketTable<address, u64>,
        public_mint_price: u64,
        public_minting_start_time: u64,
        public_minting_end_time: u64,
    }

    /// RevealConfig stores the reveal_time for the collection.
    /// After the reveal time, users can exchange their source
    /// certificate token to an NFT in the destination collection.
    struct RevealConfig has key {
        reveal_time: u64,
    }

    /// SourceToken stores the token_data_id of the source certificate token.
    struct SourceToken has key {
        source_collection_name: String,
        // this is the base name for the source token. when minting a source token, the name will be
        // {source_token_name} {counter}
        source_token_name_base: String,
        source_token_uri: String,
        source_collection_creator: address,
        source_token_counter: u64,
    }

    #[event]
    /// Emitted when a user mints a source certificate token.
    struct Minting has drop, store {
        token_receiver_address: address,
        token_id: TokenId,
    }

    #[event]
    /// Emitted when a user exchanges a source certificate token
    /// to a destination token.
    struct Exchange has drop, store {
        token_receiver_address: address,
        token_id: TokenId,
    }

    const BURNABLE_BY_OWNER: vector<u8> = b"TOKEN_BURNABLE_BY_OWNER";

    /// The account is not authorized to update the resources.
    const ENOT_AUTHORIZED: u64 = 1;
    /// The specified time is not valid.
    const EINVALID_TIME: u64 = 2;
    /// The whitelist account address does not exist.
    const EACCOUNT_DOES_NOT_EXIST: u64 = 3;
    /// The token_uri, property_keys, property_values, and property_types vectors have different lengths.
    const EVECTOR_LENGTH_UNMATCHED: u64 = 4;
    /// Adding new token uris exceeds the collection maximum.
    const EEXCEEDS_COLLECTION_MAXIMUM: u64 = 5;
    /// The user is not currently eligible for minting.
    const EUSER_IS_NOT_CURRENTLY_ELIGIBLE_TO_MINT: u64 = 6;
    /// No enough destination tokens left in the collection.
    const ENO_ENOUGH_TOKENS_LEFT: u64 = 7;
    /// Invalid numerator and denominator combo for the collection royalty setting.
    const EINVALID_ROYALTY_NUMERATOR_DENOMINATOR: u64 = 8;
    /// The config has not been initialized.
    const ECONFIG_NOT_INITIALIZED: u64 = 9;
    /// The specified amount exceeds the number of mints allowed for the specified whitelisted account.
    const EAMOUNT_EXCEEDS_MINTS_ALLOWED: u64 = 10;
    /// The source certificate id not found in the signer's account.
    const ETOKEN_ID_NOT_FOUND: u64 = 11;
    /// Can only exchange after the reveal starts.
    const ECANNOT_EXCHANGE_BEFORE_REVEAL_STARTS: u64 = 12;
    /// Can only add unique token uris.
    const EDUPLICATE_TOKEN_URI: u64 = 13;

    /// Initialize NFTMintConfig for this module.
    fun init_module(admin: &signer) {
        // Construct a seed vector that pseudo-randomizes the resource address generated.
        let seed_vec = bcs::to_bytes(&timestamp::now_seconds());
        let (_, resource_signer_cap) = account::create_resource_account(admin, seed_vec);

        move_to(admin, NFTMintConfig {
            // The initial admin account will be the source account (which created the resource account);
            // The source account can then update the admin account in NFTMintConfig struct by calling set_admin().
            treasury: @post_mint_reveal_nft,
            signer_cap: resource_signer_cap,
        });
    }

    /// Set the treasury account (where the payment for NFT goes to) of this module.
    public entry fun set_treasury(admin: &signer, new_treasury_address: address) acquires NFTMintConfig {
        assert!(signer::address_of(admin) == @post_mint_reveal_nft, error::permission_denied(ENOT_AUTHORIZED));
        assert!(account::exists_at(new_treasury_address), error::invalid_argument(EACCOUNT_DOES_NOT_EXIST));
        velor_account::assert_account_is_registered_for_apt(new_treasury_address);
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft);
        nft_mint_config.treasury = new_treasury_address;
    }

    /// Set up the source collection and destination collection.
    /// If we have already called this function before, calling it again
    /// will update the source and destination collection config.
    public entry fun set_collection_config_and_create_collection(
        admin: &signer,
        source_collection_name: String,
        source_token_name_base: String,
        source_token_uri: String,
        destination_collection_name: String,
        destination_collection_uri: String,
        destination_collection_maximum: u64,
        destination_collection_description: String,
        destination_token_name_base: String,
        destination_collection_mutate_config: vector<bool>,
        royalty_payee_address: address,
        token_description: String,
        token_maximum: u64,
        token_mutate_config: vector<bool>,
        royalty_points_den: u64,
        royalty_points_num: u64,
        public_mint_limit: u64,
    ) acquires NFTMintConfig, CollectionConfig, SourceToken {
        assert!(signer::address_of(admin) == @post_mint_reveal_nft, error::permission_denied(ENOT_AUTHORIZED));
        assert!(
            vector::length(&destination_collection_mutate_config) == 3 && vector::length(&token_mutate_config) == 5,
            error::invalid_argument(EVECTOR_LENGTH_UNMATCHED)
        );
        assert!(
            royalty_points_den > 0 && royalty_points_num < royalty_points_den,
            error::invalid_argument(EINVALID_ROYALTY_NUMERATOR_DENOMINATOR)
        );

        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft);
        let resource_signer = create_signer_with_capability(&nft_mint_config.signer_cap);

        // Create the destination collection that holds the unique art NFT.
        create_collection(
            &resource_signer,
            destination_collection_name,
            destination_collection_description,
            destination_collection_uri,
            destination_collection_maximum,
            destination_collection_mutate_config
        );

        // If CollectionConfig already exists, update it.
        if (exists<CollectionConfig>(@post_mint_reveal_nft)) {
            let collection_config = borrow_global_mut<CollectionConfig>(@post_mint_reveal_nft);
            collection_config.destination_collection_name = destination_collection_name;
            collection_config.destination_collection_description = destination_collection_description;
            collection_config.destination_collection_maximum = destination_collection_maximum;
            collection_config.destination_collection_uri = destination_collection_uri;
            collection_config.destination_collection_mutate_config = destination_collection_mutate_config;
            collection_config.destination_token_name_base = destination_token_name_base;
            collection_config.royalty_payee_address = royalty_payee_address;
            collection_config.token_description = token_description;
            collection_config.token_maximum = token_maximum;
            collection_config.token_mutate_config = create_token_mutability_config(&token_mutate_config);
            collection_config.royalty_points_den = royalty_points_den;
            collection_config.royalty_points_num = royalty_points_num;
            collection_config.public_mint_limit = public_mint_limit;
        } else {
            move_to(admin, CollectionConfig {
                destination_collection_name,
                destination_collection_description,
                destination_collection_maximum,
                destination_collection_uri,
                destination_collection_mutate_config,
                destination_token_name_base,
                royalty_payee_address,
                token_description,
                token_maximum,
                token_mutate_config: create_token_mutability_config(&token_mutate_config),
                royalty_points_den,
                royalty_points_num,
                tokens: big_vector::empty<TokenAsset>(128),
                deduped_tokens: bucket_table::new<String, bool>(128),
                public_mint_limit,
            });
        };

        if (exists<SourceToken>(@post_mint_reveal_nft)) {
            let source_token = borrow_global_mut<SourceToken>(@post_mint_reveal_nft);
            source_token.source_collection_name = source_collection_name;
            source_token.source_token_name_base = source_token_name_base;
            source_token.source_token_uri = source_token_uri;
        } else {
            move_to(admin, SourceToken {
                source_collection_name,
                source_token_name_base,
                source_token_uri,
                source_collection_creator: signer::address_of(&resource_signer),
                source_token_counter: 1,
            });
        };

        // Create the source certificate collection and token.
        create_collection(
            &resource_signer,
            source_collection_name,
            destination_collection_description,
            utf8(b""),
            destination_collection_maximum,
            destination_collection_mutate_config
        );
    }

    /// Set the minting and reveal config of this collection.
    /// If we have already called this function before, calling it again
    /// will update the minting and reveal time.
    public entry fun set_public_minting_and_reveal_config(
        admin: &signer,
        public_minting_start_time: u64,
        public_minting_end_time: u64,
        public_mint_price: u64,
        reveal_time: u64,
    ) acquires PublicMintConfig, RevealConfig {
        assert!(signer::address_of(admin) == @post_mint_reveal_nft, error::permission_denied(ENOT_AUTHORIZED));

        let now = timestamp::now_seconds();
        assert!(
            public_minting_start_time >= now && reveal_time >= public_minting_end_time,
            error::invalid_argument(EINVALID_TIME)
        );

        // If PublicMintConfig already exists, update it.
        if (exists<PublicMintConfig>(@post_mint_reveal_nft)) {
            let public_mint_config = borrow_global_mut<PublicMintConfig>(@post_mint_reveal_nft);
            public_mint_config.public_minting_start_time = public_minting_start_time;
            public_mint_config.public_minting_end_time = public_minting_end_time;
            public_mint_config.public_mint_price = public_mint_price;
        } else {
            move_to(admin, PublicMintConfig {
                // Can use a different size of bucket table depending on how big we expect the whitelist to be.
                // Here because a global public minting max is optional, we are starting with a smaller size
                // bucket table.
                public_minting_addresses: bucket_table::new<address, u64>(4),
                public_minting_start_time,
                public_minting_end_time,
                public_mint_price,
            });
        };

        if (exists<RevealConfig>(@post_mint_reveal_nft)) {
            let reveal_config = borrow_global_mut<RevealConfig>(@post_mint_reveal_nft);
            reveal_config.reveal_time = reveal_time;
        } else {
            move_to(admin, RevealConfig {
                reveal_time,
            });
        };
    }

    /// Set up different stages of the whitelist config in order.
    /// Most of the times, NFT projects will only have one-stage of whitelist.
    /// For example, if there are 3 stages of whitelist,
    /// we need to call this function 3 times with whitelist_stage being 0, 1, and 2 respectively.
    public entry fun set_up_whitelist_stage(
        admin: &signer,
        whitelist_start_time: u64,
        whitelist_end_time: u64,
        whitelist_price: u64,
        whitelist_stage: u64,
    ) {
        assert!(signer::address_of(admin) == @post_mint_reveal_nft, error::permission_denied(ENOT_AUTHORIZED));
        whitelist::add_or_update_whitelist_stage(
            admin,
            whitelist_start_time,
            whitelist_end_time,
            whitelist_price,
            whitelist_stage
        );
    }

    /// Add user addresses to the specified whitelist stage.
    /// Note that once we add an address to the whitelist, there's currently no way to remove it.
    public entry fun add_to_whitelist(
        admin: &signer,
        wl_addresses: vector<address>,
        mint_limit: u64,
        whitelist_stage: u64,
    ) {
        assert!(signer::address_of(admin) == @post_mint_reveal_nft, error::permission_denied(ENOT_AUTHORIZED));
        whitelist::add_whitelist_addresses(admin, wl_addresses, mint_limit, whitelist_stage);
    }

    /// Add destination tokens - the actual art tokens. Note that curently, there is no way to reset the list of destination tokens once it's added.
    /// The users will be able to exchange their source certificate token for a randomized destination token after the reveal time starts..
    public entry fun add_tokens(
        admin: &signer,
        token_uris: vector<String>,
        property_keys: vector<vector<String>>,
        property_values: vector<vector<vector<u8>>>,
        property_types: vector<vector<String>>
    ) acquires CollectionConfig {
        assert!(signer::address_of(admin) == @post_mint_reveal_nft, error::permission_denied(ENOT_AUTHORIZED));

        // cannot add more token uris if minting has already started
        assert!(exists<CollectionConfig>(@post_mint_reveal_nft), error::permission_denied(ECONFIG_NOT_INITIALIZED));
        assert!(
            vector::length(&token_uris) == vector::length(&property_keys) && vector::length(
                &property_keys
            ) == vector::length(&property_values) && vector::length(&property_values) == vector::length(
                &property_types
            ),
            error::invalid_argument(EVECTOR_LENGTH_UNMATCHED)
        );
        let collection_config = borrow_global_mut<CollectionConfig>(@post_mint_reveal_nft);

        assert!(
            vector::length(&token_uris) + big_vector::length(
                &collection_config.tokens
            ) <= collection_config.destination_collection_maximum || collection_config.destination_collection_maximum == 0,
            error::invalid_argument(EEXCEEDS_COLLECTION_MAXIMUM)
        );

        vector::enumerate_ref(&token_uris, |i, token_uri| {
            assert!(
                !bucket_table::contains(&collection_config.deduped_tokens, token_uri),
                error::invalid_argument(EDUPLICATE_TOKEN_URI)
            );
            big_vector::push_back(&mut collection_config.tokens, TokenAsset {
                token_uri: *token_uri,
                property_keys: *vector::borrow(&property_keys, i),
                property_values: *vector::borrow(&property_values, i),
                property_types: *vector::borrow(&property_types, i),
            });
            bucket_table::add(&mut collection_config.deduped_tokens, *token_uri, true);
        });
    }

    /// Mint source certificate.
    public entry fun mint_source_certificate(
        nft_claimer: &signer,
        amount: u64
    ) acquires NFTMintConfig, PublicMintConfig, SourceToken, CollectionConfig {
        assert!(
            exists<CollectionConfig>(@post_mint_reveal_nft) && exists<PublicMintConfig>(@post_mint_reveal_nft),
            error::permission_denied(ECONFIG_NOT_INITIALIZED)
        );

        let public_mint_config = borrow_global_mut<PublicMintConfig>(@post_mint_reveal_nft);
        let collection_config = borrow_global_mut<CollectionConfig>(@post_mint_reveal_nft);

        let now = timestamp::now_seconds();
        let price = public_mint_config.public_mint_price;
        let (whitelist_price, whitelist_stage, user_currently_eligible_for_whitelist_minting) = whitelist::is_user_currently_eligible_for_whitelisted_minting(
            @post_mint_reveal_nft,
            signer::address_of(nft_claimer)
        );
        let is_public_minting_time = now >= public_mint_config.public_minting_start_time && now < public_mint_config.public_minting_end_time;
        assert!(
            user_currently_eligible_for_whitelist_minting || is_public_minting_time,
            error::permission_denied(EUSER_IS_NOT_CURRENTLY_ELIGIBLE_TO_MINT)
        );

        let claimer_addr = signer::address_of(nft_claimer);

        // if this is the whitelist minting time
        if (user_currently_eligible_for_whitelist_minting) {
            price = whitelist_price;
            whitelist::deduct_user_minting_amount(@post_mint_reveal_nft, claimer_addr, whitelist_stage, amount);
        } else {
            if (collection_config.public_mint_limit != 0) {
                // If the claimer's address is not on the public_minting_addresses table yet, it means this is the
                // first time that this claimer mints. We will add the claimer's address and remaining amount of mints
                // to the public_minting_addresses table.
                if (!bucket_table::contains(&public_mint_config.public_minting_addresses, &claimer_addr)) {
                    bucket_table::add(
                        &mut public_mint_config.public_minting_addresses,
                        claimer_addr,
                        collection_config.public_mint_limit
                    );
                };
                let limit = bucket_table::borrow_mut(&mut public_mint_config.public_minting_addresses, claimer_addr);
                assert!(amount <= *limit, error::invalid_argument(EAMOUNT_EXCEEDS_MINTS_ALLOWED));
                *limit = *limit - amount;
            };
        };
        mint_source_certificate_internal(nft_claimer, price, amount);
    }

    // Exchange a source certificate token to a destination token. This function will burn the source certificate
    // and put a destination token in the nft_claimer's TokenStore.
    entry fun exchange(
        nft_claimer: &signer,
        source_token_name: String
    ) acquires NFTMintConfig, CollectionConfig, RevealConfig, SourceToken {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft);
        assert!(
            exists<CollectionConfig>(@post_mint_reveal_nft) && exists<RevealConfig>(@post_mint_reveal_nft),
            error::permission_denied(ECONFIG_NOT_INITIALIZED)
        );

        let reveal_config = borrow_global<RevealConfig>(@post_mint_reveal_nft);
        let now = timestamp::now_seconds();
        assert!(now > reveal_config.reveal_time, error::permission_denied(ECANNOT_EXCHANGE_BEFORE_REVEAL_STARTS));

        let collection_config = borrow_global_mut<CollectionConfig>(@post_mint_reveal_nft);
        let source_collection_creator = borrow_global<SourceToken>(@post_mint_reveal_nft).source_collection_creator;
        let source_collection_name = borrow_global<SourceToken>(@post_mint_reveal_nft).source_collection_name;
        let token_id = token::create_token_id_raw(
            source_collection_creator,
            source_collection_name,
            source_token_name,
            0
        );
        assert!(
            token::balance_of(signer::address_of(nft_claimer), token_id) == 1,
            error::invalid_argument(ETOKEN_ID_NOT_FOUND)
        );

        // Assert there's still some token uris in the vector.
        assert!(big_vector::length(&collection_config.tokens) > 0, error::permission_denied(ENO_ENOUGH_TOKENS_LEFT));

        // Randomize which token we're assigning to the user.
        let index = now % big_vector::length(&collection_config.tokens);
        let token = big_vector::swap_remove(&mut collection_config.tokens, index);
        bucket_table::remove(&mut collection_config.deduped_tokens, &token.token_uri);

        // The name of the destination token will be based on the property version of the source certificate token.
        let token_name = collection_config.destination_token_name_base;
        let index = string::index_of(&source_token_name, &utf8(b":"));
        string::append(
            &mut token_name,
            string::sub_string(&source_token_name, index, string::length(&source_token_name))
        );

        let resource_signer = create_signer_with_capability(&nft_mint_config.signer_cap);
        // Burn the source certificate token.
        token::burn(nft_claimer, source_collection_creator, source_collection_name, source_token_name, 0, 1);

        let token_data_id = create_tokendata(
            &resource_signer,
            collection_config.destination_collection_name,
            token_name,
            collection_config.token_description,
            collection_config.token_maximum,
            token.token_uri,
            collection_config.royalty_payee_address,
            collection_config.royalty_points_den,
            collection_config.royalty_points_num,
            collection_config.token_mutate_config,
            token.property_keys,
            token.property_values,
            token.property_types,
        );

        let token_id = token::mint_token(&resource_signer, token_data_id, 1);
        token::direct_transfer(&resource_signer, nft_claimer, token_id, 1);
        event::emit(Exchange { token_receiver_address: signer::address_of(nft_claimer), token_id, }
        );
    }

    /// Acquire resource signer if we later need it to do something.
    public fun acquire_resource_signer(
        admin: &signer
    ): signer acquires NFTMintConfig {
        assert!(signer::address_of(admin) == @post_mint_reveal_nft, error::permission_denied(ENOT_AUTHORIZED));
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft);
        create_signer_with_capability(&nft_mint_config.signer_cap)
    }

    // ======================================================================
    //   private helper functions //
    // ======================================================================

    fun mint_source_certificate_internal(
        nft_claimer: &signer,
        price: u64,
        amount: u64
    ) acquires NFTMintConfig, SourceToken, CollectionConfig {
        let receiver_addr = signer::address_of(nft_claimer);

        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft);
        let source_token = borrow_global_mut<SourceToken>(@post_mint_reveal_nft);
        let collection_config = borrow_global_mut<CollectionConfig>(@post_mint_reveal_nft);
        assert!(source_token.source_token_counter + amount <= big_vector::length(&collection_config.tokens) + 1,
            error::permission_denied(ENO_ENOUGH_TOKENS_LEFT));

        // pay for the source NFT
        coin::transfer<VelorCoin>(nft_claimer, nft_mint_config.treasury, price * amount);

        // mint token to the receiver
        let resource_signer = create_signer_with_capability(&nft_mint_config.signer_cap);

        while (amount > 0) {
            let token_name = source_token.source_token_name_base;
            string::append_utf8(&mut token_name, b": ");
            let num = u64_to_string(source_token.source_token_counter);
            string::append(&mut token_name, num);

            let token_data_id = create_tokendata(
                &resource_signer,
                source_token.source_collection_name,
                token_name,
                collection_config.token_description,
                collection_config.token_maximum,
                source_token.source_token_uri,
                collection_config.royalty_payee_address,
                collection_config.royalty_points_den,
                collection_config.royalty_points_num,
                collection_config.token_mutate_config,
                vector<String>[utf8(BURNABLE_BY_OWNER)],
                vector<vector<u8>>[bcs::to_bytes<bool>(&true)],
                vector<String>[utf8(b"bool")],
            );
            let token_id = token::mint_token(&resource_signer, token_data_id, 1);
            token::direct_transfer(&resource_signer, nft_claimer, token_id, 1);
            event::emit(Minting { token_receiver_address: receiver_addr, token_id, }
            );

            source_token.source_token_counter = source_token.source_token_counter + 1;
            amount = amount - 1;
        };
    }

    fun u64_to_string(value: u64): String {
        if (value == 0) {
            return utf8(b"0")
        };
        let buffer = vector::empty<u8>();
        while (value != 0) {
            vector::push_back(&mut buffer, ((48 + value % 10) as u8));
            value = value / 10;
        };
        vector::reverse(&mut buffer);
        utf8(buffer)
    }

    // ======================================================================
    //   unit tests //
    // ======================================================================
    #[test_only]
    use velor_framework::account::create_account_for_test;
    use velor_framework::velor_account;

    #[test_only]
    public fun set_up_test(
        admin_account: &signer,
        wl_nft_claimer: &signer,
        public_nft_claimer: &signer,
        treasury_account: &signer,
        velor_framework: &signer,
        timestamp: u64,
        collection_maximum: u64,
    ) acquires NFTMintConfig, CollectionConfig, SourceToken {
        // set up global time for testing purpose
        timestamp::set_time_has_started_for_testing(velor_framework);
        timestamp::update_global_time_for_test_secs(timestamp);

        create_account_for_test(signer::address_of(admin_account));
        init_module(admin_account);

        create_account_for_test(signer::address_of(wl_nft_claimer));
        create_account_for_test(signer::address_of(public_nft_claimer));
        create_account_for_test(signer::address_of(treasury_account));

        let (burn_cap, mint_cap) = velor_framework::velor_coin::initialize_for_test(velor_framework);
        coin::register<VelorCoin>(wl_nft_claimer);
        coin::register<VelorCoin>(public_nft_claimer);
        coin::register<VelorCoin>(treasury_account);
        coin::deposit(signer::address_of(wl_nft_claimer), coin::mint(100, &mint_cap));
        coin::deposit(signer::address_of(public_nft_claimer), coin::mint(100, &mint_cap));

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);

        let colleciton_setting = vector<bool>[false, false, false];
        let token_setting = vector<bool>[false, false, false, false, false];
        set_collection_config_and_create_collection(
            admin_account,
            utf8(b"source collection name"),
            utf8(b"source token base name"),
            utf8(b"source token uri"),
            utf8(b"test"),
            utf8(b"test collection uri"),
            collection_maximum,
            utf8(b"test collection description"),
            utf8(b"destination token name"),
            colleciton_setting,
            signer::address_of(treasury_account),
            utf8(b"token description"),
            0,
            token_setting,
            1,
            0,
            2,
        );

        set_treasury(admin_account, signer::address_of(treasury_account));
    }

    #[test_only]
    public entry fun set_up_token_uris(admin_account: &signer) acquires CollectionConfig {
        let token_uris = vector::empty<String>();
        let property_keys = vector::empty<vector<String>>();
        let property_values = vector::empty<vector<vector<u8>>>();
        let property_types = vector::empty<vector<String>>();
        let i = 0;
        while (i < 3) {
            let token_uri = utf8(b"token uri");
            string::append(&mut token_uri, u64_to_string(i));
            vector::push_back(&mut token_uris, token_uri);
            vector::push_back(&mut property_keys, vector::empty<String>());
            vector::push_back(&mut property_values, vector::empty<vector<u8>>());
            vector::push_back(&mut property_types, vector::empty<String>());
            i = i + 1;
        };
        add_tokens(admin_account, token_uris, property_keys, property_values, property_types);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    public entry fun test_happy_path(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, CollectionConfig, SourceToken, RevealConfig {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 0);

        set_public_minting_and_reveal_config(&admin_account, 201, 400, 10, 400);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        set_up_whitelist_stage(&admin_account, 50, 200, 5, 0);
        add_to_whitelist(&admin_account, wl_addresses, 2, 0);
        set_up_token_uris(&admin_account);

        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer, 2);

        timestamp::fast_forward_seconds(160);
        mint_source_certificate(&public_nft_claimer, 1);
        let public_mint_config = borrow_global_mut<PublicMintConfig>(@post_mint_reveal_nft);
        assert!(
            *bucket_table::borrow(
                &mut public_mint_config.public_minting_addresses,
                signer::address_of(&public_nft_claimer)
            ) == 1,
            1
        );

        // Assert that the source certificates exist in the nft claimers' TokenStore.
        let source_creator = borrow_global<SourceToken>(@post_mint_reveal_nft).source_collection_creator;
        let source_collection_name = borrow_global<SourceToken>(@post_mint_reveal_nft).source_collection_name;
        let token_id1 = token::create_token_id_raw(
            source_creator,
            source_collection_name,
            utf8(b"source token base name: 1"),
            0
        );
        let token_id2 = token::create_token_id_raw(
            source_creator,
            source_collection_name,
            utf8(b"source token base name: 2"),
            0
        );
        let token_id3 = token::create_token_id_raw(
            source_creator,
            source_collection_name,
            utf8(b"source token base name: 3"),
            0
        );
        assert!(token::balance_of(signer::address_of(&wl_nft_claimer), token_id1) == 1, 0);
        assert!(token::balance_of(signer::address_of(&wl_nft_claimer), token_id2) == 1, 1);
        assert!(token::balance_of(signer::address_of(&public_nft_claimer), token_id3) == 1, 2);
        assert!(coin::balance<VelorCoin>(signer::address_of(&treasury_account)) == 20, 1);
        assert!(coin::balance<VelorCoin>(signer::address_of(&wl_nft_claimer)) == 90, 2);
        assert!(coin::balance<VelorCoin>(signer::address_of(&public_nft_claimer)) == 90, 3);

        // Exchange to the destination NFT.
        timestamp::fast_forward_seconds(401);
        exchange(&public_nft_claimer, utf8(b"source token base name: 3"));
        exchange(&wl_nft_claimer, utf8(b"source token base name: 1"));
        exchange(&wl_nft_claimer, utf8(b"source token base name: 2"));

        // Assert that the exchange was successful.
        let collection_config = borrow_global<CollectionConfig>(@post_mint_reveal_nft);
        let exchanged_token_id1 = token::create_token_id_raw(
            source_creator,
            collection_config.destination_collection_name,
            utf8(b"destination token name: 1"),
            0
        );
        let exchanged_token_id2 = token::create_token_id_raw(
            source_creator,
            collection_config.destination_collection_name,
            utf8(b"destination token name: 2"),
            0
        );
        let exchanged_token_id3 = token::create_token_id_raw(
            source_creator,
            collection_config.destination_collection_name,
            utf8(b"destination token name: 3"),
            0
        );
        assert!(token::balance_of(signer::address_of(&wl_nft_claimer), exchanged_token_id1) == 1, 3);
        assert!(token::balance_of(signer::address_of(&wl_nft_claimer), exchanged_token_id2) == 1, 4);
        assert!(token::balance_of(signer::address_of(&public_nft_claimer), exchanged_token_id3) == 1, 5);
        assert!(big_vector::length(&collection_config.tokens) == 0, 6);
        assert!(bucket_table::length(&collection_config.deduped_tokens) == 0, 7);

        // Assert that we burned the source certificate.
        assert!(token::balance_of(signer::address_of(&wl_nft_claimer), token_id1) == 0, 8);
        assert!(token::balance_of(signer::address_of(&wl_nft_claimer), token_id2) == 0, 9);
        assert!(token::balance_of(signer::address_of(&public_nft_claimer), token_id3) == 0, 10);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x10005, location = Self)]
    public entry fun test_adding_token_uris_exceeds_collection_maximum(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, RevealConfig, CollectionConfig, SourceToken {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 2);
        set_public_minting_and_reveal_config(&admin_account, 201, 400, 10, 400);
        set_up_whitelist_stage(&admin_account, 50, 200, 5, 0);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2, 0);
        set_up_token_uris(&admin_account);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x5000c, location = Self)]
    public entry fun test_exchange_before_minting_ends(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, CollectionConfig, SourceToken, RevealConfig {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 0);
        set_public_minting_and_reveal_config(&admin_account, 201, 400, 10, 400);
        set_up_whitelist_stage(&admin_account, 50, 200, 5, 0);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2, 0);
        set_up_token_uris(&admin_account);

        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer, 2);

        timestamp::fast_forward_seconds(160);
        mint_source_certificate(&public_nft_claimer, 1);

        // Assert that the source certificates exist in the nft claimers' TokenStore.
        let source_token = borrow_global<SourceToken>(@post_mint_reveal_nft);
        let token_id1 = token::create_token_id_raw(
            source_token.source_collection_creator,
            source_token.source_collection_name,
            utf8(b"source token base name: 1"),
            0
        );
        assert!(token::balance_of(signer::address_of(&wl_nft_claimer), token_id1) == 1, 0);

        // Exchange to the destination NFT when the minting is ongoing.
        exchange(&wl_nft_claimer, utf8(b"source token base name: 1"));
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x50001, location = Self)]
    public entry fun invalid_set_treasury_address(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, CollectionConfig, SourceToken {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 0);
        set_treasury(&treasury_account, signer::address_of(&treasury_account));
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x10002, location = Self)]
    public entry fun invalid_set_minting_time_and_price(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, RevealConfig, CollectionConfig, SourceToken {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 0);
        set_public_minting_and_reveal_config(&admin_account, 150, 400, 10, 200);
        set_up_whitelist_stage(&admin_account, 50, 200, 5, 0);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x50009, location = Self)]
    public entry fun test_mint_before_set_up(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, SourceToken, CollectionConfig {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 0);
        mint_source_certificate(&wl_nft_claimer, 2);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x10004, location = post_mint_reveal_nft::whitelist)]
    public entry fun test_amount_exceeds_mint_allowed_whitelisted(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, CollectionConfig, SourceToken, RevealConfig {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 0);
        set_public_minting_and_reveal_config(&admin_account, 201, 400, 10, 400);
        set_up_whitelist_stage(&admin_account, 50, 200, 5, 0);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2, 0);
        set_up_token_uris(&admin_account);
        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer, 3);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x1000a, location = Self)]
    public entry fun test_amount_exceeds_mint_allowed_public(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, CollectionConfig, SourceToken, RevealConfig {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 0);
        set_public_minting_and_reveal_config(&admin_account, 201, 400, 10, 400);
        set_up_whitelist_stage(&admin_account, 50, 200, 5, 0);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2, 0);
        set_up_token_uris(&admin_account);
        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer, 2);
        timestamp::fast_forward_seconds(160);
        mint_source_certificate(&public_nft_claimer, 4);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x50007, location = Self)]
    public entry fun test_minting_source_certificate_exceeds_collection_maximum(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, RevealConfig, CollectionConfig, SourceToken {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 3);
        set_public_minting_and_reveal_config(&admin_account, 201, 400, 10, 400);
        set_up_whitelist_stage(&admin_account, 50, 200, 5, 0);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2, 0);
        set_up_token_uris(&admin_account);

        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer, 2);

        timestamp::fast_forward_seconds(160);
        mint_source_certificate(&public_nft_claimer, 2);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x50006, location = Self)]
    public entry fun test_account_not_on_whitelist(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, CollectionConfig, SourceToken, RevealConfig {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 0);
        set_public_minting_and_reveal_config(&admin_account, 201, 400, 10, 400);
        set_up_whitelist_stage(&admin_account, 50, 200, 5, 0);
        let wl_addresses = vector::empty<address>();
        add_to_whitelist(&admin_account, wl_addresses, 2, 0);
        set_up_token_uris(&admin_account);
        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&public_nft_claimer, 2);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    public entry fun test_update_public_minting_time(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, RevealConfig, CollectionConfig, SourceToken {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 0);
        set_public_minting_and_reveal_config(&admin_account, 201, 400, 10, 400);
        set_public_minting_and_reveal_config(&admin_account, 400, 600, 50, 600);

        let public_minting_config = borrow_global_mut<PublicMintConfig>(@post_mint_reveal_nft);
        assert!(public_minting_config.public_minting_start_time == 400, 3);
        assert!(public_minting_config.public_minting_end_time == 600, 4);
        assert!(public_minting_config.public_mint_price == 50, 5);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x10005, location = post_mint_reveal_nft::whitelist)]
    public entry fun invalid_add_to_whitelist(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, RevealConfig, CollectionConfig, SourceToken {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 0);
        set_public_minting_and_reveal_config(&admin_account, 201, 400, 10, 400);
        set_up_whitelist_stage(&admin_account, 50, 200, 5, 0);
        timestamp::fast_forward_seconds(200);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2, 0);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x50007, location = Self)]
    public entry fun test_all_tokens_minted(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, CollectionConfig, SourceToken, RevealConfig {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 0);
        set_public_minting_and_reveal_config(&admin_account, 201, 400, 10, 400);
        set_up_whitelist_stage(&admin_account, 50, 200, 5, 0);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2, 0);
        set_up_token_uris(&admin_account);

        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer, 2);

        timestamp::fast_forward_seconds(160);
        mint_source_certificate(&public_nft_claimer, 2);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x10004, location = Self)]
    public entry fun test_invalid_add_token_uri(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, CollectionConfig, SourceToken, RevealConfig {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 0);
        set_public_minting_and_reveal_config(&admin_account, 201, 400, 10, 400);
        set_up_whitelist_stage(&admin_account, 50, 200, 5, 0);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2, 0);
        let token_uris = vector::empty<String>();
        let property_keys = vector::empty<vector<String>>();
        let property_values = vector::empty<vector<vector<u8>>>();
        let property_types = vector::empty<vector<String>>();
        let i = 0;
        while (i < 3) {
            vector::push_back(&mut token_uris, utf8(b"token uri"));
            vector::push_back(&mut property_keys, vector::empty<String>());
            vector::push_back(&mut property_values, vector::empty<vector<u8>>());
            i = i + 1;
        };
        add_tokens(&admin_account, token_uris, property_keys, property_values, property_types);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    public entry fun test_acquire_signer(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, SourceToken, CollectionConfig {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 0);
        let resource_signer = acquire_resource_signer(&admin_account);
        let source_token = borrow_global<SourceToken>(@post_mint_reveal_nft);
        assert!(signer::address_of(&resource_signer) == source_token.source_collection_creator, 0);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        public_nft_claimer = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x1000d, location = Self)]
    public entry fun test_duplicate_token_uris(
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, RevealConfig, CollectionConfig, SourceToken {
        set_up_test(&admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &velor_framework, 10, 0);
        set_public_minting_and_reveal_config(&admin_account, 201, 400, 10, 400);
        set_up_whitelist_stage(&admin_account, 50, 200, 5, 0);
        set_up_token_uris(&admin_account);
        set_up_token_uris(&admin_account);
    }

    #[test_only]
    public entry fun set_up_multi_stage_whitelist_test(
        admin_account: signer,
        wl_nft_claimer: &signer,
        wl_nft_claimer2: &signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, RevealConfig, CollectionConfig, SourceToken {
        set_up_test(&admin_account, wl_nft_claimer, wl_nft_claimer2, &treasury_account, &velor_framework, 10, 0);
        // setting up public minting and reveal config
        set_public_minting_and_reveal_config(&admin_account, 201, 400, 10, 400);
        set_up_token_uris(&admin_account);

        // setting up stage-1 whitelist
        set_up_whitelist_stage(&admin_account, 50, 100, 5, 0);
        let wl_addresses_stage_1 = vector::empty<address>();
        vector::push_back(&mut wl_addresses_stage_1, signer::address_of(wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses_stage_1, 2, 0);

        // setting up stage-2 whitelist
        set_up_whitelist_stage(&admin_account, 101, 200, 6, 1);
        let wl_addresses_stage_2 = vector::empty<address>();
        vector::push_back(&mut wl_addresses_stage_2, signer::address_of(wl_nft_claimer2));
        add_to_whitelist(&admin_account, wl_addresses_stage_2, 1, 1);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        wl_nft_claimer2 = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    public entry fun test_multi_stage_whitelist_happy_path(
        admin_account: signer,
        wl_nft_claimer: signer,
        wl_nft_claimer2: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, RevealConfig, SourceToken, CollectionConfig {
        set_up_multi_stage_whitelist_test(
            admin_account,
            &wl_nft_claimer,
            &wl_nft_claimer2,
            treasury_account,
            velor_framework
        );
        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer, 2);
        let source_creator = borrow_global<SourceToken>(@post_mint_reveal_nft).source_collection_creator;
        let source_collection_name = borrow_global<SourceToken>(@post_mint_reveal_nft).source_collection_name;
        let token_id1 = token::create_token_id_raw(
            source_creator,
            source_collection_name,
            utf8(b"source token base name: 1"),
            0
        );
        let token_id2 = token::create_token_id_raw(
            source_creator,
            source_collection_name,
            utf8(b"source token base name: 2"),
            0
        );
        assert!(token::balance_of(signer::address_of(&wl_nft_claimer), token_id1) == 1, 0);
        assert!(token::balance_of(signer::address_of(&wl_nft_claimer), token_id2) == 1, 1);
        assert!(coin::balance<VelorCoin>(signer::address_of(&wl_nft_claimer)) == 90, 2);

        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer2, 1);
        let token_id3 = token::create_token_id_raw(
            source_creator,
            source_collection_name,
            utf8(b"source token base name: 3"),
            0
        );
        assert!(token::balance_of(signer::address_of(&wl_nft_claimer2), token_id3) == 1, 3);
        assert!(coin::balance<VelorCoin>(signer::address_of(&wl_nft_claimer2)) == 94, 4);
    }

    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        wl_nft_claimer2 = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x50006, location = Self)]
    public entry fun test_multi_stage_whitelist_account_not_on_whitelist(
        admin_account: signer,
        wl_nft_claimer: signer,
        wl_nft_claimer2: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, RevealConfig, CollectionConfig, SourceToken {
        set_up_multi_stage_whitelist_test(
            admin_account,
            &wl_nft_claimer,
            &wl_nft_claimer2,
            treasury_account,
            velor_framework
        );
        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer2, 2);
    }


    #[test (
        admin_account = @post_mint_reveal_nft,
        wl_nft_claimer = @0x123,
        wl_nft_claimer2 = @0x234,
        treasury_account = @0x345,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x10004, location = post_mint_reveal_nft::whitelist)]
    public entry fun test_multi_stage_whitelist_mint_amount_exceeds(
        admin_account: signer,
        wl_nft_claimer: signer,
        wl_nft_claimer2: signer,
        treasury_account: signer,
        velor_framework: signer,
    ) acquires NFTMintConfig, PublicMintConfig, RevealConfig, CollectionConfig, SourceToken {
        set_up_multi_stage_whitelist_test(
            admin_account,
            &wl_nft_claimer,
            &wl_nft_claimer2,
            treasury_account,
            velor_framework
        );
        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer, 2);
        mint_source_certificate(&wl_nft_claimer, 1);
    }
}
