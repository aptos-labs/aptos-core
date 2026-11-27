spec aptos_token::token_transfers {
    spec module {
        pragma aborts_if_is_strict;
    }

    spec initialize_token_transfers(account: &signer) {
        include InitializeTokenTransfersAbortsIf;
        ensures exists<PendingClaims>(signer::address_of(account));
    }

    /// Creating the three event handles bumps the account's GUID counter three times.
    spec schema InitializeTokenTransfersAbortsIf {
        use aptos_framework::account::{Account};
        account: &signer;

        let addr = signer::address_of(account);
        aborts_if exists<PendingClaims>(addr);
        include account::NewEventHandleAbortsIf;
        let creation_num = if (exists<Account>(addr)) { global<Account>(addr).guid_creation_num } else { 2 };
        aborts_if creation_num + 3 >= account::MAX_GUID_CREATION_NUM;
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
        include token::CreateTokenDataIdAbortsIf;
        let token_id = token::create_token_id_raw(creator, collection, name, property_version);
        include OfferAbortsIf { sender };
        include OfferEnsures { sender };
    }

    spec offer(
        sender: &signer,
        receiver: address,
        token_id: TokenId,
        amount: u64,
    ){
        include OfferAbortsIf;
        include OfferEnsures;
    }

    spec schema OfferAbortsIf {
        use aptos_token::token::{TokenStore};
        sender: signer;
        receiver: address;
        token_id: TokenId;
        amount: u64;

        let sender_addr = signer::address_of(sender);
        include !exists<PendingClaims>(sender_addr) ==> InitializeTokenTransfersAbortsIf { account: sender };
        include token::WithdrawWithEventInternalAbortsIf { account_addr: sender_addr, id: token_id };
        // Withdrawing the whole entry moves the stored token, which carries its own id.
        let stored = table::spec_get(global<TokenStore>(sender_addr).tokens, token_id);
        let moved_id = if (stored.amount > amount) { token_id } else { stored.id };
        // A freshly created PendingClaims is empty, so only an existing offer is merged into.
        let pending_claims = global<PendingClaims>(sender_addr).pending_claims;
        let token_offer_id = create_token_offer_id(receiver, token_id);
        let merging = exists<PendingClaims>(sender_addr) && table::spec_contains(pending_claims, token_offer_id);
        let dst_token = table::spec_get(pending_claims, token_offer_id);
        aborts_if merging && dst_token.id != moved_id;
        aborts_if merging && dst_token.amount + amount > MAX_U64;
    }

    spec schema OfferEnsures {
        sender: signer;
        receiver: address;
        token_id: TokenId;
        amount: u64;

        let sender_addr = signer::address_of(sender);
        let token_offer_id = create_token_offer_id(receiver, token_id);
        let pending_claims = global<PendingClaims>(sender_addr).pending_claims;
        let merging = exists<PendingClaims>(sender_addr) && table::spec_contains(pending_claims, token_offer_id);
        let post post_pending_claims = global<PendingClaims>(sender_addr).pending_claims;
        ensures token::spec_balance_of(sender_addr, token_id)
            == old(token::spec_balance_of(sender_addr, token_id)) - amount;
        ensures table::spec_contains(post_pending_claims, token_offer_id);
        ensures table::spec_get(post_pending_claims, token_offer_id).amount
            == if (merging) { table::spec_get(pending_claims, token_offer_id).amount + amount } else { amount };
    }

    spec claim_script(
        receiver: signer,
        sender: address,
        creator: address,
        collection: String,
        name: String,
        property_version: u64,
    ){
        include token::CreateTokenDataIdAbortsIf;
        let token_id = token::create_token_id_raw(creator, collection, name, property_version);
        include TakeOfferAbortsIfAndEnsures {
            owner: sender,
            to_addr: signer::address_of(receiver),
            account: receiver,
        };
    }

    spec claim(
        receiver: &signer,
        sender: address,
        token_id: TokenId,
    ){
        include TakeOfferAbortsIfAndEnsures {
            owner: sender,
            to_addr: signer::address_of(receiver),
            account: receiver,
        };
    }

    spec cancel_offer_script(
        sender: signer,
        receiver: address,
        creator: address,
        collection: String,
        name: String,
        property_version: u64,
    ){
        include token::CreateTokenDataIdAbortsIf;
        let token_id = token::create_token_id_raw(creator, collection, name, property_version);
        include TakeOfferAbortsIfAndEnsures {
            owner: signer::address_of(sender),
            to_addr: receiver,
            account: sender,
        };
    }

    spec cancel_offer(
        sender: &signer,
        receiver: address,
        token_id: TokenId,
    ){
        include TakeOfferAbortsIfAndEnsures {
            owner: signer::address_of(sender),
            to_addr: receiver,
            account: sender,
        };
    }

    /// Removes the offer of `owner` to `to_addr` and deposits its token into `account`.
    spec schema TakeOfferAbortsIfAndEnsures {
        use aptos_token::token::{TokenStore};
        owner: address;
        to_addr: address;
        token_id: TokenId;
        account: signer;

        aborts_if !exists<PendingClaims>(owner);
        let pending_claims = global<PendingClaims>(owner).pending_claims;
        let token_offer_id = create_token_offer_id(to_addr, token_id);
        aborts_if !table::spec_contains(pending_claims, token_offer_id);
        let token = table::spec_get(pending_claims, token_offer_id);
        let account_addr = signer::address_of(account);
        aborts_if token.amount <= 0;
        include token::InitializeTokenStore;
        include exists<TokenStore>(account_addr) ==> token::DirectDepositAbortsIf {
            account_addr,
            token_id: token.id,
            token_amount: token.amount,
        };
        ensures !table::spec_contains(global<PendingClaims>(owner).pending_claims, token_offer_id);
        ensures token::spec_balance_of(account_addr, token.id)
            == old(token::spec_balance_of(account_addr, token.id)) + token.amount;
    }
}
