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
        collections: Table<ID, Collection<TokenType>>,
    }

    fun initialize_collections<TokenType: copy + drop + store>(account: &signer) {
        move_to(
            account,
            Collections {
                collections: Table::create<ID, Collection<TokenType>>(),
            },
        )
    }

    // The source of Tokens, their collection!
    struct Collection<TokenType: copy + drop + store> has store {
        // Keep track of all Tokens, even if their balance is 0.
        tokens: Table<ID, TokenMetadata<TokenType>>,
        // In the case of NFTs (supply == 1), keep track of where the tokens are
        claimed_tokens: Table<ID, address>,
        // Unique identifier for this collection
        id: ID,
        description: ASCII::String,
        name: ASCII::String,
        // URL for additional information /media
        uri: ASCII::String,
        // Total number of distinct Tokens tracked by the collection
        count: u64,
        // Optional maximum number of tokens allowed within this collections
        maximum: Option<u64>,
    }

    public fun create_collection<TokenType: copy + drop + store>(
        account: &signer,
        description: ASCII::String,
        name: ASCII::String,
        uri: ASCII::String,
        maximum: Option<u64>,
    ): ID acquires Collections {
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
            id: GUID::id(&GUID::create(account)),
            description,
            name,
            uri,
            count: 0,
            maximum,
        };

        let id = *&collection.id;
        Table::insert(collections, *&id, collection);
        id
    }

    // An account's set of Tokens
    struct Gallery<TokenType: copy + drop + store> has key {
        gallery: Table<ID, Token<TokenType>>,
    }

    fun initialize_gallery<TokenType: copy + drop + store>(signer: &signer) {
        move_to(
            signer,
            Gallery {
                gallery: Table::create<ID, Token<TokenType>>(),
            },
        )
    }

    // A non-fungible or semi-fungible (edition) token
    struct Token<TokenType: copy + drop + store> has drop, store {
        // Unique identifier for this token
        id: ID,
        // The collection or set of related Tokens
        collection: ID,
        // Current store of data at this location
        balance: u64,
        // Token data, left as optional as it can be stored directly with the Token or at the
        // source, currently the intent is to copy
        data: Option<TokenData<TokenType>>,
    }

    // The metadata of a non-fungible or semi-fungible (edition) token -- that is it doesn't
    // contain a balance or pointer back to the collection.
    struct TokenMetadata<TokenType: copy + drop + store> has drop, store {
        // Unique identifier for this token
        id: ID,
        // Token data, left as optional as it can be stored directly with the Token or at the
        // source, currently the intent is to copy
        data: Option<TokenData<TokenType>>,
    }

    // Specific data of a token that can be generalized across an entire edition of an Token
    struct TokenData<TokenType: copy + drop + store> has copy, drop, store {
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

    public fun token_id<TokenType: copy + drop + store>(token: &Token<TokenType>): &ID {
        &token.id
    }

    // Create a new token, place the metadata into the collection and the token into the gallery
    public fun create_token<TokenType: copy + drop + store>(
        account: &signer,
        collection_id: ID,
        description: ASCII::String,
        name: ASCII::String,
        supply: u64,
        uri: ASCII::String,
        metadata: TokenType,
    ): ID acquires Collections, Gallery {
        let account_addr = Signer::address_of(account);
        let collections = &mut borrow_global_mut<Collections<TokenType>>(account_addr).collections;
        let gallery = &mut borrow_global_mut<Gallery<TokenType>>(account_addr).gallery;

        let some_data = Option::some(TokenData {
            description,
            metadata,
            name,
            supply,
            uri,
        });

        let (collection_data, gallery_data) = if (supply == 1) {
            (Option::none(), some_data)
        } else {
            (some_data, Option::none())
        };

        let collection_token = TokenMetadata {
            id: GUID::id(&GUID::create(account)),
            data: collection_data,
        };

        let token_id  = *&collection_token.id;
        let collection = Table::borrow_mut(collections, &collection_id);
        if (supply == 1) {
            Table::insert(&mut collection.claimed_tokens, *&collection_token.id, account_addr)
        };
        Table::insert(&mut collection.tokens, *&collection_token.id, collection_token);

        let gallery_token = Token {
            id: *&token_id,
            collection: collection_id,
            balance: supply,
            data: gallery_data,
        };

        Table::insert(gallery, *&gallery_token.id, gallery_token);
        token_id
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
                collection: *&token.collection,
                balance: amount,
                data: *&token.data,
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

        let creator_addr = GUID::id_creator_address(&token.collection);
        let collections = &mut borrow_global_mut<Collections<TokenType>>(creator_addr).collections;
        let collection = Table::borrow_mut(collections, &token.collection);
        if (Option::is_some(&token.data) && Option::borrow(&token.data).supply == 1) {
          Table::remove(&mut collection.claimed_tokens, &token.id);
          Table::insert(&mut collection.claimed_tokens, *&token.id, account_addr)
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
        let (collection_id, token_id) = create_collection_and_token(&creator, 1);

        let creator_addr = Signer::address_of(&creator);
        {
            let collections = &borrow_global<Collections<u64>>(creator_addr).collections;
            let collection = Table::borrow(collections, &collection_id);
            assert!(Table::borrow(&collection.claimed_tokens, &token_id) == &creator_addr, 0)
        };

        let token = withdraw_token<u64>(&creator, &token_id, 1);
        deposit_token<u64>(&owner, token);

        let owner_addr = Signer::address_of(&owner);
        {
            let collections = &borrow_global<Collections<u64>>(creator_addr).collections;
            let collection = Table::borrow(collections, &collection_id);
            assert!(Table::borrow(&collection.claimed_tokens, &token_id) == &owner_addr, 1)
        };
    }

    #[test(creator = @0x1, owner = @0x2)]
    public fun create_withdraw_deposit_editions(
        creator: signer,
        owner: signer,
    ) acquires Collections, Gallery {
        let (_collection_id, token_id) = create_collection_and_token(&creator, 2);

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
    ): (ID, ID) acquires Collections, Gallery {
        let collection_id = create_collection<u64>(
            creator,
            ASCII::string(b"Collection: Hello, World"),
            ASCII::string(b"Hello, World"),
            ASCII::string(b"https://aptos.dev"),
            Option::none(),
        );

        let token_id = create_token<u64>(
            creator,
            *&collection_id,
            ASCII::string(b"Token: Hello, Token"),
            ASCII::string(b"Hello, Token"),
            amount,
            ASCII::string(b"https://aptos.dev"),
            0,
        );

        (collection_id, token_id)
    }
}
