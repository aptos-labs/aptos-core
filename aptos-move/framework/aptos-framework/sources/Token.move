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

    // A creator may publish multiple collections
    struct Collections<TokenType: copy + drop + store> has key {
        collections: Table<ASCII::String, Collection<TokenType>>,
    }

    fun initialize_collections<TokenType: copy + drop + store>(account: &signer) {
        move_to(
            account,
            Collections {
                collections: Table::create<ASCII::String, Collection<TokenType>>(),
            },
        )
    }

    // The source of Tokens, their collection!
    struct Collection<TokenType: copy + drop + store> has store {
        // Keep track of all Tokens, even if their balance is 0.
        tokens: Table<ASCII::String, TokenData<TokenType>>,
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
    public(script) fun create_finite_collection_script<TokenType: copy + drop + store>(
        account: signer,
        description: vector<u8>,
        name: vector<u8>,
        uri: vector<u8>,
        maximum: u64,
    ) acquires Collections {
        create_collection<TokenType>(
            &account,
            ASCII::string(description),
            ASCII::string(name),
            ASCII::string(uri),
            Option::some(maximum),
        );
    }

    // Creates a collection with a unbounded number of tokens in it
    public(script) fun create_unlimited_collection_script<TokenType: copy + drop + store>(
        account: signer,
        description: vector<u8>,
        name: vector<u8>,
        uri: vector<u8>,
    ) acquires Collections {
        create_collection<TokenType>(
            &account,
            ASCII::string(description),
            ASCII::string(name),
            ASCII::string(uri),
            Option::none(),
        );
    }


    public fun create_collection<TokenType: copy + drop + store>(
        account: &signer,
        description: ASCII::String,
        name: ASCII::String,
        uri: ASCII::String,
        maximum: Option<u64>,
    ) acquires Collections {
        let account_addr = Signer::address_of(account);
        if (!exists<Collections<TokenType>>(account_addr)) {
            initialize_collections<TokenType>(account)
        };
        if (!exists<Gallery<TokenType>>(account_addr)) {
            initialize_gallery<TokenType>(account)
        };

        let collections = &mut borrow_global_mut<Collections<TokenType>>(account_addr).collections;
        let collection = Collection<TokenType> {
            tokens: Table::create(),
            claimed_tokens: Table::create(),
            description,
            name,
            uri,
            count: 0,
            maximum,
        };

        Table::insert(collections, *&name, collection);
    }

    // An account's set of Tokens
    struct Gallery<phantom TokenType: copy + drop + store> has key {
        gallery: Table<ID, Token<TokenType>>,
    }

    fun initialize_gallery<TokenType: copy + drop + store>(signer: &signer) {
        move_to(
            signer,
            Gallery<TokenType> {
                gallery: Table::create<ID, Token<TokenType>>(),
            },
        )
    }

    // Represents ownership of a the data associated with this Token
    struct Token<phantom TokenType: copy + drop + store> has drop, store {
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
    struct TokenData<TokenType: copy + drop + store> has copy, drop, store {
        // Unique identifier for this token
        id: ID,
        // Describes this Token
        description: ASCII::String,
        // Additional data that describes this Token
        metadata: TokenType,
        // The name of this Token
        name: ASCII::String,
        // Total number of editions of this Token
        supply: u64,
        /// URL for additional information / media
        uri: ASCII::String,
    }

    public fun create_token_script<TokenType: copy + drop + store>(
        account: signer,
        collection_name: vector<u8>,
        description: vector<u8>,
        name: vector<u8>,
        supply: u64,
        uri: vector<u8>,
        metadata: TokenType,
    ) acquires Collections, Gallery {
      create_token<TokenType>(
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
    public fun create_token<TokenType: copy + drop + store>(
        account: &signer,
        collection_name: ASCII::String,
        description: ASCII::String,
        name: ASCII::String,
        supply: u64,
        uri: ASCII::String,
        metadata: TokenType,
    ): ID acquires Collections, Gallery {
        let account_addr = Signer::address_of(account);
        let collections = &mut borrow_global_mut<Collections<TokenType>>(account_addr).collections;
        let gallery = &mut borrow_global_mut<Gallery<TokenType>>(account_addr).gallery;

        let token_id = GUID::id(&GUID::create(account));
        let token = Token<TokenType> {
            id: *&token_id,
            name: *&name,
            collection: *&collection_name,
            balance: supply,
        };

        let token_data = TokenData {
            id: *&token_id,
            description,
            metadata,
            name: *&name,
            supply,
            uri,
        };

        let collection = Table::borrow_mut(collections, &collection_name);
        if (supply == 1) {
            Table::insert(&mut collection.claimed_tokens, *&name, account_addr)
        };
        Table::insert(&mut collection.tokens, name, token_data);

        Table::insert(gallery, *&token_id, token);
        token_id
    }

    public fun token_id<TokenType: copy + drop + store>(token: &Token<TokenType>): &ID {
        &token.id
    }

    public fun withdraw_token<TokenType: copy + drop + store>(
        account: &signer,
        token_id: &ID,
        amount: u64,
    ): Token<TokenType> acquires Gallery {
        let account_addr = Signer::address_of(account);

        let gallery = &mut borrow_global_mut<Gallery<TokenType>>(account_addr).gallery;
        let balance = Table::borrow(gallery, token_id).balance;
        assert!(balance >= amount, EINSUFFICIENT_BALANCE);

        if (balance == amount) {
            let (_key, value) = Table::remove(gallery, token_id);
            value
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

    public fun deposit_token<TokenType: copy + drop + store>(
        account: &signer,
        token: Token<TokenType>,
    ) acquires Collections, Gallery {
        let account_addr = Signer::address_of(account);
        if (!exists<Gallery<TokenType>>(account_addr)) {
            initialize_gallery<TokenType>(account)
        };

        let creator_addr = GUID::id_creator_address(&token.id);
        let collections = &mut borrow_global_mut<Collections<TokenType>>(creator_addr).collections;
        let collection = Table::borrow_mut(collections, &token.collection);
        if (Table::borrow(&collection.tokens, &token.name).supply == 1) {
          Table::remove(&mut collection.claimed_tokens, &token.name);
          Table::insert(&mut collection.claimed_tokens, *&token.name, account_addr)
        };

        let gallery = &mut borrow_global_mut<Gallery<TokenType>>(account_addr).gallery;
        if (Table::contains_key(gallery, &token.id)) {
            let current_token = Table::borrow_mut(gallery, &token.id);
            merge_token(token, current_token);
        } else {
            Table::insert(gallery, *&token.id, token)
        }
    }

    public fun merge_token<TokenType: copy + drop + store>(
        source_token: Token<TokenType>,
        dst_token: &mut Token<TokenType>,
    ) {
        assert!(dst_token.id == source_token.id, EINVALID_TOKEN_MERGE);
        dst_token.balance = dst_token.balance + source_token.balance;
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
            let collections = &borrow_global<Collections<u64>>(creator_addr).collections;
            let collection = Table::borrow(collections, &collection_name);
            assert!(Table::borrow(&collection.claimed_tokens, &token_name) == &creator_addr, 0)
        };

        let token = withdraw_token<u64>(&creator, &token_id, 1);
        deposit_token<u64>(&owner, token);

        let owner_addr = Signer::address_of(&owner);
        {
            let collections = &borrow_global<Collections<u64>>(creator_addr).collections;
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

        let token_0 = withdraw_token<u64>(&creator, &token_id, 1);
        let token_1 = withdraw_token<u64>(&creator, &token_id, 1);
        deposit_token<u64>(&owner, token_0);
        deposit_token<u64>(&creator, token_1);
        let token_2 = withdraw_token<u64>(&creator, &token_id, 1);
        deposit_token<u64>(&owner, token_2);
    }

    fun create_collection_and_token(
        creator: &signer,
        amount: u64,
    ): (ASCII::String, ID) acquires Collections, Gallery {
        let collection_name = ASCII::string(b"Hello, World");
        create_collection<u64>(
            creator,
            ASCII::string(b"Collection: Hello, World"),
            *&collection_name,
            ASCII::string(b"https://aptos.dev"),
            Option::none(),
        );

        let token_id = create_token<u64>(
            creator,
            *&collection_name,
            ASCII::string(b"Token: Hello, Token"),
            ASCII::string(b"Hello, Token"),
            amount,
            ASCII::string(b"https://aptos.dev"),
            0,
        );

        (collection_name, token_id)
    }
}
