#[test_only]
module vote_lockup::voting_token_tests {
    use aptos_framework::fungible_asset;
    use aptos_framework::primary_fungible_store;
    use vote_lockup::test_helpers;
    use vote_lockup::voting_token;
    use std::signer;

    #[test(sender = @0x123, recipient = @0xdead)]
    fun test_e2e(sender: &signer, recipient: &signer) {
        test_helpers::set_up();
        let tokens = voting_token::mint(1000);
        assert!(fungible_asset::amount(&tokens) == 1000, 0);
        let sender_addr = signer::address_of(sender);
        primary_fungible_store::deposit(sender_addr, tokens);
        let token = voting_token::token();
        assert!(primary_fungible_store::balance(sender_addr, token) == 1000, 0);
        let recipient_addr = signer::address_of(recipient);
        primary_fungible_store::transfer(sender, token, recipient_addr, 500);
        assert!(primary_fungible_store::balance(sender_addr, token) == 500, 0);
        assert!(primary_fungible_store::balance(recipient_addr, token) == 500, 0);
        let tokens = primary_fungible_store::withdraw(recipient, token, 500);
        voting_token::burn(tokens);
        assert!(primary_fungible_store::balance(recipient_addr, token) == 0, 0);
        voting_token::transfer(
            primary_fungible_store::primary_store(sender_addr, token),
            primary_fungible_store::primary_store(recipient_addr, token),
            500,
        );
        assert!(primary_fungible_store::balance(sender_addr, token) == 0, 0);
        assert!(primary_fungible_store::balance(recipient_addr, token) == 500, 0);
        voting_token::disable_transfer(primary_fungible_store::primary_store(recipient_addr, token));
        assert!(primary_fungible_store::is_frozen(recipient_addr, token), 0);
    }
}
