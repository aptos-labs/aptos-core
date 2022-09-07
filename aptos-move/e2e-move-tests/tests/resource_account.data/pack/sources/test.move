module 0x0b6beee9bc1ad3177403a04efeefb1901c12b7b575ac5124c0205efc0dd2e32a::test {
    use std::error;
    use std::signer;
    use std::string;
    use aptos_framework::account;
    use aptos_framework::coin::{Self, BurnCapability, FreezeCapability, MintCapability};
    use aptos_framework::resource_account;
    use aptos_framework::account::exists_at;

    struct ModuleData has key {
        resource_signer_cap: account::SignerCapability,
    }

    struct TestCoin {}
    struct WrappedTestCoin {
        test_coin: TestCoin
    }

    /// Capabilities resource storing mint and burn capabilities.
    /// The resource is stored on the account that initialized coin `CoinType`.
    struct Capabilities<phantom CoinType> has key {
        burn_cap: BurnCapability<CoinType>,
        freeze_cap: FreezeCapability<CoinType>,
        mint_cap: MintCapability<CoinType>,
    }

    const RESOURCE_ADDR: address = @0x0b6beee9bc1ad3177403a04efeefb1901c12b7b575ac5124c0205efc0dd2e32a;
    const EACCOUNT_DOES_NOT_EXIST: u64 = 0;
    const EINSUFFICIENT_FUND: u64 = 1;

    fun init_module(account: &signer) {
        // retrieve the signer capability cap and store it within this module
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(account, @0xcafe);
        move_to(account, ModuleData { resource_signer_cap });

        // initialize coins and store their corresponding burn cap, freeze cap, and mint cap in the account
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<TestCoin>(
            account, string::utf8(b"test coin"), string::utf8(b"TEST"), 8, true
        );
        move_to(account, Capabilities<TestCoin>{
            burn_cap,
            freeze_cap,
            mint_cap,
        });

        let (wrapped_burn_cap, wrapped_freeze_cap, wrapped_mint_cap) = coin::initialize<WrappedTestCoin>(
            account, string::utf8(b"wrapped test coin"), string::utf8(b"WTEST"),8 , true
        );
        move_to(account, Capabilities<WrappedTestCoin>{
            burn_cap: wrapped_burn_cap,
            freeze_cap: wrapped_freeze_cap,
            mint_cap: wrapped_mint_cap,
        });
    }

    // swap wrapped test coin at the recipient's address with test coin at resource account's address
    // assume the value of one test coin == the value of one wrapped test coin
    fun swap_test_coin_with_wrapped_test_coin(recipient: &signer, amount_test_coin: u64) acquires ModuleData {
        let recipient_address = signer::address_of(recipient);
        assert!(exists_at(recipient_address), error::invalid_argument(EACCOUNT_DOES_NOT_EXIST));
        assert!(coin::balance<WrappedTestCoin>(recipient_address) > amount_test_coin, EINSUFFICIENT_FUND);
        assert!(coin::balance<TestCoin>(RESOURCE_ADDR) > amount_test_coin, EINSUFFICIENT_FUND);

        let module_data = borrow_global_mut<ModuleData>(RESOURCE_ADDR);
        let resource_signer = account::create_signer_with_capability(&module_data.resource_signer_cap);

        coin::transfer<WrappedTestCoin>(recipient, RESOURCE_ADDR, amount_test_coin);
        coin::transfer<TestCoin>(&resource_signer, signer::address_of(recipient), amount_test_coin);
    }
}
