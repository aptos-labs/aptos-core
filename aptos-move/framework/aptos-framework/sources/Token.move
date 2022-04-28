/// This module provides the foundation for (collectible) Tokens often called NFTs
module AptosFramework::Token {
    use Std::ASCII;
    use Std::GUID::{Self, ID};
    use Std::Option::{Self, Option};
    use Std::Signer;
    use AptosFramework::Table::{Self, Table};

    // Error map
    const EINSUFFICIENT_BALANCE: u64 = 0;
    const EMISSING_CLAIMED_TOKEN: u64 = 1;
    const EINVALID_TOKEN_MERGE: u64 = 2;
    const EMAXIMUM_NUMBER_OF_TOKENS_FOR_COLLECTION: u64 = 3;

    // A creator may publish multiple collections
    struct Collections has key {
        collections: Table<ASCII::String, Collection>,
    }

    fun initialize_collections(account: &signer) {
        move_to(
            account,
            Collections {
                collections: Table::new<ASCII::String, Collection>(),
            },
        )
    }

    // The source of Tokens, their collection!
    struct Collection has store {
        // Keep track of all Tokens, even if their balance is 0.
        tokens: Table<ASCII::String, TokenData>,
        // In the case of NFTs (supply == 1), keep track of where the tokens are
        claimed_tokens: Table<ASCII::String, address>,
        // Describes the collection
        description: ASCII::String,
        // Unique name within this creators account for this collection
        name: ASCII::String,
        // URL for additional information /media
        uri: ASCII::String,
        // Total number of distinct Tokens tracked by the collection
        count: u64,
        // Optional maximum number of tokens allowed within this collections
        maximum: Option<u64>,
    }

    // Creates a collection with a bounded number of tokens in it
    public(script) fun create_finite_collection_script(
        account: signer,
        description: vector<u8>,
        name: vector<u8>,
        uri: vector<u8>,
        maximum: u64,
    ) acquires Collections {
        create_collection(
            &account,
            ASCII::string(description),
            ASCII::string(name),
            ASCII::string(uri),
            Option::some(maximum),
        );
    }

    // Creates a collection with a unbounded number of tokens in it
    public(script) fun create_unlimited_collection_script(
        account: signer,
        description: vector<u8>,
        name: vector<u8>,
        uri: vector<u8>,
    ) acquires Collections {
        create_collection(
            &account,
            ASCII::string(description),
            ASCII::string(name),
            ASCII::string(uri),
            Option::none(),
        );
    }


    public fun create_collection(
        account: &signer,
        description: ASCII::String,
        name: ASCII::String,
        uri: ASCII::String,
        maximum: Option<u64>,
    ) acquires Collections {
        let account_addr = Signer::address_of(account);
        if (!exists<Collections>(account_addr)) {
            initialize_collections(account)
        };
        if (!exists<Gallery>(account_addr)) {
            initialize_gallery(account)
        };

        let collections = &mut borrow_global_mut<Collections>(account_addr).collections;
        let collection = Collection {
            tokens: Table::new(),
            claimed_tokens: Table::new(),
            description,
            name,
            uri,
            count: 0,
            maximum,
        };

        Table::add(collections, &name, collection);
    }

    // Enables storage solutions of Tokens outside of this module to inform the collection the
    // current hosting address of a token.
    public fun claim_token_ownership(
        account: &signer,
        token: Token,
    ): Token acquires Collections {
        let creator_addr = GUID::id_creator_address(&token.id);
        let collections = &mut borrow_global_mut<Collections>(creator_addr).collections;
        let collection = Table::borrow_mut(collections, &token.collection);
        if (Table::borrow(&collection.tokens, &token.name).supply == 1) {
          Table::remove(&mut collection.claimed_tokens, &token.name);
          Table::add(&mut collection.claimed_tokens, &token.name, Signer::address_of(account))
        };
        token
    }

    // An account's set of Tokens
    struct Gallery has key {
        gallery: Table<ID, Token>,
    }

    fun initialize_gallery(signer: &signer) {
        move_to(
            signer,
            Gallery {
                gallery: Table::new<ID, Token>(),
            },
        )
    }

    // Represents ownership of a the data associated with this Token
    struct Token has store {
        // Unique identifier for this token
        id: ID,
        // The name of this token
        name: ASCII::String,
        // The collection or set of related Tokens
        collection: ASCII::String,
        // Current store of data at this location
        balance: u64,
    }

    // Specific data of a token that can be generalized across an entire edition of an Token
    struct TokenData has copy, drop, store {
        // Unique identifier for this token
        id: ID,
        // Describes this Token
        description: ASCII::String,
        // The name of this Token
        name: ASCII::String,
        // Total number of editions of this Token
        supply: u64,
        /// URL for additional information / media
        uri: ASCII::String,
    }

    // Some Tokens may want additional fields outside the specification. As the Token itself does
    // not contain the data, it is easier in Move to have this extra metadata sit on the side so
    // that only creation and read operations (outside of Move) need to deal with the nuance of
    // metadata.
    struct TokenMetadata<phantom TokenType: store> has key {
        metadata: Table<ID, TokenType>,
    }

    fun initialize_token_metadata<TokenType: store>(account: &signer) {
        move_to(
            account,
            TokenMetadata {
                metadata: Table::new<ID, TokenType>(),
            },
        )
    }

    public(script) fun create_token_script(
        account: signer,
        collection_name: vector<u8>,
        description: vector<u8>,
        name: vector<u8>,
        supply: u64,
        uri: vector<u8>,
    ) acquires Collections, Gallery {
      create_token(
          &account,
          ASCII::string(collection_name),
          ASCII::string(description),
          ASCII::string(name),
          supply,
          ASCII::string(uri),
      );
    }

    public fun create_token_with_metadata_script<TokenType: store>(
        account: signer,
        collection_name: vector<u8>,
        description: vector<u8>,
        name: vector<u8>,
        supply: u64,
        uri: vector<u8>,
        metadata: TokenType,
    ) acquires Collections, Gallery, TokenMetadata {
      create_token_with_metadata<TokenType>(
          &account,
          ASCII::string(collection_name),
          ASCII::string(description),
          ASCII::string(name),
          supply,
          ASCII::string(uri),
          metadata,
      );
    }

    // Create a new token, place the metadata into the collection and the token into the gallery
    public fun create_token(
        account: &signer,
        collection_name: ASCII::String,
        description: ASCII::String,
        name: ASCII::String,
        supply: u64,
        uri: ASCII::String,
    ): ID acquires Collections, Gallery {
        let account_addr = Signer::address_of(account);
        let collections = &mut borrow_global_mut<Collections>(account_addr).collections;
        let collection = Table::borrow_mut(collections, &collection_name);

        if (Option::is_some(&collection.maximum)) {
            let current = Table::length(&collection.tokens);
            let maximum = Option::borrow(&collection.maximum);
            assert!(current != *maximum, EMAXIMUM_NUMBER_OF_TOKENS_FOR_COLLECTION)
        };

        let gallery = &mut borrow_global_mut<Gallery>(account_addr).gallery;

        let token_id = GUID::id(&GUID::create(account));
        let token = Token {
            id: *&token_id,
            name: *&name,
            collection: *&collection_name,
            balance: supply,
        };

        let token_data = TokenData {
            id: *&token_id,
            description,
            name: *&name,
            supply,
            uri,
        };

        if (supply == 1) {
            Table::add(&mut collection.claimed_tokens, &name, account_addr)
        };
        Table::add(&mut collection.tokens, &name, token_data);

        let token = claim_token_ownership(account, token);
        Table::add(gallery, &token_id, token);
        token_id
    }

    public fun create_token_with_metadata<TokenType: store>(
        account: &signer,
        collection_name: ASCII::String,
        description: ASCII::String,
        name: ASCII::String,
        supply: u64,
        uri: ASCII::String,
        metadata: TokenType,
    ): ID acquires Collections, Gallery, TokenMetadata {
        let account_addr = Signer::address_of(account);
        if (!exists<TokenMetadata<TokenType>>(account_addr)) {
            initialize_token_metadata<TokenType>(account)
        };

        let id = create_token(account, collection_name, description, name, supply, uri);
        let metadata_table = borrow_global_mut<TokenMetadata<TokenType>>(account_addr);
        Table::add(&mut metadata_table.metadata, &id, metadata);
        id
    }

    public fun token_id(token: &Token): &ID {
        &token.id
    }

    public fun withdraw_token(
        account: &signer,
        token_id: &ID,
        amount: u64,
    ): Token acquires Gallery {
        let account_addr = Signer::address_of(account);

        let gallery = &mut borrow_global_mut<Gallery>(account_addr).gallery;
        let balance = Table::borrow(gallery, token_id).balance;
        assert!(balance >= amount, EINSUFFICIENT_BALANCE);

        if (balance == amount) {
            Table::remove(gallery, token_id)
        } else {
            let token = Table::borrow_mut(gallery, token_id);
            token.balance = balance - amount;
            Token {
                id: *&token.id,
                name: *&token.name,
                collection: *&token.collection,
                balance: amount,
            }
        }
    }

    public fun deposit_token(
        account: &signer,
        token: Token,
    ) acquires Collections, Gallery {
        let account_addr = Signer::address_of(account);
        if (!exists<Gallery>(account_addr)) {
            initialize_gallery(account)
        };

        let token = claim_token_ownership(account, token);

        let gallery = &mut borrow_global_mut<Gallery>(account_addr).gallery;
        if (Table::contains(gallery, &token.id)) {
            let current_token = Table::borrow_mut(gallery, &token.id);
            merge_token(token, current_token);
        } else {
            let token_id = token.id;
            Table::add(gallery, &token_id, token)
        }
    }

    public fun merge_token(
        source_token: Token,
        dst_token: &mut Token,
    ) acquires Collections {
        assert!(dst_token.id == source_token.id, EINVALID_TOKEN_MERGE);
        dst_token.balance = dst_token.balance + source_token.balance;
        destroy_token(source_token);
    }

    fun destroy_token(
        token: Token,
    ) acquires Collections {
        let Token { id, name, collection, balance } = token;

        let creator_addr = GUID::id_creator_address(&id);
        let collections = &mut borrow_global_mut<Collections>(creator_addr).collections;
        let collection = Table::borrow_mut(collections, &collection);
        let token_data = Table::borrow_mut(&mut collection.tokens, &name);
        *&mut token_data.supply = token_data.supply - balance;
    }

    public(script) fun direct_transfer(
        sender: &signer,
        receiver: &signer,
        creator: address,
        creation_num: u64,
        amount: u64,
    ) acquires Collections, Gallery {
        let token_id = GUID::create_id(creator, creation_num);
        let token = withdraw_token(sender, &token_id, amount);
        deposit_token(receiver, token)
    }

    #[test(creator = @0x1, owner = @0x2)]
    public fun create_withdraw_deposit_nft(
        creator: signer,
        owner: signer,
    ) acquires Collections, Gallery {
        let (collection_name, token_id) = create_collection_and_token(&creator, 1);
        let token_name = ASCII::string(b"Hello, Token");

        let creator_addr = Signer::address_of(&creator);
        {
            let collections = &borrow_global<Collections>(creator_addr).collections;
            let collection = Table::borrow(collections, &collection_name);
            assert!(Table::borrow(&collection.claimed_tokens, &token_name) == &creator_addr, 0)
        };

        let token = withdraw_token(&creator, &token_id, 1);
        deposit_token(&owner, token);

        let owner_addr = Signer::address_of(&owner);
        {
            let collections = &borrow_global<Collections>(creator_addr).collections;
            let collection = Table::borrow(collections, &collection_name);
            assert!(Table::borrow(&collection.claimed_tokens, &token_name) == &owner_addr, 1)
        };
    }

    #[test(creator = @0x1, owner = @0x2)]
    public fun create_withdraw_deposit_editions(
        creator: signer,
        owner: signer,
    ) acquires Collections, Gallery {
        let (_collection_name, token_id) = create_collection_and_token(&creator, 2);

        let token_0 = withdraw_token(&creator, &token_id, 1);
        let token_1 = withdraw_token(&creator, &token_id, 1);
        deposit_token(&owner, token_0);
        deposit_token(&creator, token_1);
        let token_2 = withdraw_token(&creator, &token_id, 1);
        deposit_token(&owner, token_2);
    }

    #[test(creator = @0x1)]
    #[expected_failure(abort_code = 3)]
    public fun test_maximum(creator: signer) acquires Collections, Gallery {
        let (collection_name, _token_id) = create_collection_and_token(&creator, 2);
        create_token(
            &creator,
            collection_name,
            ASCII::string(b"Token: Hello, Token 2"),
            ASCII::string(b"Hello, Token 2"),
            1,
            ASCII::string(b"https://aptos.dev"),
        );
    }

    #[test(creator = @0x1, owner = @0x2)]
    public(script) fun direct_transfer_test(creator: signer, owner: signer) acquires Collections, Gallery {
        let (_collection_name, token_id) = create_collection_and_token(&creator, 2);
        let creator_addr = GUID::id_creator_address(&token_id);
        let creation_num = GUID::id_creation_num(&token_id);
        direct_transfer(&creator, &owner, creator_addr, creation_num, 1);
        let token = withdraw_token(&owner, &token_id, 1);
        deposit_token(&creator, token);
    }

    fun create_collection_and_token(
        creator: &signer,
        amount: u64,
    ): (ASCII::String, ID) acquires Collections, Gallery {
        let collection_name = ASCII::string(b"Hello, World");
        create_collection(
            creator,
            ASCII::string(b"Collection: Hello, World"),
            *&collection_name,
            ASCII::string(b"https://aptos.dev"),
            Option::some(1),
        );

        let token_id = create_token(
            creator,
            *&collection_name,
            ASCII::string(b"Token: Hello, Token"),
            ASCII::string(b"Hello, Token"),
            amount,
            ASCII::string(b"https://aptos.dev"),
        );

        (collection_name, token_id)
    }
}
