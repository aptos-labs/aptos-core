
module 0xABCD::token_v1 {
    use std::signer;
    use std::string::{Self, String};
    use std::vector;
    use aptos_token::token::{Self, Token, TokenId, TokenDataId};
    use aptos_std::table::{Self, Table};
    use aptos_framework::account;
    use std::option::{Self, Option};
    use aptos_std::string_utils::{to_string};

    //
    //  Code for minting Token V1
    //

    // Token URI used for NFT Mints
    const COLLECTION_NAME: vector<u8> = b"An NFT Collection Name";
    const COLLECTION_DESCRIPTION: vector<u8> = b"An NFT Collection Description";
    const COLLECTION_URL: vector<u8> = b"";
    const TOKEN_URI: vector<u8> = b"https://aptos.dev";
    const TOKEN_DESCRIPTION: vector<u8> = b"";
    const TOKEN_NAME: vector<u8> = b"NFT Collectible";

    // Royalty Config for NFTs (0/100 = 0%)
    const ROYALTY_POINTS_NUMERATOR: u64 = 0;
    const ROYALTY_POINTS_DENOMINATOR: u64 = 100;

    struct MinterConfig has key {
        signer_cap: account::SignerCapability,
        minted_tokens: Table<address, Option<Token>>,
        tokendata_id: TokenDataId,
    }

    public entry fun token_v1_initialize_collection(creator: &signer) {
        // Create the signer capability for the collection
        let (resource_signer, signer_cap) = account::create_resource_account(creator, vector::empty());

        let collection_name = string::utf8(COLLECTION_NAME);
        let description = string::utf8(COLLECTION_DESCRIPTION);
        let collection_uri = string::utf8(COLLECTION_URL);
        // One million tokens max
        let maximum_supply = 1000000;
        let mutate_setting = vector<bool>[ false, false, false ];

        token::create_collection(
            &resource_signer,
            collection_name,
            description,
            collection_uri,
            maximum_supply,
            mutate_setting
        );

        // Create token data for fungible token
        let tokendata_id = token_v1_create_token_data(&resource_signer, string::utf8(TOKEN_URI), string::utf8(TOKEN_NAME), 5000000);

        // Create the Minter resource and publish it under the creator's address
        move_to(creator, MinterConfig {
            signer_cap,
            minted_tokens: table::new(),
            tokendata_id
        });
    }

    fun get_signer(account_address: address): signer acquires MinterConfig {
        account::create_signer_with_capability(&borrow_global<MinterConfig>(account_address).signer_cap)
    }

    fun set_token_minted(user_address: address, creator_address: address, token: Token) acquires MinterConfig {
        let minted_tokens = &mut borrow_global_mut<MinterConfig>(creator_address).minted_tokens;
        table::add(minted_tokens, user_address, option::some(token));
    }

    /// Make the tokendata name unique by appending a ` #` and the index
    fun build_token_name(token_prefix: String, index: u64): String {
        string::append_utf8(&mut token_prefix, b" #");
        string::append(&mut token_prefix, to_string<u64>(&index));
        token_prefix
    }

    fun token_v1_create_token_data(
        creator: &signer,
        token_uri: String,
        tokendata_name: String,
        maximum: u64
    ): TokenDataId {
        let collection_name = string::utf8(COLLECTION_NAME);
        let tokendata_description = string::utf8(TOKEN_DESCRIPTION);
        let creator_address = signer::address_of(creator);

        // tokan max mutable: true
        // token URI mutable: true
        // token description mutable: true
        // token royalty mutable: true
        // token properties mutable: true
        let token_mutate_config = token::create_token_mutability_config(&vector<bool>[ true, true, true, true, true ]);

        let property_keys = vector[];
        let property_values = vector[];
        let property_types = vector[];

        token::create_tokendata(
            creator,
            collection_name,
            tokendata_name,
            tokendata_description,
            maximum,
            token_uri,
            creator_address,
            ROYALTY_POINTS_DENOMINATOR,
            ROYALTY_POINTS_NUMERATOR,
            token_mutate_config,
            property_keys,
            property_values,
            property_types
        )
    }


    fun mint_nft_sequential(creator_address: address) : (signer, TokenId) acquires MinterConfig {
        let resource_signer = get_signer(creator_address);
        let resource_signer_address = signer::address_of(&resource_signer);

        // Make the tokendata name unique by appending a ` #` and the current supply + 1
        let current_supply_opt = token::get_collection_supply(resource_signer_address, string::utf8(COLLECTION_NAME));
        let index = option::extract(&mut current_supply_opt) + 1;
        let tokendata_name = build_token_name(string::utf8(TOKEN_NAME), index);

        let tokendata_id = token_v1_create_token_data(&resource_signer, string::utf8(TOKEN_URI), tokendata_name, 1);
        let token_id = token::mint_token(&resource_signer, tokendata_id, 1);
        (resource_signer, token_id)
    }

    fun mint_nft_parallel(user: &signer, creator_address: address) : (signer, TokenId) acquires MinterConfig {
        let resource_signer = get_signer(creator_address);
        let token_name = build_token_name(to_string<address>(&signer::address_of(user)), account::get_sequence_number(signer::address_of(user)));
        let tokendata_id = token_v1_create_token_data(&resource_signer, string::utf8(TOKEN_URI), token_name, 1);
        let token_id = token::mint_token(&resource_signer, tokendata_id, 1);
        (resource_signer, token_id)
    }

    /// Mint NFT and store it in a table
    public entry fun token_v1_mint_and_store_nft_sequential(user: &signer, creator_address: address) acquires MinterConfig {
        let (resource_signer, token_id) = mint_nft_sequential(creator_address);
        let token = token::withdraw_token(&resource_signer, token_id, 1);
        set_token_minted(signer::address_of(user), creator_address, token);
    }

    /// Mint NFT and transfer it to the user
    public entry fun token_v1_mint_and_transfer_nft_sequential(user: &signer, creator_address: address) acquires MinterConfig {
        let (resource_signer, token_id) = mint_nft_sequential(creator_address);
        token::direct_transfer(&resource_signer, user, token_id, 1);
    }

    /// Mint NFT and store it in a table
    public entry fun token_v1_mint_and_store_nft_parallel(user: &signer, creator_address: address) acquires MinterConfig {
        let (resource_signer, token_id) = mint_nft_parallel(user, creator_address);
        let token = token::withdraw_token(&resource_signer, token_id, 1);
        set_token_minted(signer::address_of(user), creator_address, token);
    }

    /// Mint NFT and transfer it to the user
    public entry fun token_v1_mint_and_transfer_nft_parallel(user: &signer, creator_address: address) acquires MinterConfig {
        let (resource_signer, token_id) = mint_nft_parallel(user, creator_address);
        token::direct_transfer(&resource_signer, user, token_id, 1);
    }

    /// Mint FT and store it in a table
    public entry fun token_v1_mint_and_store_ft(user: &signer, creator_address: address) acquires MinterConfig {
        let resource_signer = &get_signer(creator_address);
        let tokendata_id = borrow_global<MinterConfig>(creator_address).tokendata_id;
        let token_id = token::mint_token(resource_signer, tokendata_id, 1);
        let token = token::withdraw_token(resource_signer, token_id, 1);
        set_token_minted(signer::address_of(user), creator_address, token);
    }

    /// Mint FT and transfer it to the user
    public entry fun token_v1_mint_and_transfer_ft(user: &signer, creator_address: address) acquires MinterConfig {
        let resource_signer = &get_signer(creator_address);
        let tokendata_id = borrow_global<MinterConfig>(creator_address).tokendata_id;
        let token_id = token::mint_token(resource_signer, tokendata_id, 1);
        token::direct_transfer(resource_signer, user, token_id, 1);
    }
}
