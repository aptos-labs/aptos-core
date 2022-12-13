/// TokenStore holds opaque references to Tokens via TokenRef. An application should be able to
/// convert any type (container) of TokenRef to a base TokenRef and then store it here. An
/// application can also remove the TokenRef and convert it back into the appropriate type
/// (container).
///
/// @TODO(davidiw): add transfer, borrow semantics, and other useful helpers
module nfts_as_accounts::token_store {
    use std::signer;
    use aptos_std::table;

    use nfts_as_accounts::token;

    struct TokenStore has key {
        inner: table::Table<address, token::TokenRef<token::BaseToken>>,
    }

    public fun init(account: &signer) {
        move_to(account, TokenStore { inner: table::new() })
    }

    public fun take<Data: store>(
        account: &signer,
        addr: address,
    ): token::TokenRef<Data> acquires TokenStore {
        let account_addr = signer::address_of(account);
        let token_store = borrow_global_mut<TokenStore>(account_addr);
        let ref = table::remove(&mut token_store.inner, addr);
        token::from_base_token(ref)
    }

    public fun store<Data: store>(
        account: &signer,
        ref: token::TokenRef<Data>,
    ) acquires TokenStore {
        let account_addr = signer::address_of(account);
        let token_store = borrow_global_mut<TokenStore>(account_addr);
        let token_addr = token::token_addr_from_ref(&ref);
        let base_ref = token::to_base_token(ref);
        table::add(&mut token_store.inner, token_addr, base_ref);
    }
}
