module resource_account::resource_account {
    use std::signer;
    use std::string;

    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin, MintCapability, BurnCapability};
    use aptos_framework::resource_account;
    use aptos_framework::aptos_coin::{AptosCoin};

    struct ModuleData has key {
        resource_signer_cap: account::SignerCapability,
    }

    struct CoinCapabilities has key {
        burn_cap: BurnCapability<WrappedAptosCoin>,
        mint_cap: MintCapability<WrappedAptosCoin>,
    }

    struct WrappedAptosCoin {
        aptos_coin: AptosCoin
    }

    const EACCOUNT_DOES_NOT_EXIST: u64 = 0;
    const EINSUFFICIENT_FUND: u64 = 1;
    const EINVALID_SIGNER: u64 = 2;
    const EINVALID_COIN_VALUE: u64 = 3;
    const EINBALANCED_COIN_EXCHANGE: u64 = 4;

    fun init_module(account: &signer) {
        // get the resource signer
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(account, @0xcafe);
        let resource_signer = account::create_signer_with_capability(&resource_signer_cap);

        let (wrapped_burn_cap, freeze_cap, wrapped_mint_cap) = coin::initialize<WrappedAptosCoin>(
            &resource_signer,
            string::utf8(b"Wrapepd Aptos Coin"),
            string::utf8(b"WAPT"),
            8,
            false,
        );

        coin::destroy_freeze_cap(freeze_cap);

        coin::register<AptosCoin>(account);
        coin::register<WrappedAptosCoin>(account);

        move_to(account, ModuleData {
            resource_signer_cap,
        });

        move_to(account, CoinCapabilities {
            burn_cap: wrapped_burn_cap,
            mint_cap: wrapped_mint_cap,
        });
    }

    // Exchange AptosCoin to WrappedAptosCoin
    public fun exchange_to(coin: Coin<AptosCoin>): Coin<WrappedAptosCoin> acquires CoinCapabilities {
        let coin_cap = borrow_global_mut<CoinCapabilities>(@resource_account);
        let amount = coin::value(&coin);
        coin::deposit(@resource_account, coin);
        coin::mint<WrappedAptosCoin>(amount, &coin_cap.mint_cap)
    }

    // Exchange WrappedAptosCoin to AptosCoin
    public fun exchange_from(coin: Coin<WrappedAptosCoin>): Coin<AptosCoin> acquires ModuleData, CoinCapabilities {
        let amount = coin::value(&coin);
        let coin_cap = borrow_global_mut<CoinCapabilities>(@resource_account);
        coin::burn<WrappedAptosCoin>(coin, &coin_cap.burn_cap);

        let module_data = borrow_global_mut<ModuleData>(@resource_account);
        let resource_signer = account::create_signer_with_capability(&module_data.resource_signer_cap);
        coin::withdraw<AptosCoin>(&resource_signer, amount)
    }

    // Entry function version of exchange_to() for e2e tests
    public entry fun exchange_to_entry(account: &signer, amount: u64) acquires CoinCapabilities {
        let coin = coin::withdraw<AptosCoin>(account, amount);
        let wrapped_aptos_coin = exchange_to(coin);

        coin::register<WrappedAptosCoin>(account);
        coin::deposit(signer::address_of(account), wrapped_aptos_coin);
    }

    // Entry function version of exchange_from() for e2e tests
    public entry fun exchange_from_entry(account: &signer, amount: u64) acquires ModuleData, CoinCapabilities {
        let coin = coin::withdraw<WrappedAptosCoin>(account, amount);
        let aptos_coin = exchange_from(coin);

        coin::deposit(signer::address_of(account), aptos_coin);
    }

    #[test_only]
    public entry fun set_up_test(origin_account: &signer, resource_account: &signer) {
        use std::vector;

        account::create_account_for_test(signer::address_of(origin_account));

        // create a resource account from the origin account, mocking the module publishing process
        resource_account::create_resource_account(origin_account, vector::empty<u8>(), vector::empty<u8>());
        init_module(resource_account);
    }

    #[test(origin_account = @0xcafe, resource_account = @0x0b6beee9bc1ad3177403a04efeefb1901c12b7b575ac5124c0205efc0dd2e32a, framework = @aptos_framework)]
    public entry fun test_exchange_to_and_exchange_from(origin_account: signer, resource_account: signer, framework: signer) acquires ModuleData, CoinCapabilities {
        use aptos_framework::aptos_coin;

        set_up_test(&origin_account, &resource_account);
        let (aptos_coin_burn_cap, aptos_coin_mint_cap) = aptos_coin::initialize_for_test(&framework);

        // exchange from 5 aptos coins to 5 wrapped aptos coins & assert the results are expected
        let five_aptos_coins = coin::mint(5, &aptos_coin_mint_cap);
        let wrapped_coins = exchange_to(five_aptos_coins);
        assert!(coin::value(&wrapped_coins) == 5, 0);
        assert!(coin::balance<AptosCoin>(signer::address_of(&resource_account)) == 5, 1);
        assert!(coin::balance<WrappedAptosCoin>(signer::address_of(&resource_account)) == 0, 2);

        // exchange from 5 wrapped aptos coins to 5 aptos coins & assert the results are expected
        let aptos_coins = exchange_from(wrapped_coins);
        assert!(coin::value(&aptos_coins) == 5, 0);
        assert!(coin::balance<AptosCoin>(signer::address_of(&resource_account)) == 0, 3);
        assert!(coin::balance<WrappedAptosCoin>(signer::address_of(&resource_account)) == 0, 4);

        // burn the remaining coins & destroy the capabilities since they aren't droppable
        coin::burn(aptos_coins, &aptos_coin_burn_cap);
        coin::destroy_mint_cap<AptosCoin>(aptos_coin_mint_cap);
        coin::destroy_burn_cap<AptosCoin>(aptos_coin_burn_cap);
    }
}
