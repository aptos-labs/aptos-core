/// The base Token type and partners. In this model, a Token is stored as a resource in a Token
/// Account. Hence each token is a distinct account.
/// Some properties:
///    * Tokens are defined by the tuple (creator address, collection, name).
///    * Names can be changed after creation, because the account address has been decoupled. This
///      is similar to how authentication keys work for regular aptos accounts.
///    * Each Token is stored at an account as a top-level resource.
///    * Additional token metadata can also be stored as other resources in this account.
///    * When creating a Token, a TokenRef is returned for managing that Token.
///    * The TokenRef is what offers composability of Tokens, since a Token can contain other
///       TokenRefs.
///    * The creator continues to be the "manager" of the token even after creation.
///    * A Token is globally accessible and cannot be hidden, thus a creator always has access to
///       the token to mutate it as appropriate.
///
/// Some notes:
/// * This module doesn't have clean error handling because some operations are not cheap. It is
///   more effective to just let the upstream abort happen, rather than try to handle it locally,
///   unless there's a means to recover.
///
/// TODO(@davidiw):
/// * add appropriate set of accessors for fields
/// * consider tracking ownership if the token is inserted into a TokenStore
/// * create_token should probably have another function that returns the signer for descendants
module nfts_as_accounts::token {
    use std::option::{Option, Self};
    use std::signer;
    use std::string::{Self, String};
    use std::vector;

    use aptos_framework::account;

    const ENOT_THE_CREATOR: u64 = 1;
    const EINVALID_REF_CONVERSION: u64 = 2;

    /// Represents the common fields to all tokens.
    struct Token<Data> has key {
        /// An optional categorization of similar token, there are no constraints on collections.
        collection: String,
        /// The original creator of this token.
        creator: address,
        /// A brief description of the token.
        description: String,
        /// Determines which fields are mutable.
        mutability_config: MutabilityConfig,
        /// The name of the token, which should be unique within the collection; the length of name
        ///should be smaller than 128, characters, eg: "Aptos Animal #1234"
        name: String,
        /// The denominator and numerator for calculating the royalty fee; it also contains payee
        /// account address for depositing the Royalty
        royalty: Royalty,
        /// The Uniform Resource Identifier (uri) pointing to the JSON file stored in off-chain
        /// storage; the URL length will likely need a maximum any suggestions?
        uri: String,
        /// Inner data
        data: Option<Data>,
    }

    /// This config specifies which fields in the TokenData are mutable
    struct MutabilityConfig has copy, drop, store {
        description: bool,
        name: bool,
        uri: bool,
    }

    /// The royalty of a token
    struct Royalty has copy, drop, store {
        numerator: u64,
        denominator: u64,
        /// The recipient of royalty payments. See the `shared_account` for how to handle multiple
        /// creators.
        payee_address: address,
    }

    struct TokenRef<phantom Data> has store {
        inner: account::SignerCapability,
    }

    struct BaseToken has store { }

    public fun create_token<Data: store>(
        creator: &signer, 
        collection: String,
        description: String,
        mutability_config: MutabilityConfig,
        name: String,
        royalty: Royalty,
        uri: String,
        data: Data,
    ): TokenRef<Data> {
        let seed = *string::bytes(&collection);
        vector::append(&mut seed, b"::");
        vector::append(&mut seed, *string::bytes(&name));
        // To keep costs down, this function does not check to see if the token already exists
        let (token_account, inner) = account::create_resource_account(creator, seed);
        let token = Token {
            collection,
            creator: signer::address_of(creator),
            description,
            mutability_config,
            name,
            royalty,
            uri,
            data: option::some(data),
        };

        move_to(&token_account, token);
        TokenRef<Data> { inner }
    }

    public fun create_mutability_config(description: bool, name: bool, uri: bool): MutabilityConfig {
        MutabilityConfig {
            description,
            name,
            uri,
        }
    }

    public fun create_royalty(numerator: u64, denominator: u64, payee_address: address): Royalty {
        Royalty {
            numerator,
            denominator,
            payee_address,
        }
    }

    public fun generate_token_address(creator: address, collection: &String, name: &String): address {
        let seed = *string::bytes(collection);
        vector::append(&mut seed, b"::");
        vector::append(&mut seed, *string::bytes(name));
        account::create_resource_address(&creator, seed)
    }

    public fun to_base_token<Data>(ref: TokenRef<Data>): TokenRef<BaseToken> {
        let TokenRef<Data>{ inner: inner } = ref;
        TokenRef<BaseToken> { inner }
    }

    public fun from_base_token<Data: store>(ref: TokenRef<BaseToken>): TokenRef<Data> {
        let addr = token_addr_from_ref(&ref);
        assert!(exists<Token<Data>>(addr), EINVALID_REF_CONVERSION);
        let TokenRef<BaseToken>{ inner: inner } = ref;
        TokenRef<Data> { inner }
    }

    public fun token_addr_from_ref<Data>(token_ref: &TokenRef<Data>): address {
        account::get_signer_capability_address(&token_ref.inner)
    }

    public fun token_signer<Data: store>(
        creator: &signer,
        token_ref: &TokenRef<Data>,
    ): signer acquires Token {
        let token = borrow_global<Token<Data>>(token_addr_from_ref(token_ref));
        assert!(token.creator == signer::address_of(creator), ENOT_THE_CREATOR);
        account::create_signer_with_capability(&token_ref.inner)
    }

    public fun set_data<Data: store>(ref: &TokenRef<Data>, data: Data) acquires Token {
        let token = borrow_global_mut<Token<Data>>(token_addr_from_ref(ref));
        option::fill(&mut token.data, data)
    }

    public fun take_data<Data: store>(ref: &TokenRef<Data>): Data acquires Token {
        let token = borrow_global_mut<Token<Data>>(token_addr_from_ref(ref));
        option::extract(&mut token.data)
    }

    #[test(account = @0x3)]
    fun test_creation(account: &signer) {
        let collection = string::utf8(b"Collection");
        let mutability_config = create_mutability_config(false, false, false);
        let name = string::utf8(b"Name");

        let account_addr = signer::address_of(account);
        let royalty = create_royalty(0, 0, account_addr);

        let token_addr = generate_token_address(account_addr, &collection, &name);

        let token_ref = create_token(
            account,
            collection,
            string::utf8(b"Description"),
            mutability_config,
            name,
            royalty,
            string::utf8(b"Uri"),
            BaseToken { },
        );

        assert!(token_addr == account::get_signer_capability_address(&token_ref.inner), 1);

        let TokenRef<BaseToken> { inner: _inner } = token_ref;
    }

    // The same token cannot be created twice, there are no duplicates.
    #[test(account = @0x3)]
    #[expected_failure(abort_code = 0x8000F, location = 0x1::account)]
    fun test_creation_twice(account: &signer) {
        let collection = string::utf8(b"Collection");
        let mutability_config = create_mutability_config(false, false, false);
        let name = string::utf8(b"Name");

        let account_addr = signer::address_of(account);
        let royalty = create_royalty(0, 0, account_addr);

        let token_ref_one = create_token(
            account,
            *&collection,
            string::utf8(b"Description"),
            *&mutability_config,
            *&name,
            *&royalty,
            string::utf8(b"Uri"),
            BaseToken { },
        );

        let token_ref_two = create_token(
            account,
            collection,
            string::utf8(b"Description"),
            mutability_config,
            name,
            royalty,
            string::utf8(b"Uri"),
            BaseToken { },
        );

        let TokenRef { inner: _inner } = token_ref_one;
        let TokenRef { inner: _inner } = token_ref_two;
    }
}
