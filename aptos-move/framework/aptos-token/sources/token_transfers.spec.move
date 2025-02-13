spec aptos_token::token_transfers {
    spec module {
        pragma verify = false;
        pragma aborts_if_is_strict;
    }

    spec initialize_token_transfers(account: &signer) {
        include InitializeTokenTransfersAbortsIf;
    }

    /// Abort according to the code
    spec schema InitializeTokenTransfersAbortsIf {
        use aptos_framework::account::{Account};
        account: &signer;

        let addr = signer::address_of(account);
        aborts_if exists<PendingClaims>(addr);
        let account = global<Account>(addr);
        aborts_if !exists<Account>(addr);
        aborts_if account.guid_creation_num + 3 >= account::MAX_GUID_CREATION_NUM;
        aborts_if account.guid_creation_num + 3 > MAX_U64;
    }

    spec create_token_offer_id(to_addr: address, token_id: TokenId): TokenOfferId {
        aborts_if false;
    }

    spec offer_script(
        sender: signer,
        receiver: address,
        creator: address,
        collection: String,
        name: String,
        property_version: u64,
        amount: u64,
    ){
        pragma verify = false;
        let token_id = token::create_token_id_raw(creator, collection, name, property_version);
    }

    spec offer(
        sender: &signer,
        receiver: address,
        token_id: TokenId,
        amount: u64,
    ){
        use aptos_token::token::{TokenStore,Self};

        // TODO: Can't get the return from `withdraw_token`.
        pragma verify = false;

        let sender_addr = signer::address_of(sender);
        include !exists<PendingClaims>(sender_addr) ==> InitializeTokenTransfersAbortsIf{account : sender};
        let pending_claims = global<PendingClaims>(sender_addr).pending_claims;
        let token_offer_id = create_token_offer_id(receiver, token_id);

        let tokens = global<TokenStore>(sender_addr).tokens;
        aborts_if amount <= 0;
        aborts_if token::spec_balance_of(sender_addr, token_id) < amount;
        aborts_if !exists<TokenStore>(sender_addr);
        aborts_if !table::spec_contains(tokens, token_id);

        aborts_if !table::spec_contains(pending_claims, token_offer_id);
        let a = table::spec_contains(pending_claims, token_offer_id);
        let dst_token = table::spec_get(pending_claims, token_offer_id);

        aborts_if dst_token.amount + spce_get(signer::address_of(sender), token_id, amount) > MAX_U64;
    }

    /// Get the amount from sender token
    spec fun spce_get(
        account_addr: address,
        id: TokenId,
        amount: u64
    ): u64 {
        use aptos_token::token::{TokenStore};
        use aptos_std::table::{Self};
        let tokens = global<TokenStore>(account_addr).tokens;
        let balance = table::spec_get(tokens, id).amount;
        if (balance > amount) {
            amount
        } else {
            table::spec_get(tokens, id).amount
        }
    }

    spec claim_script(
        receiver: signer,
        sender: address,
        creator: address,
        collection: String,
        name: String,
        property_version: u64,
    ){
        use aptos_token::token::{TokenStore};

        // TODO: deposit_token has pending issues
        pragma aborts_if_is_partial;

        let token_id = token::create_token_id_raw(creator, collection, name, property_version);
        aborts_if !exists<PendingClaims>(sender);
        let pending_claims = global<PendingClaims>(sender).pending_claims;
        let token_offer_id = create_token_offer_id(signer::address_of(receiver), token_id);
        aborts_if !table::spec_contains(pending_claims, token_offer_id);
        let tokens = table::spec_get(pending_claims, token_offer_id);

        include token::InitializeTokenStore{account: receiver };

        let account_addr = signer::address_of(receiver);
        let token = tokens;
        let token_store = global<TokenStore>(account_addr);
        let recipient_token = table::spec_get(token_store.tokens, token.id);
        let b = table::spec_contains(token_store.tokens, token.id);
        aborts_if token.amount <= 0;

    }

    spec claim(
        receiver: &signer,
        sender: address,
        token_id: TokenId,
    ){
        use aptos_token::token::{TokenStore};
        // TODO: deposit_token has pending issues
        pragma aborts_if_is_partial;

        aborts_if !exists<PendingClaims>(sender);
        let pending_claims = global<PendingClaims>(sender).pending_claims;
        let token_offer_id = create_token_offer_id(signer::address_of(receiver), token_id);
        aborts_if !table::spec_contains(pending_claims, token_offer_id);
        let tokens = table::spec_get(pending_claims, token_offer_id);

        include token::InitializeTokenStore{account: receiver };

        let account_addr = signer::address_of(receiver);
        let token = tokens;
        let token_store = global<TokenStore>(account_addr);
        let recipient_token = table::spec_get(token_store.tokens, token.id);
        let b = table::spec_contains(token_store.tokens, token.id);
        aborts_if token.amount <= 0;
    }

    spec cancel_offer_script(
        sender: signer,
        receiver: address,
        creator: address,
        collection: String,
        name: String,
        property_version: u64,
    ){
        use aptos_token::token::{TokenStore};

        // TODO: deposit_token has pending issues.
        pragma aborts_if_is_partial;

        let token_id = token::create_token_id_raw(creator, collection, name, property_version);

        let sender_addr = signer::address_of(sender);
        aborts_if !exists<PendingClaims>(sender_addr);
        let pending_claims = global<PendingClaims>(sender_addr).pending_claims;
        let token_offer_id = create_token_offer_id(receiver, token_id);
        aborts_if !table::spec_contains(pending_claims, token_offer_id);

        include token::InitializeTokenStore{account: sender };
        let dst_token = table::spec_get(pending_claims, token_offer_id);

        let account_addr = sender_addr;
        let token = dst_token;
        let token_store = global<TokenStore>(account_addr);
        let recipient_token = table::spec_get(token_store.tokens, token.id);
        let b = table::spec_contains(token_store.tokens, token.id);
        aborts_if token.amount <= 0;
    }

    spec cancel_offer(
        sender: &signer,
        receiver: address,
        token_id: TokenId,
    ){
        use aptos_token::token::{TokenStore};

        // TODO: deposit_token has pending issues.
        pragma aborts_if_is_partial;

        let sender_addr = signer::address_of(sender);
        aborts_if !exists<PendingClaims>(sender_addr);
        let pending_claims = global<PendingClaims>(sender_addr).pending_claims;
        let token_offer_id = create_token_offer_id(receiver, token_id);
        aborts_if !table::spec_contains(pending_claims, token_offer_id);

        include token::InitializeTokenStore{account: sender };
        let dst_token = table::spec_get(pending_claims, token_offer_id);

        let account_addr = sender_addr;
        let token = dst_token;
        let token_store = global<TokenStore>(account_addr);
        let recipient_token = table::spec_get(token_store.tokens, token.id);
        let b = table::spec_contains(token_store.tokens, token.id);
        aborts_if token.amount <= 0;
    }
}
