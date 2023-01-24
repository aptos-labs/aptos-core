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
// The flow will be:
// 1. Publish this contract using a resource account
// 2. Set up the collection by calling `set_admin`, `set_treasury`, `set_collection_config_and_create_collection`, `
// set_minting_and_reveal_config`, `add_to_whitelist`, `add_tokens` functions.
// 3. When minting starts, call `mint_source_certificate` to mint a source certificate token.
// 4. When reveal starts, call `exchange` to exchange a source certificate token to a destination token.
// You can refer to the unit tests below `set_up_test` and `test_happy_path` as examples of the expected flow.
module post_mint_reveal_nft::minting {
    use std::error;
    use std::signer;
    use std::string::{Self, String, utf8};
    use std::vector;

    use aptos_framework::account::{Self, SignerCapability, create_signer_with_capability};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::resource_account;
    use aptos_framework::timestamp;
    use aptos_token::token::{Self, TokenMutabilityConfig, create_token_mutability_config, create_collection, create_tokendata, TokenId, TokenDataId};
    use post_mint_reveal_nft::bucket_table::{Self, BucketTable};
    use post_mint_reveal_nft::big_vector::{Self, BigVector};
    use std::bcs;


    /// NFTMintConfig stores relevant information and events of this module.
    struct NFTMintConfig has key {
        admin: address,
        treasury: address,
        signer_cap: SignerCapability,
        token_minting_events: EventHandle<MintingEvent>,
        token_exchange_events: EventHandle<ExchangeEvent>,
    }

    /// CollectionConfig stores information about the destination collection and token.
    struct CollectionConfig has key {
        collection_name: String,
        collection_description: String,
        collection_maximum: u64,
        collection_uri: String,
        collection_mutate_config: vector<bool>,
        // this is base name, when exchanging from a certificate token to a destination token,
        // we will generate the destination token name as `token_name_base: property version of the certificate tokens`
        token_name_base: String,
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

    /// WhitelistMintConfig stores information about whitelist minting.
    struct WhitelistMintConfig has key {
        whitelisted_address: BucketTable<address, u64>,
        whitelist_mint_price: u64,
        whitelist_minting_start_time: u64,
        whitelist_minting_end_time: u64,
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
        token_data_id: TokenDataId,
    }

    /// Emitted when a user mints a source certificate token.
    struct MintingEvent has drop, store {
        token_receiver_address: address,
        token_id: TokenId,
    }

    /// Emitted when a user exchanges a source certificate token
    /// to a destination token.
    struct ExchangeEvent has drop, store {
        token_receiver_address: address,
        token_id: TokenId,
    }

    const BURNABLE_BY_OWNER: vector<u8> = b"TOKEN_BURNABLE_BY_OWNER";
    const MAX_U64: u64 = 18446744073709551615;

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
    /// Whitelist mintint price cannot be more than the public minting price.
    const EINVALID_PRICE: u64 = 6;
    /// Cannot update the collection after minting starts.
    const EINVALID_UPDATE_AFTER_MINTING: u64 = 7;
    /// Minting hasn't started yet.
    const EMINTING_IS_NOT_ENABLED: u64 = 8;
    /// No enough destination tokens left in the collection.
    const ENO_ENOUGH_TOKENS_LEFT: u64 = 9;
    /// The account trying to mint during the whitelist minting time is not whitelisted.
    const EACCOUNT_NOT_WHITELISTED: u64 = 10;
    /// Invalid numerator and denominator combo for the collection royalty setting.
    const EINVALID_ROYALTY_NUMERATOR_DENOMINATOR: u64 = 11;
    /// The collection is already created.
    const ECOLLECTION_ALREADY_CREATED: u64 = 12;
    /// The config has not been initialized.
    const ECONFIG_NOT_INITIALIZED: u64 = 13;
    /// The specified amount exceeds the number of mints allowed for the specified whitelisted account.
    const EAMOUNT_EXCEEDS_MINTS_ALLOWED: u64 = 14;
    /// The source certificate id not found in the signer's account.
    const ETOKEN_ID_NOT_FOUND: u64 = 15;
    /// Can only exchange after the reveal starts.
    const ECANNOT_EXCHANGE_BEFORE_REVEAL_SRARTS: u64 = 16;
    /// Can only add unique token uris.
    const EDUPLICATE_TOKEN_URI: u64 = 17;

    /// Initialize NFTMintConfig for this module.
    fun init_module(post_mint_reveal_nft_resource_account: &signer) {
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(post_mint_reveal_nft_resource_account, @source_addr);
        move_to(post_mint_reveal_nft_resource_account, NFTMintConfig {
            // The initial admin account will be the source account (which created the resource account);
            // The source account can then update the admin account in NFTMintConfig struct by calling set_admin().
            admin: @source_addr,
            treasury: @source_addr,
            signer_cap: resource_signer_cap,
            token_minting_events: account::new_event_handle<MintingEvent>(post_mint_reveal_nft_resource_account),
            token_exchange_events: account::new_event_handle<ExchangeEvent>(post_mint_reveal_nft_resource_account),
        });
    }

    /// Set admin of this module.
    public entry fun set_admin(admin: &signer, new_admin_address: address) acquires NFTMintConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));
        nft_mint_config.admin = new_admin_address;
    }

    /// Set the treasury account (where the payment for NFT goes to) of this module.
    public entry fun set_treasury(admin: &signer, new_treasury_address: address) acquires NFTMintConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));
        nft_mint_config.treasury = new_treasury_address;
    }

    /// Set up and create the destination collection.
    public entry fun set_collection_config_and_create_collection(
        admin: &signer,
        collection_name: String,
        collection_description: String,
        collection_maximum: u64,
        collection_uri: String,
        collection_mutate_config: vector<bool>,
        token_name_base: String,
        royalty_payee_address: address,
        token_description: String,
        token_maximum: u64,
        token_mutate_config: vector<bool>,
        royalty_points_den: u64,
        royalty_points_num: u64,
        public_mint_limit: u64,
    ) acquires NFTMintConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));

        assert!(vector::length(&collection_mutate_config) == 3 && vector::length(&token_mutate_config) == 5, error::invalid_argument(EVECTOR_LENGTH_UNMATCHED));
        assert!(royalty_points_den > 0 && royalty_points_num < royalty_points_den, error::invalid_argument(EINVALID_ROYALTY_NUMERATOR_DENOMINATOR));
        assert!(!exists<CollectionConfig>(@post_mint_reveal_nft), error::permission_denied(ECOLLECTION_ALREADY_CREATED));

        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft);
        let resource_signer = create_signer_with_capability(&nft_mint_config.signer_cap);

        // Create the destination collection that holds the unique art NFT.
        create_collection(&resource_signer, collection_name, collection_description, collection_uri, collection_maximum, collection_mutate_config);

        move_to(&resource_signer, CollectionConfig {
            collection_name,
            collection_description,
            collection_maximum,
            collection_uri,
            collection_mutate_config,
            token_name_base,
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

        // Create the source certificate collection and token.
        let source_collection_name = collection_name;
        string::append_utf8(&mut source_collection_name, b" - source collection");
        create_collection(&resource_signer, source_collection_name, collection_description, utf8(b""), collection_maximum, collection_mutate_config);

        let source_token_data_id = create_tokendata(
            &resource_signer,
            source_collection_name,
            // token name
            utf8(b"source token"),
            // token description
            utf8(b"source token"),
            // token maximum
            collection_maximum,
            // token uri
            utf8(b""),
            signer::address_of(&resource_signer),
            1,
            0,
            create_token_mutability_config(
                &vector<bool>[ false, false, false, false, true ]
            ),
            vector<String>[string::utf8(BURNABLE_BY_OWNER)],
            vector<vector<u8>>[bcs::to_bytes<bool>(&true)],
            vector<String>[string::utf8(b"bool")],
        );

        move_to(&resource_signer, SourceToken { token_data_id: source_token_data_id });
    }

    /// Set the minting and reveal config of this collection.
    public entry fun set_minting_and_reveal_config(
        admin: &signer,
        whitelist_minting_start_time: u64,
        whitelist_minting_end_time: u64,
        whitelist_mint_price: u64,
        public_minting_start_time: u64,
        public_minting_end_time: u64,
        public_mint_price: u64,
        reveal_time: u64,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, RevealConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));

        let now = timestamp::now_seconds();
        // assert that we are setting the whitelist time to sometime in the future
        assert!(whitelist_minting_start_time > now && whitelist_minting_start_time < whitelist_minting_end_time, error::invalid_argument(EINVALID_TIME));
        // assert that the public minting starts after the whitelist minting ends
        assert!(public_minting_start_time > whitelist_minting_end_time && public_minting_start_time < public_minting_end_time, error::invalid_argument(EINVALID_TIME));
        // assert that the public minting price is equal or more expensive than the whitelist minting price
        assert!(public_mint_price >= whitelist_mint_price, error::invalid_argument(EINVALID_PRICE));
        assert!(reveal_time >= public_minting_end_time, error::invalid_argument(EINVALID_TIME));

        if (exists<WhitelistMintConfig>(@post_mint_reveal_nft)) {
            let whitelist_mint_config = borrow_global_mut<WhitelistMintConfig>(@post_mint_reveal_nft);
            whitelist_mint_config.whitelist_minting_start_time = whitelist_minting_start_time;
            whitelist_mint_config.whitelist_minting_end_time = whitelist_minting_end_time;
            whitelist_mint_config.whitelist_mint_price = whitelist_mint_price;
        } else {
            let resource_account = create_signer_with_capability(&nft_mint_config.signer_cap);
            move_to(&resource_account, WhitelistMintConfig {
                // Can use a different size of bucket table depending on how big we expect the whitelist to be.
                whitelisted_address: bucket_table::new<address, u64>(4),
                whitelist_minting_start_time,
                whitelist_minting_end_time,
                whitelist_mint_price,
            });
        };

        if (exists<PublicMintConfig>(@post_mint_reveal_nft)) {
            let public_mint_config = borrow_global_mut<PublicMintConfig>(@post_mint_reveal_nft);
            public_mint_config.public_minting_start_time = public_minting_start_time;
            public_mint_config.public_minting_end_time = public_minting_end_time;
            public_mint_config.public_mint_price = public_mint_price;
        } else {
            let resource_account = create_signer_with_capability(&nft_mint_config.signer_cap);
            move_to(&resource_account, PublicMintConfig {
                // Can use a different size of bucket table depending on how big we expect the whitelist to be.
                // Here because a global pubic minting max is optional, we are starting with a smaller size
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
            let resource_account = create_signer_with_capability(&nft_mint_config.signer_cap);
            move_to(&resource_account, RevealConfig {
                reveal_time,
            });
        };
    }

    /// Add user addresses to the whitelist.
    public entry fun add_to_whitelist(
        admin: &signer,
        wl_addresses: vector<address>,
        mint_limit: u64
    ) acquires NFTMintConfig, WhitelistMintConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));
        assert!(exists<WhitelistMintConfig>(@post_mint_reveal_nft), error::permission_denied(ECONFIG_NOT_INITIALIZED));
        // cannot update whitelisted addresses if the whitelist minting period has already passed
        let whitelist_mint_config = borrow_global_mut<WhitelistMintConfig>(@post_mint_reveal_nft);
        assert!(whitelist_mint_config.whitelist_minting_end_time > timestamp::now_seconds(), error::permission_denied(EINVALID_UPDATE_AFTER_MINTING));

        let i = 0;
        while (i < vector::length(&wl_addresses)) {
            let addr = *vector::borrow(&wl_addresses, i);
            // assert that the specified address exists
            assert!(account::exists_at(addr), error::invalid_argument(EACCOUNT_DOES_NOT_EXIST));
            bucket_table::add(&mut whitelist_mint_config.whitelisted_address, addr, mint_limit);
            i = i + 1;
        };
    }

    /// Add destination tokens, which are the actual art tokens. The users will be able to exchange their source certificate token
    /// for a randomized destination token after the reveal time starts.
    public entry fun add_tokens(
        admin: &signer,
        token_uris: vector<String>,
        property_keys: vector<vector<String>>,
        property_values: vector<vector<vector<u8>>>,
        property_types: vector<vector<String>>
    ) acquires NFTMintConfig, CollectionConfig, WhitelistMintConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));

        // cannot add more token uris if minting has already started
        assert!(exists<WhitelistMintConfig>(@post_mint_reveal_nft) && exists<PublicMintConfig>(@post_mint_reveal_nft), error::permission_denied(ECONFIG_NOT_INITIALIZED));
        let whitelist_mint_config = borrow_global_mut<WhitelistMintConfig>(@post_mint_reveal_nft);
        assert!(whitelist_mint_config.whitelist_minting_start_time > timestamp::now_seconds(), error::permission_denied(EINVALID_UPDATE_AFTER_MINTING));

        assert!(exists<CollectionConfig>(@post_mint_reveal_nft), error::permission_denied(ECONFIG_NOT_INITIALIZED));
        assert!(vector::length(&token_uris) == vector::length(&property_keys) && vector::length(&property_keys) == vector::length(&property_values) && vector::length(&property_values) == vector::length(&property_types), error::invalid_argument(EVECTOR_LENGTH_UNMATCHED));
        let collection_config = borrow_global_mut<CollectionConfig>(@post_mint_reveal_nft);

        assert!(vector::length(&token_uris) + big_vector::length(&collection_config.tokens) <= collection_config.collection_maximum || collection_config.collection_maximum == 0, error::invalid_argument(EEXCEEDS_COLLECTION_MAXIMUM));

        let i = 0;
        while (i < vector::length(&token_uris)) {
            let token_uri = vector::borrow(&token_uris, i);
            assert!(!bucket_table::contains(&collection_config.deduped_tokens, token_uri), error::invalid_argument(EDUPLICATE_TOKEN_URI));
            big_vector::push_back(&mut collection_config.tokens, TokenAsset {
                token_uri: *token_uri,
                property_keys: *vector::borrow(&property_keys, i),
                property_values: *vector::borrow(&property_values, i),
                property_types: *vector::borrow(&property_types, i),
            });
            bucket_table::add(&mut collection_config.deduped_tokens, *token_uri, true);
            i = i + 1;
        };
    }

    /// Mint source certificate.
    public entry fun mint_source_certificate(
        nft_claimer: &signer,
        amount: u64
    ) acquires NFTMintConfig, PublicMintConfig, WhitelistMintConfig, SourceToken, CollectionConfig {
        assert!(exists<CollectionConfig>(@post_mint_reveal_nft) && exists<WhitelistMintConfig>(@post_mint_reveal_nft) && exists<PublicMintConfig>(@post_mint_reveal_nft), error::permission_denied(ECONFIG_NOT_INITIALIZED));

        let whitelist_mint_config = borrow_global_mut<WhitelistMintConfig>(@post_mint_reveal_nft);
        let public_mint_config = borrow_global_mut<PublicMintConfig>(@post_mint_reveal_nft);
        let collection_config = borrow_global_mut<CollectionConfig>(@post_mint_reveal_nft);

        let now = timestamp::now_seconds();
        let is_whitelist_minting_time = now >= whitelist_mint_config.whitelist_minting_start_time && now < whitelist_mint_config.whitelist_minting_end_time;
        let is_public_minting_time = now >= public_mint_config.public_minting_start_time && now < public_mint_config.public_minting_end_time;
        assert!(is_whitelist_minting_time || is_public_minting_time, error::permission_denied(EMINTING_IS_NOT_ENABLED));

        let claimer_addr = signer::address_of(nft_claimer);
        let price = public_mint_config.public_mint_price;
        // if this is the whitelist minting time
        if (is_whitelist_minting_time) {
            assert!(bucket_table::contains(&whitelist_mint_config.whitelisted_address, &claimer_addr), error::permission_denied(EACCOUNT_NOT_WHITELISTED));
            let remaining_mint_allowed = bucket_table::borrow_mut(&mut whitelist_mint_config.whitelisted_address, claimer_addr);
            assert!(amount <= *remaining_mint_allowed, error::invalid_argument(EAMOUNT_EXCEEDS_MINTS_ALLOWED));
            *remaining_mint_allowed = *remaining_mint_allowed - amount;
            price = whitelist_mint_config.whitelist_mint_price;
        } else {
            if (collection_config.public_mint_limit != 0) {
                // If the claimer's address is not on the public_minting_addresses table yet, it means this is the
                // first time that this claimer mints. We will add the claimer's address and remaining amount of mints
                // to the public_minting_addresses table.
                if (!bucket_table::contains(&public_mint_config.public_minting_addresses, &claimer_addr)) {
                    bucket_table::add(&mut public_mint_config.public_minting_addresses, claimer_addr, collection_config.public_mint_limit);
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
    entry fun exchange(nft_claimer: &signer, property_version: u64) acquires NFTMintConfig, CollectionConfig, RevealConfig {
        assert!(exists<CollectionConfig>(@post_mint_reveal_nft) && exists<RevealConfig>(@post_mint_reveal_nft), error::permission_denied(ECONFIG_NOT_INITIALIZED));

        let reveal_config = borrow_global<RevealConfig>(@post_mint_reveal_nft);

        assert!(timestamp::now_seconds() > reveal_config.reveal_time, error::permission_denied(ECANNOT_EXCHANGE_BEFORE_REVEAL_SRARTS));

        let collection_config = borrow_global_mut<CollectionConfig>(@post_mint_reveal_nft);
        let source_collection_name = collection_config.collection_name;
        string::append_utf8(&mut source_collection_name, b" - source collection");
        let token_id = token::create_token_id_raw(@post_mint_reveal_nft, source_collection_name, utf8(b"source token"), property_version);
        assert!(token::balance_of(signer::address_of(nft_claimer), token_id) > 0, error::invalid_argument(ETOKEN_ID_NOT_FOUND));

        let now = timestamp::now_microseconds();
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft);

        // Assert there's still some token uris in the vector.
        assert!(big_vector::length(&collection_config.tokens) > 0, error::permission_denied(ENO_ENOUGH_TOKENS_LEFT));

        // Randomize which token we're assigning to the user.
        let index = now % big_vector::length(&collection_config.tokens);
        let token = big_vector::swap_remove(&mut collection_config.tokens, index);
        bucket_table::remove(&mut collection_config.deduped_tokens, &token.token_uri);

        // The name of the destination token will be based on the property version of the source certificate token.
        let token_name = collection_config.token_name_base;
        string::append_utf8(&mut token_name, b": ");
        let (owner, collection, name, property_version) = token::get_token_id_fields(&token_id);
        let num = u64_to_string(property_version);
        string::append(&mut token_name, num);

        let resource_signer = create_signer_with_capability(&nft_mint_config.signer_cap);
        // Burn the source certificate token.
        token::burn(nft_claimer, owner, collection, name, property_version, 1);

        let token_data_id = create_tokendata(
            &resource_signer,
            collection_config.collection_name,
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

        event::emit_event<ExchangeEvent>(
            &mut nft_mint_config.token_exchange_events,
            ExchangeEvent {
                token_receiver_address: signer::address_of(nft_claimer),
                token_id,
            }
        );
    }

    /// Acquire resource signer if we later need it to do something.
    public fun acquire_resource_signer(
        admin: &signer
    ): signer acquires NFTMintConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));
        create_signer_with_capability(&nft_mint_config.signer_cap)
    }

    // ======================================================================
    //   private helper functions //
    // ======================================================================

    fun mint_source_certificate_internal(nft_claimer: &signer, price: u64, amount: u64) acquires NFTMintConfig, SourceToken, CollectionConfig {
        let receiver_addr = signer::address_of(nft_claimer);

        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft);
        let source_token = borrow_global<SourceToken>(@post_mint_reveal_nft);
        let collection_config = borrow_global_mut<CollectionConfig>(@post_mint_reveal_nft);
        assert!(token::get_tokendata_largest_property_version(@post_mint_reveal_nft, source_token.token_data_id) + amount <= big_vector::length(&collection_config.tokens),
            error::permission_denied(ENO_ENOUGH_TOKENS_LEFT));

        // pay for the source NFT
        coin::transfer<AptosCoin>(nft_claimer, nft_mint_config.treasury, price * amount);

        // mint token to the receiver
        let resource_signer = create_signer_with_capability(&nft_mint_config.signer_cap);

        while (amount > 0) {
            let token_id = token::mint_token(&resource_signer, source_token.token_data_id, 1);
            token::direct_transfer(&resource_signer, nft_claimer, token_id, 1);

            event::emit_event<MintingEvent>(
                &mut nft_mint_config.token_minting_events,
                MintingEvent {
                    token_receiver_address: receiver_addr,
                    token_id,
                }
            );

            // mutate the token properties to update the property version of this token
            let (creator_address, collection, name) = token::get_token_data_id_fields(&source_token.token_data_id);

            token::mutate_token_properties(
                &resource_signer,
                receiver_addr,
                creator_address,
                collection,
                name,
                0,
                1,
                vector<String>[],
                vector<vector<u8>>[],
                vector<String>[],
            );
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
    public fun set_up_test(
        source_account: &signer,
        resource_account: &signer,
        admin_account: &signer,
        wl_nft_claimer: &signer,
        public_nft_claimer: &signer,
        treasury_account: &signer,
        aptos_framework: &signer,
        timestamp: u64,
        collection_maximum: u64,
    ) acquires NFTMintConfig {
        use aptos_framework::account::create_account_for_test;

        // set up global time for testing purpose
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(timestamp);

        create_account_for_test(signer::address_of(source_account));
        // create a resource account from the origin account, mocking the module publishing process
        resource_account::create_resource_account(source_account, vector::empty<u8>(), vector::empty<u8>());
        init_module(resource_account);

        create_account_for_test(signer::address_of(wl_nft_claimer));
        create_account_for_test(signer::address_of(public_nft_claimer));
        create_account_for_test(signer::address_of(admin_account));
        create_account_for_test(signer::address_of(treasury_account));

        let (burn_cap, mint_cap) = aptos_framework::aptos_coin::initialize_for_test(aptos_framework);
        coin::register<AptosCoin>(wl_nft_claimer);
        coin::register<AptosCoin>(public_nft_claimer);
        coin::register<AptosCoin>(treasury_account);
        coin::deposit(signer::address_of(wl_nft_claimer), coin::mint(100, &mint_cap));
        coin::deposit(signer::address_of(public_nft_claimer), coin::mint(100, &mint_cap));

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);

        let colleciton_setting = vector<bool>[false, false, false];
        let token_setting = vector<bool>[false, false, false, false, false];
        set_collection_config_and_create_collection(
            source_account,
            utf8(b"test"),
            utf8(b"test collection description"),
            collection_maximum,
            utf8(b"test collection uri"),
            colleciton_setting,
            utf8(b"base token name"),
            signer::address_of(treasury_account),
            utf8(b"token description"),
            0,
            token_setting,
            1,
            0,
            2,
        );

        set_admin(source_account, signer::address_of(admin_account));
        set_treasury(admin_account, signer::address_of(treasury_account));
    }

    #[test_only]
    public entry fun set_up_token_uris(admin_account: &signer) acquires NFTMintConfig, CollectionConfig, WhitelistMintConfig {
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

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    public entry fun test_happy_path(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, CollectionConfig, SourceToken, RevealConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);

        set_minting_and_reveal_config(&admin_account, 50, 200, 5, 201, 400, 10, 400);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2);
        set_up_token_uris(&admin_account);

        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer, 2);
        let white_list_config = borrow_global_mut<WhitelistMintConfig>(@post_mint_reveal_nft);
        assert!(*bucket_table::borrow(&mut white_list_config.whitelisted_address, signer::address_of(&wl_nft_claimer)) == 0, 0);

        timestamp::fast_forward_seconds(160);
        mint_source_certificate(&public_nft_claimer, 1);
        let public_mint_config = borrow_global_mut<PublicMintConfig>(@post_mint_reveal_nft);
        assert!(*bucket_table::borrow(&mut public_mint_config.public_minting_addresses, signer::address_of(&public_nft_claimer)) == 1, 1);

        // Assert that the source certificates exist in the nft claimers' TokenStore.
        let source_token = borrow_global<SourceToken>(@post_mint_reveal_nft);
        let (owner, collection, name) = token::get_token_data_id_fields(&source_token.token_data_id);
        let token_id1 = token::create_token_id_raw(owner, collection, name, 1);
        let token_id2 = token::create_token_id_raw(owner, collection, name, 2);
        let token_id3 = token::create_token_id_raw(owner, collection, name, 3);
        assert!(token::balance_of(signer::address_of(&wl_nft_claimer), token_id1) == 1, 0);
        assert!(token::balance_of(signer::address_of(&wl_nft_claimer), token_id2) == 1, 1);
        assert!(token::balance_of(signer::address_of(&public_nft_claimer), token_id3) == 1, 2);

        let whitelist_mint_config = borrow_global_mut<WhitelistMintConfig>(@post_mint_reveal_nft);
        assert!(*bucket_table::borrow(&mut whitelist_mint_config.whitelisted_address, signer::address_of(&wl_nft_claimer)) == 0, 0);
        assert!(coin::balance<AptosCoin>(signer::address_of(&treasury_account))== 20, 1);
        assert!(coin::balance<AptosCoin>(signer::address_of(&wl_nft_claimer))== 90, 2);
        assert!(coin::balance<AptosCoin>(signer::address_of(&public_nft_claimer))== 90, 3);

        // Exchange to the destination NFT.
        timestamp::fast_forward_seconds(401);
        exchange(&public_nft_claimer, 3);
        exchange(&wl_nft_claimer, 1);
        exchange(&wl_nft_claimer, 2);

        // Assert that the exchange was successful.
        let collection_config = borrow_global<CollectionConfig>(@post_mint_reveal_nft);
        let exchanged_token_id1 = token::create_token_id_raw(signer::address_of(&resource_account), collection_config.collection_name, utf8(b"base token name: 1"), 0);
        let exchanged_token_id2 = token::create_token_id_raw(signer::address_of(&resource_account), collection_config.collection_name, utf8(b"base token name: 2"), 0);
        let exchanged_token_id3 = token::create_token_id_raw(signer::address_of(&resource_account), collection_config.collection_name, utf8(b"base token name: 3"), 0);
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

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10005, location = Self)]
    public entry fun test_adding_token_uris_exceeds_collection_maximum(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, RevealConfig, CollectionConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 2);
        set_minting_and_reveal_config(&admin_account, 50, 200, 5, 201, 400, 10, 400);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2);
        set_up_token_uris(&admin_account);
    }

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50010, location = Self)]
    public entry fun test_exchange_before_minting_ends(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, CollectionConfig, SourceToken, RevealConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);
        set_minting_and_reveal_config(&admin_account, 50, 200, 5, 201, 400, 10, 400);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2);
        set_up_token_uris(&admin_account);

        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer, 2);

        timestamp::fast_forward_seconds(160);
        mint_source_certificate(&public_nft_claimer, 1);

        // Assert that the source certificates exist in the nft claimers' TokenStore.
        let source_token = borrow_global<SourceToken>(@post_mint_reveal_nft);
        let (owner, collection, name) = token::get_token_data_id_fields(&source_token.token_data_id);
        let token_id1 = token::create_token_id_raw(owner, collection, name, 1);
        assert!(token::balance_of(signer::address_of(&wl_nft_claimer), token_id1) == 1, 0);

        // Exchange to the destination NFT when the minting is ongoing.
        exchange(&wl_nft_claimer, 1);
    }

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50001, location = Self)]
    public entry fun invalid_set_admin_address(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);
        set_admin(&source_account, signer::address_of(&treasury_account));
    }

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50001, location = Self)]
    public entry fun invalid_set_treasury_address(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);
        set_treasury(&source_account, signer::address_of(&treasury_account));
    }

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50001, location = Self)]
    public entry fun invalid_set_minting_time_and_price(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, RevealConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);
        set_minting_and_reveal_config(&source_account, 50, 200, 5, 150, 400, 10, 400);
    }

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10002, location = Self)]
    public entry fun test_invalid_time(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, RevealConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);
        set_minting_and_reveal_config(&admin_account, 50, 200, 5, 150, 400, 10, 400);
    }

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x5000d, location = Self)]
    public entry fun test_mint_before_set_up(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, SourceToken, CollectionConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);
        mint_source_certificate(&wl_nft_claimer, 2);
    }

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x1000e, location = Self)]
    public entry fun test_amount_exceeds_mint_allowed_whitelisted(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, CollectionConfig, SourceToken, RevealConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);
        set_minting_and_reveal_config(&admin_account, 50, 200, 5, 201, 400, 10, 400);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2);
        set_up_token_uris(&admin_account);
        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer, 3);
    }

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x1000e, location = Self)]
    public entry fun test_amount_exceeds_mint_allowed_public(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, CollectionConfig, SourceToken, RevealConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);
        set_minting_and_reveal_config(&admin_account, 50, 200, 5, 201, 400, 10, 400);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2);
        set_up_token_uris(&admin_account);
        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer, 2);
        timestamp::fast_forward_seconds(160);
        mint_source_certificate(&public_nft_claimer, 4);
    }

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50009, location = Self)]
    public entry fun test_minting_source_certificate_exceeds_collection_maximum(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, RevealConfig, CollectionConfig, SourceToken {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 3);
        set_minting_and_reveal_config(&admin_account, 50, 200, 5, 201, 400, 10, 400);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2);
        set_up_token_uris(&admin_account);

        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer, 2);

        timestamp::fast_forward_seconds(160);
        mint_source_certificate(&public_nft_claimer, 2);
    }

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x5000a, location = Self)]
    public entry fun test_account_not_on_whitelist(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, CollectionConfig, SourceToken, RevealConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);
        set_minting_and_reveal_config(&admin_account, 50, 200, 5, 201, 400, 10, 400);
        let wl_addresses = vector::empty<address>();
        add_to_whitelist(&admin_account, wl_addresses, 2);
        set_up_token_uris(&admin_account);
        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&public_nft_claimer, 2);
    }

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    public entry fun test_update_minting_time(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, RevealConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);
        set_minting_and_reveal_config(&admin_account, 50, 200, 5, 201, 400, 10, 400);
        set_minting_and_reveal_config(&admin_account, 60, 300, 10, 400, 600, 50, 600);

        let whitelist_minting_config = borrow_global_mut<WhitelistMintConfig>(@post_mint_reveal_nft);
        assert!(whitelist_minting_config.whitelist_minting_start_time == 60, 0);
        assert!(whitelist_minting_config.whitelist_minting_end_time == 300, 1);
        assert!(whitelist_minting_config.whitelist_mint_price == 10, 2);

        let public_minting_config = borrow_global_mut<PublicMintConfig>(@post_mint_reveal_nft);
        assert!(public_minting_config.public_minting_start_time == 400, 3);
        assert!(public_minting_config.public_minting_end_time == 600, 4);
        assert!(public_minting_config.public_mint_price == 50, 5);
    }

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50007, location = Self)]
    public entry fun invalid_add_to_whitelist(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, RevealConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);
        set_minting_and_reveal_config(&admin_account, 50, 200, 5, 201, 400, 10, 400);
        timestamp::fast_forward_seconds(200);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2);
    }

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50009, location = Self)]
    public entry fun test_all_tokens_minted(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, CollectionConfig, SourceToken, RevealConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);
        set_minting_and_reveal_config(&admin_account, 50, 200, 5, 201, 400, 10, 400);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2);
        set_up_token_uris(&admin_account);

        timestamp::fast_forward_seconds(50);
        mint_source_certificate(&wl_nft_claimer, 2);

        timestamp::fast_forward_seconds(160);
        mint_source_certificate(&public_nft_claimer, 2);
    }

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10004, location = Self)]
    public entry fun test_invalid_add_token_uri(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, CollectionConfig, RevealConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);
        set_minting_and_reveal_config(&admin_account, 50, 200, 5, 201, 400, 10, 400);
        let wl_addresses = vector::empty<address>();
        vector::push_back(&mut wl_addresses, signer::address_of(&wl_nft_claimer));
        add_to_whitelist(&admin_account, wl_addresses, 2);
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

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    public entry fun test_acquire_signer(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);
        let resource_signer = acquire_resource_signer(&admin_account);
        assert!(signer::address_of(&resource_signer) == signer::address_of(&resource_account), 0);
    }

    #[test (source_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin_account = @0x456, wl_nft_claimer = @0x123, public_nft_claimer = @0x234, treasury_account = @0x345, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10011, location = Self)]
    public entry fun test_duplicate_token_uris(
        source_account: signer,
        resource_account: signer,
        admin_account: signer,
        wl_nft_claimer: signer,
        public_nft_claimer: signer,
        treasury_account: signer,
        aptos_framework: signer,
    ) acquires NFTMintConfig, WhitelistMintConfig, PublicMintConfig, RevealConfig, CollectionConfig {
        set_up_test(&source_account, &resource_account, &admin_account, &wl_nft_claimer, &public_nft_claimer, &treasury_account, &aptos_framework, 10, 0);
        set_minting_and_reveal_config(&admin_account, 50, 200, 5, 201, 400, 10, 400);
        set_up_token_uris(&admin_account);
        set_up_token_uris(&admin_account);
    }
}
