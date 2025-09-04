/// This module demonstrates how to create a new coin and build simple defi swap functions for the new coin
/// using a resource account.
///
/// - Initialization of this module
/// Let's say we have an original account at address `0xcafe`. We can use it to call
/// `create_resource_account_and_publish_package(origin, vector::empty<>(), ...)` - this will create a resource account at
/// address `0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5`. The module `simple_defi` will be published
/// under the resource account's address.
///
/// - The basic flow
/// (1) call create_resource_account_and_publish_package() to publish this module under the resource account's address.
/// init_module() will be called with resource account's signer as part of publishing the package.
/// - In init_module(), we do two things: first, we create the new coin; secondly, we store the resource account's signer capability
/// and the coin's mint and burn capabilities within `ModuleData`. Storing the signer capability allows the module to programmatically
/// sign transactions without needing a private key
/// (2) when exchanging coins, we call `exchange_to` to swap `VelorCoin` to `ChloesCoin`, and `exchange_from` to swap `VelorCoin` from `ChloesCoin`
module resource_account::simple_defi {
    use std::signer;
    use std::string;

    use velor_framework::account;
    use velor_framework::coin::{Self, Coin, MintCapability, BurnCapability};
    use velor_framework::resource_account;
    use velor_framework::velor_coin::{VelorCoin};

    struct ModuleData has key {
        resource_signer_cap: account::SignerCapability,
        burn_cap: BurnCapability<ChloesCoin>,
        mint_cap: MintCapability<ChloesCoin>,
    }

    struct ChloesCoin {
        velor_coin: VelorCoin
    }

    /// initialize the module and store the signer cap, mint cap and burn cap within `ModuleData`
    fun init_module(account: &signer) {
        // store the capabilities within `ModuleData`
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(account, @source_addr);
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<ChloesCoin>(
            account,
            string::utf8(b"Chloe's Coin"),
            string::utf8(b"CCOIN"),
            8,
            false,
        );
        move_to(account, ModuleData {
            resource_signer_cap,
            burn_cap,
            mint_cap,
        });

        // destroy freeze cap because we aren't using it
        coin::destroy_freeze_cap(freeze_cap);

        // regsiter the resource account with both coins so it has a CoinStore to store those coins
        coin::register<VelorCoin>(account);
        coin::register<ChloesCoin>(account);
    }

    /// Exchange VelorCoin to ChloesCoin
    public fun exchange_to(a_coin: Coin<VelorCoin>): Coin<ChloesCoin> acquires ModuleData {
        let coin_cap = borrow_global_mut<ModuleData>(@resource_account);
        let amount = coin::value(&a_coin);
        coin::deposit(@resource_account, a_coin);
        coin::mint<ChloesCoin>(amount, &coin_cap.mint_cap)
    }

    /// Exchange ChloesCoin to VelorCoin
    public fun exchange_from(c_coin: Coin<ChloesCoin>): Coin<VelorCoin> acquires ModuleData {
        let amount = coin::value(&c_coin);
        let coin_cap = borrow_global_mut<ModuleData>(@resource_account);
        coin::burn<ChloesCoin>(c_coin, &coin_cap.burn_cap);

        let module_data = borrow_global_mut<ModuleData>(@resource_account);
        let resource_signer = account::create_signer_with_capability(&module_data.resource_signer_cap);
        coin::withdraw<VelorCoin>(&resource_signer, amount)
    }

    /// Entry function version of exchange_to() for e2e tests only
    public entry fun exchange_to_entry(account: &signer, amount: u64) acquires ModuleData {
        let a_coin = coin::withdraw<VelorCoin>(account, amount);
        let c_coin = exchange_to(a_coin);

        coin::register<ChloesCoin>(account);
        coin::deposit(signer::address_of(account), c_coin);
    }

    /// Entry function version of exchange_from() for e2e tests only
    public entry fun exchange_from_entry(account: &signer, amount: u64) acquires ModuleData {
        let c_coin = coin::withdraw<ChloesCoin>(account, amount);
        let a_coin = exchange_from(c_coin);

        coin::deposit(signer::address_of(account), a_coin);
    }

    #[test_only]
    public entry fun set_up_test(origin_account: &signer, resource_account: &signer) {
        use std::vector;

        account::create_account_for_test(signer::address_of(origin_account));

        // create a resource account from the origin account, mocking the module publishing process
        resource_account::create_resource_account(origin_account, vector::empty<u8>(), vector::empty<u8>());
        init_module(resource_account);
    }

    #[test(origin_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, framework = @velor_framework)]
    public entry fun test_exchange_to_and_exchange_from(origin_account: signer, resource_account: signer, framework: signer) acquires ModuleData {
        use velor_framework::velor_coin;

        let (velor_coin_burn_cap, velor_coin_mint_cap) = velor_coin::initialize_for_test(&framework);
        set_up_test(&origin_account, &resource_account);

        // exchange from 5 velor coins to 5 chloe's coins & assert the results are expected
        let five_a_coins = coin::mint(5, &velor_coin_mint_cap);
        let c_coins = exchange_to(five_a_coins);
        assert!(coin::value(&c_coins) == 5, 0);
        assert!(coin::balance<VelorCoin>(signer::address_of(&resource_account)) == 5, 1);
        assert!(coin::balance<ChloesCoin>(signer::address_of(&resource_account)) == 0, 2);

        // exchange from 5 chloe's coins to 5 velor coins & assert the results are expected
        let a_coins = exchange_from(c_coins);
        assert!(coin::value(&a_coins) == 5, 0);
        assert!(coin::balance<VelorCoin>(signer::address_of(&resource_account)) == 0, 3);
        assert!(coin::balance<ChloesCoin>(signer::address_of(&resource_account)) == 0, 4);

        // burn the remaining coins & destroy the capabilities since they aren't droppable
        coin::burn(a_coins, &velor_coin_burn_cap);
        coin::destroy_mint_cap<VelorCoin>(velor_coin_mint_cap);
        coin::destroy_burn_cap<VelorCoin>(velor_coin_burn_cap);
    }
}
