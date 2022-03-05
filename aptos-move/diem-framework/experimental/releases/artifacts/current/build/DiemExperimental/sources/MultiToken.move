module ExperimentalFramework::MultiToken {
    use Std::Errors;
    use Std::Event;
    use Std::GUID;
    use Std::Option::{Self, Option};
    use Std::Signer;
    use Std::Vector;

    /// Struct representing data of a specific token with token_id,
    /// stored under the creator's address inside TokenInfoCollection.
    /// For each token_id, there is only one MultiTokenData.
    struct TokenData<TokenType: store> has key, store {
        metadata: Option<TokenType>,
        /// Identifier for the token.
        token_id: GUID::GUID,
        /// Pointer to where the content and metadata is stored.
        content_uri: vector<u8>,
        supply: u64,
    }

    /// A hot potato wrapper for the token's metadata. Since this wrapper has no `key` or `store`
    /// ability, it can't be stored in global storage. This wrapper can be safely passed outside
    /// of this module because we know it will have to come back to this module, where
    /// it will be unpacked.
    struct TokenDataWrapper<TokenType: store> {
        origin: address,
        index: u64,
        metadata: TokenType,
    }

    /// Struct representing a semi-fungible or non-fungible token (depending on the supply).
    /// There can be multiple tokens with the same id (unless supply is 1). Each token's
    /// corresponding token metadata is stored inside a MultiTokenData inside TokenDataCollection
    /// under the creator's address.
    struct Token<phantom TokenType: store> has key, store {
        id: GUID::ID,
        balance: u64,
    }

    struct MintEvent has copy, drop, store {
        id: GUID::ID,
        creator: address,
        content_uri: vector<u8>,
        amount: u64,
    }

    struct Admin has key {
        mint_events: Event::EventHandle<MintEvent>,
    }

    struct TokenDataCollection<TokenType: store> has key {
        tokens: vector<TokenData<TokenType>>,
    }

    spec fun get_tokens<TokenType>(addr: address): vector<TokenData<TokenType>>{
        global<TokenDataCollection<TokenType>>(addr).tokens
    }

    spec fun is_in_tokens<TokenType>(tokens: vector<TokenData<TokenType>>, token_id: GUID::ID): bool {
        exists token in tokens: token.token_id.id == token_id
    }

    spec fun find_token_index_by_id<TokenType>(tokens: vector<TokenData<TokenType>>, id: GUID::ID): u64 {
        choose min i in range(tokens) where tokens[i].token_id.id == id
    }

    const ADMIN: address = @0xa550c18;
    const MAX_U64: u64 = 18446744073709551615u64;
    // Error codes
    /// Function can only be called by the admin address
    const ENOT_ADMIN: u64  = 0;
    const EWRONG_TOKEN_ID: u64 = 1;
    const ETOKEN_BALANCE_OVERFLOWS: u64 = 2;
    const EAMOUNT_EXCEEDS_TOKEN_BALANCE: u64 = 3;
    const ETOKEN_EXTRACTED: u64 = 4;
    const EINDEX_EXCEEDS_LENGTH: u64 = 5;
    const ETOKEN_PRESENT: u64 = 6;

    /// Returns the id of given token
    public fun id<TokenType: store>(token: &Token<TokenType>): GUID::ID {
        *&token.id
    }

    /// Returns the balance of given token
    public fun balance<TokenType: store>(token: &Token<TokenType>): u64 {
        token.balance
    }

    public fun metadata<TokenType: store>(wrapper: &TokenDataWrapper<TokenType>): &TokenType {
        &wrapper.metadata
    }

    /// Returns the supply of tokens with `id` on the chain.
    public fun supply<TokenType: store>(id: &GUID::ID): u64 acquires TokenDataCollection {
        let owner_addr = GUID::id_creator_address(id);
        let tokens = &mut borrow_global_mut<TokenDataCollection<TokenType>>(owner_addr).tokens;
        let index_opt = index_of_token<TokenType>(tokens, id);
        assert!(Option::is_some(&index_opt), Errors::invalid_argument(EWRONG_TOKEN_ID));
        let index = Option::extract(&mut index_opt);
        Vector::borrow(tokens, index).supply
    }

    spec supply {
        let addr = GUID::id_creator_address(id);
        let token_collection = get_tokens<TokenType>(addr);
        let min_token_idx = find_token_index_by_id(token_collection,id);
        aborts_if !exists<TokenDataCollection<TokenType>>(addr);
        aborts_if !is_in_tokens(token_collection, id);
        ensures result == token_collection[min_token_idx].supply;
    }

    /// Extract the MultiToken data of the given token into a hot potato wrapper.
    public fun extract_token<TokenType: store>(nft: &Token<TokenType>): TokenDataWrapper<TokenType> acquires TokenDataCollection {
        let owner_addr = GUID::id_creator_address(&nft.id);
        let tokens = &mut borrow_global_mut<TokenDataCollection<TokenType>>(owner_addr).tokens;
        let index_opt = index_of_token<TokenType>(tokens, &nft.id);
        assert!(Option::is_some(&index_opt), Errors::invalid_argument(EWRONG_TOKEN_ID));
        let index = Option::extract(&mut index_opt);
        let item_opt = &mut Vector::borrow_mut(tokens, index).metadata;
        assert!(Option::is_some(item_opt), Errors::invalid_state(ETOKEN_EXTRACTED));
        TokenDataWrapper { origin: owner_addr, index, metadata: Option::extract(item_opt) }
    }

    spec extract_token {
        let addr = GUID::id_creator_address(nft.id);
        let token_collection = get_tokens<TokenType>(addr);
        let id = nft.id;
        let min_token_idx = find_token_index_by_id(token_collection, id);
        aborts_if !exists<TokenDataCollection<TokenType>>(addr);
        aborts_if token_collection[min_token_idx].metadata == Option::spec_none();
        aborts_if !is_in_tokens(token_collection, id);
        ensures result == TokenDataWrapper { origin: addr, index: min_token_idx,
        metadata: Option::borrow(token_collection[min_token_idx].metadata)};
        ensures get_tokens<TokenType>(addr)[min_token_idx].metadata == Option::spec_none();
    }

    /// Restore the token in the wrapper back into global storage under original address.
    public fun restore_token<TokenType: store>(wrapper: TokenDataWrapper<TokenType>) acquires TokenDataCollection {
        let TokenDataWrapper { origin, index, metadata } = wrapper;
        let tokens = &mut borrow_global_mut<TokenDataCollection<TokenType>>(origin).tokens;
        assert!(Vector::length(tokens) > index, EINDEX_EXCEEDS_LENGTH);
        let item_opt = &mut Vector::borrow_mut(tokens, index).metadata;
        assert!(Option::is_none(item_opt), ETOKEN_PRESENT);
        Option::fill(item_opt, metadata);
    }

    spec restore_token {
        let addr = wrapper.origin;
        let token_collection = get_tokens<TokenType>(addr);
        let item_opt = token_collection[wrapper.index].metadata;
        aborts_if !exists<TokenDataCollection<TokenType>>(addr);
        aborts_if len(token_collection) <= wrapper.index;
        aborts_if item_opt != Option::spec_none();
        ensures get_tokens<TokenType>(addr)[wrapper.index].metadata == Option::spec_some(wrapper.metadata);
    }

    /// Finds the index of token with the given id in the gallery.
    fun index_of_token<TokenType: store>(gallery: &vector<TokenData<TokenType>>, id: &GUID::ID): Option<u64> {
        let i = 0;
        let len = Vector::length(gallery);
        while ({spec {
            invariant i >= 0;
            invariant i <= len(gallery);
            invariant forall k in 0..i: gallery[k].token_id.id != id;
        };(i < len)}) {
            if (GUID::eq_id(&Vector::borrow(gallery, i).token_id, id)) {
                return Option::some(i)
            };
            i = i + 1;
        };
        Option::none()
    }

    spec index_of_token{
        let min_token_idx = find_token_index_by_id(gallery, id);
        let post res_id = Option::borrow(result);
        ensures is_in_tokens(gallery, id) <==> (Option::is_some(result) && res_id == min_token_idx);
        ensures result ==  Option::spec_none() <==> !is_in_tokens(gallery, id);
    }

    /// Join two multi tokens and return a multi token with the combined value of the two.
    public fun join<TokenType: store>(token: &mut Token<TokenType>, other: Token<TokenType>) {
        let Token { id, balance } = other;
        assert!(*&token.id == id, EWRONG_TOKEN_ID);
        assert!(MAX_U64 - token.balance >= balance, ETOKEN_BALANCE_OVERFLOWS);
        token.balance = token.balance + balance
    }

    spec join{
        aborts_if token.id != other.id with EWRONG_TOKEN_ID;
        aborts_if MAX_U64 - token.balance < other.balance with ETOKEN_BALANCE_OVERFLOWS;
        ensures token.balance == old(token).balance + other.balance;
    }

    /// Split the token into two tokens, one with balance `amount` and the other one with balance
    public fun split<TokenType: store>(token: Token<TokenType>, amount: u64): (Token<TokenType>, Token<TokenType>) {
        assert!(token.balance >= amount, EAMOUNT_EXCEEDS_TOKEN_BALANCE);
        token.balance = token.balance - amount;
        let id = *&token.id;
        (token,
        Token {
            id,
            balance: amount
        } )
    }

    spec split {
        aborts_if token.balance < amount with EAMOUNT_EXCEEDS_TOKEN_BALANCE;
        ensures result_1.balance == token.balance - amount;
        ensures result_2.balance == amount;
        ensures result_1.id == result_2.id;
    }

    /// Initialize this module, to be called in genesis.
    public fun initialize_multi_token(account: signer) {
        assert!(Signer::address_of(&account) == ADMIN, ENOT_ADMIN);
        move_to(&account, Admin {
            mint_events: Event::new_event_handle<MintEvent>(&account),
        })
    }

    spec initialize_multi_token{
        let addr = Signer::address_of(account);
        aborts_if addr != ADMIN;
        aborts_if exists<Admin>(addr);
        ensures exists<Admin>(addr);
    }

    /// Create a` TokenData<TokenType>` that wraps `metadata` and with balance of `amount`
    public fun create<TokenType: store>(
        account: &signer, metadata: TokenType, content_uri: vector<u8>, amount: u64
    ): Token<TokenType> acquires Admin, TokenDataCollection {
        let guid = GUID::create(account);
        Event::emit_event(
            &mut borrow_global_mut<Admin>(ADMIN).mint_events,
            MintEvent {
                id: GUID::id(&guid),
                creator: Signer::address_of(account),
                content_uri: copy content_uri,
                amount,
            }
        );
        let id = GUID::id(&guid);
        if (!exists<TokenDataCollection<TokenType>>(Signer::address_of(account))) {
            move_to(account, TokenDataCollection { tokens: Vector::empty<TokenData<TokenType>>() });
        };
        let token_data_collection = &mut borrow_global_mut<TokenDataCollection<TokenType>>(Signer::address_of(account)).tokens;
        Vector::push_back(
            token_data_collection,
            TokenData { metadata: Option::some(metadata), token_id: guid, content_uri, supply: amount }
        );
        Token { id, balance: amount }
    }

    spec create {
        let addr = Signer::address_of(account);
        let post post_tokens = get_tokens<TokenType>(addr);

        aborts_if !exists<Admin>(ADMIN);
        aborts_if exists<GUID::Generator>(addr) && global<GUID::Generator>(addr).counter + 1 > MAX_U64;

        ensures result.balance == amount;
        ensures GUID::id_creator_address(result.id) == addr;
        ensures exists<TokenDataCollection<TokenType>>(addr);
        ensures post_tokens[len(post_tokens) - 1] ==
                TokenData<TokenType> {metadata: Option::spec_some(metadata), token_id: GUID::GUID {id: result.id}, content_uri, supply:amount};
    }

}
