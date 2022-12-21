module post_mint_reveal_nft_custom::minting {
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
    use aptos_token::token::{Self, create_token_mutability_config, create_collection, create_tokendata, TokenId};
    use post_mint_reveal_nft_custom::bucket_table::{Self, BucketTable};
    use post_mint_reveal_nft_custom::big_vector::{Self, BigVector};
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
        // this is base name, when exchanging from a certificate token to a destination token,
        // we will generate the destination token name as `token_name_base #number of the key`
        token_name_base: String,
        royalty_payee_address: address,
        token_description: String,
        token_maximum: u64,
        royalty_points_den: u64,
        royalty_points_num: u64,
        tokens: BigVector<TokenAsset>,
    }

    struct TokenAsset has drop, store {
        token_uri: String,
    }

    /// WhitelistMintConfig stores information about whitelist minting.
    struct WhitelistMintConfig has key {
        whitelisted_address: BucketTable<address, u64>,
    }

    /// RevealConfig stores the reveal_time for the collection.
    /// After the reveal time, users can exchange their source
    /// certificate token to an NFT in the destination collection.
    struct RevealConfig has key {
        reveal_time: u64,
        price: u64,
    }

    /// SourceToken stores the token_data_id of the source certificate token.
    struct SourceToken has key {
        collection_name: String,
        base_token_name: String,
        token_uri: String,
        token_description: String, 
        royalty_payee_address: address,
        royalty_points_den: u64,
        royalty_points_num: u64,
        largest_token_number: u64,
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
        price: u64,
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
    const ECANNOT_EXCHANGE_BEFORE_REVEAL_STARTS: u64 = 16;

    const COLLECTION_MUTABILITY_CONFIG: vector<bool> = vector[true, true, true];
    const TOKEN_MUTABILITY_CONFIG: vector<bool> = vector[true, true, true, true, true];

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
        coin::register<AptosCoin>(post_mint_reveal_nft_resource_account);
    }

    /// Set admin of this module.
    public entry fun set_admin(admin: &signer, new_admin_address: address) acquires NFTMintConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft_custom);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));
        nft_mint_config.admin = new_admin_address;
    }

    /// Set the treasury account (where the payment for NFT goes to) of this module.
    public entry fun set_treasury(admin: &signer, new_treasury_address: address) acquires NFTMintConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft_custom);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));
        nft_mint_config.treasury = new_treasury_address;
    }

    /// Set up and create the destination collection.
    public entry fun set_destination_collection_config(
        admin: &signer,
        collection_name: String,
        collection_description: String,
        collection_maximum: u64,
        collection_uri: String,
        token_name_base: String,
        royalty_payee_address: address,
        token_description: String,
        token_maximum: u64,
        royalty_points_den: u64,
        royalty_points_num: u64,
    ) acquires NFTMintConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft_custom);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));
        
        assert!(royalty_points_den > 0 && royalty_points_num < royalty_points_den, error::invalid_argument(EINVALID_ROYALTY_NUMERATOR_DENOMINATOR));
        // TODO: can we change collection config if it's already set?`
        assert!(!exists<CollectionConfig>(@post_mint_reveal_nft_custom), error::permission_denied(ECOLLECTION_ALREADY_CREATED));

        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft_custom);
        let resource_signer = create_signer_with_capability(&nft_mint_config.signer_cap);
        move_to(&resource_signer, CollectionConfig {
            collection_name,
            collection_description,
            collection_maximum,
            collection_uri,
            token_name_base,
            royalty_payee_address,
            token_description,
            token_maximum,
            royalty_points_den,
            royalty_points_num,
            tokens: big_vector::empty<TokenAsset>(128),
        });
    }

    /// Set up and create the destination collection.
    public entry fun create_destination_collection_from_config(
        admin: &signer,
    ) acquires NFTMintConfig, CollectionConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft_custom);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));
        
        let resource_signer = create_signer_with_capability(&nft_mint_config.signer_cap);
        let collection_config = borrow_global_mut<CollectionConfig>(@post_mint_reveal_nft_custom);

        // Create the destination collection that holds the unique art NFT.
        create_collection(
            &resource_signer,
            collection_config.collection_name,
            collection_config.collection_description,
            collection_config.collection_uri,
            collection_config.collection_maximum,
            COLLECTION_MUTABILITY_CONFIG
        );
    }

    public entry fun create_keys_collection_with_key_metadata(
        admin: &signer,
        collection_name: String,
        collection_description: String,
        collection_maximum: u64,
        collection_uri: String,
        base_token_name: String,
        token_description: String,
        token_uri: String,
        royalty_payee_address: address,
        royalty_points_den: u64,
        royalty_points_num: u64,
    ) acquires NFTMintConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft_custom);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));

        assert!(royalty_points_den > 0 && royalty_points_num < royalty_points_den, error::invalid_argument(EINVALID_ROYALTY_NUMERATOR_DENOMINATOR));

        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft_custom);
        let resource_signer = create_signer_with_capability(&nft_mint_config.signer_cap);

        // Create the source certificate collection and token.
        create_collection(&resource_signer, collection_name, collection_description, collection_uri, collection_maximum, COLLECTION_MUTABILITY_CONFIG);

        let source_token = SourceToken {
            collection_name,
            base_token_name,
            token_description,
            token_uri, 
            royalty_payee_address, 
            royalty_points_den,
            royalty_points_num,
            largest_token_number: 0,
        };

        move_to(&resource_signer, source_token);
    }

    /// Set the reveal config of this collection.
    public entry fun set_reveal_config(
        admin: &signer,
        reveal_time: u64,
        price: u64,
    ) acquires NFTMintConfig, RevealConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft_custom);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));

        if (exists<RevealConfig>(@post_mint_reveal_nft_custom)) {
            let reveal_config = borrow_global_mut<RevealConfig>(@post_mint_reveal_nft_custom);
            reveal_config.reveal_time = reveal_time;
            reveal_config.price = price;
        } else {
            let resource_account = create_signer_with_capability(&nft_mint_config.signer_cap);
            move_to(&resource_account, RevealConfig {
                reveal_time,
                price,
            });
        };
    }

    /// Add user addresses to the whitelist.
    public entry fun add_to_whitelist(
        admin: &signer,
        wl_addresses: vector<address>,
        mint_limit: u64
    ) acquires NFTMintConfig, WhitelistMintConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft_custom);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));
        if (!exists<WhitelistMintConfig>(@post_mint_reveal_nft_custom)) {
            let resource_account = create_signer_with_capability(&nft_mint_config.signer_cap);
            move_to(&resource_account, WhitelistMintConfig {
                whitelisted_address: bucket_table::new<address, u64>(10),
            });
        };
        let whitelist_mint_config = borrow_global_mut<WhitelistMintConfig>(@post_mint_reveal_nft_custom);

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
    ) acquires NFTMintConfig, CollectionConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft_custom);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));

        assert!(exists<CollectionConfig>(@post_mint_reveal_nft_custom), error::permission_denied(ECONFIG_NOT_INITIALIZED));
        
        let collection_config = borrow_global_mut<CollectionConfig>(@post_mint_reveal_nft_custom);

        assert!(vector::length(&token_uris) + big_vector::length(&collection_config.tokens) <= collection_config.collection_maximum || collection_config.collection_maximum == 0, error::invalid_argument(EEXCEEDS_COLLECTION_MAXIMUM));

        let i = 0;
        while (i < vector::length(&token_uris)) {
            big_vector::push_back(&mut collection_config.tokens, TokenAsset {
                token_uri: *vector::borrow(&token_uris, i),
            });
            i = i + 1;
        };
    }

    /// Mint source certificate, backdoor for admin
    public entry fun mint_keys_admin(
        admin: &signer,
        amount: u64
    ) acquires NFTMintConfig, SourceToken {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft_custom);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));
        
        mint_source_certificate_internal(admin, amount);
    }

    entry fun mint_keys(
        nft_claimer: &signer,
        amount: u64
    ) acquires NFTMintConfig, SourceToken, WhitelistMintConfig {
        let claimer_addr = signer::address_of(nft_claimer);
        let whitelist_mint_config = borrow_global_mut<WhitelistMintConfig>(@post_mint_reveal_nft_custom);

        assert!(bucket_table::contains(&whitelist_mint_config.whitelisted_address, &claimer_addr), error::permission_denied(EACCOUNT_NOT_WHITELISTED));
        let remaining_mint_allowed = bucket_table::borrow_mut(&mut whitelist_mint_config.whitelisted_address, claimer_addr);
        assert!(amount <= *remaining_mint_allowed, error::invalid_argument(EAMOUNT_EXCEEDS_MINTS_ALLOWED));
        *remaining_mint_allowed = *remaining_mint_allowed - amount;

        mint_source_certificate_internal(nft_claimer, amount);
    }

    // Exchange a source certificate token to a destination token. This function will burn the source certificate
    // and put a destination token in the nft_claimer's TokenStore.
    entry fun exchange(nft_claimer: &signer, source_token_name: String) acquires NFTMintConfig, CollectionConfig, RevealConfig, SourceToken {
        assert!(exists<CollectionConfig>(@post_mint_reveal_nft_custom) && exists<RevealConfig>(@post_mint_reveal_nft_custom), error::permission_denied(ECONFIG_NOT_INITIALIZED));

        let reveal_config = borrow_global<RevealConfig>(@post_mint_reveal_nft_custom);
        assert!(timestamp::now_seconds() > reveal_config.reveal_time, error::permission_denied(ECANNOT_EXCHANGE_BEFORE_REVEAL_STARTS));

        let source_token = borrow_global<SourceToken>(@post_mint_reveal_nft_custom);

        let collection_config = borrow_global_mut<CollectionConfig>(@post_mint_reveal_nft_custom);
        let source_collection_name = source_token.collection_name;
        
        let token_id = token::create_token_id_raw(@post_mint_reveal_nft_custom, source_collection_name, source_token_name, 0);
        assert!(token::balance_of(signer::address_of(nft_claimer), token_id) > 0, error::invalid_argument(ETOKEN_ID_NOT_FOUND));

        let now = timestamp::now_microseconds();
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft_custom);

        // Assert there's still some token uris in the vector.
        assert!(big_vector::length(&collection_config.tokens) > 0, error::permission_denied(ENO_ENOUGH_TOKENS_LEFT));

        // Randomize which token we're assigning to the user.
        let index = now % big_vector::length(&collection_config.tokens);
        let token = big_vector::swap_remove(&mut collection_config.tokens, index);

        // The name of the destination token will be based on the name of the source token
        let token_name = collection_config.token_name_base;
        string::append_utf8(&mut token_name, b" #");
        let num = num_from_source_token_name(source_token_name);
        string::append(&mut token_name, num);

        let resource_signer = create_signer_with_capability(&nft_mint_config.signer_cap);
        // Burn the source certificate token.
        token::burn(nft_claimer, @post_mint_reveal_nft_custom, source_collection_name, source_token_name, 0, 1);

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
            create_token_mutability_config(&TOKEN_MUTABILITY_CONFIG),
            vector[],
            vector[],
            vector[],
        );

        let token_id = token::mint_token(&resource_signer, token_data_id, 1);
        token::direct_transfer(&resource_signer, nft_claimer, token_id, 1);

        // pay for the NFT
        let price = reveal_config.price;
        coin::transfer<AptosCoin>(nft_claimer, nft_mint_config.treasury, price);

        event::emit_event<ExchangeEvent>(
            &mut nft_mint_config.token_exchange_events,
            ExchangeEvent {
                token_receiver_address: signer::address_of(nft_claimer),
                price,
                token_id,
            }
        );
    }

    /// Acquire resource signer if we later need it to do something.
    public fun acquire_resource_signer(
        admin: &signer
    ): signer acquires NFTMintConfig {
        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft_custom);
        assert!(signer::address_of(admin) == nft_mint_config.admin, error::permission_denied(ENOT_AUTHORIZED));
        create_signer_with_capability(&nft_mint_config.signer_cap)
    }

    // ======================================================================
    //   private helper functions //
    // ======================================================================

    fun mint_source_certificate_internal(nft_claimer: &signer, amount: u64) acquires NFTMintConfig, SourceToken {
        let receiver_addr = signer::address_of(nft_claimer);

        let nft_mint_config = borrow_global_mut<NFTMintConfig>(@post_mint_reveal_nft_custom);
        let source_token = borrow_global_mut<SourceToken>(@post_mint_reveal_nft_custom);

        // mint token to the receiver
        let resource_signer = create_signer_with_capability(&nft_mint_config.signer_cap);

        while (amount > 0) {
            let token_name = source_token.base_token_name;
            string::append_utf8(&mut token_name, b" #");
            let num = u64_to_string(source_token.largest_token_number);
            string::append(&mut token_name, num);
            token::create_token_script(
                &resource_signer,
                source_token.collection_name,
                token_name,
                source_token.token_description,
                1,
                1,
                source_token.token_uri,
                source_token.royalty_payee_address,
                source_token.royalty_points_den,
                source_token.royalty_points_num,
                TOKEN_MUTABILITY_CONFIG,
                vector<String>[utf8(BURNABLE_BY_OWNER)],
                vector<vector<u8>>[bcs::to_bytes<bool>(&true)],
                vector<String>[utf8(b"bool")],
            );
            let token_id = token::create_token_id_raw(
                signer::address_of(&resource_signer),
                source_token.collection_name,
                token_name,
                0
            );
            token::direct_transfer(&resource_signer, nft_claimer, token_id, 1);

            event::emit_event<MintingEvent>(
                &mut nft_mint_config.token_minting_events,
                MintingEvent {
                    token_receiver_address: receiver_addr,
                    token_id,
                }
            );

            source_token.largest_token_number = source_token.largest_token_number + 1;
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

    fun num_from_source_token_name(name: String): String {
        let ind = string::index_of(&name, &string::utf8(b"#"));
        string::sub_string(&name, ind + 1, string::length(&name))
    }

    // ======================================================================
    //   unit tests //
    // ======================================================================
}
