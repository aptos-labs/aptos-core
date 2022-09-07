module resource_account::resource_account {
    use std::error;

    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::resource_account;
    use aptos_framework::aptos_coin::AptosCoin;

    struct ModuleData has key {
        resource_signer_cap: account::SignerCapability,
    }

    struct WrappedAptosCoin {
        aptos_coin: AptosCoin
    }

    const EACCOUNT_DOES_NOT_EXIST: u64 = 0;
    const EINSUFFICIENT_FUND: u64 = 1;
    const EINVALID_SIGNER: u64 = 2;

    fun init_module(account: &signer) {
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(account, @0xcafe);
        move_to(account, ModuleData { resource_signer_cap });
    }

    // Swap an AptosCoin with a WrappedAptosCoin
    public fun swap_to_wrapped_aptos_coin(coin: Coin<AptosCoin>): (Coin<AptosCoin>, Coin<WrappedAptosCoin>) acquires ModuleData {
        assert!(coin::balance<WrappedAptosCoin>(@resource_account) >= 1, error::out_of_range(EINSUFFICIENT_FUND));

        let one_aptos_coin = coin::extract(&mut coin, 1);
        coin::deposit<AptosCoin>(@resource_account, one_aptos_coin);

        let module_data = borrow_global_mut<ModuleData>(@resource_account);
        let resource_signer = account::create_signer_with_capability(&module_data.resource_signer_cap);

        (coin, coin::withdraw<WrappedAptosCoin>(&resource_signer, 1))
    }

    // Swap a WrappedAptosCoin with an AptosCoin
    public fun swap_to_aptos_coin(coin: Coin<WrappedAptosCoin>): (Coin<WrappedAptosCoin>, Coin<AptosCoin>) acquires ModuleData {
        assert!(coin::balance<AptosCoin>(@resource_account) >= 1, error::out_of_range(EINSUFFICIENT_FUND));

        let one_wrapped_aptos_coin = coin::extract(&mut coin, 1);
        coin::deposit<WrappedAptosCoin>(@resource_account, one_wrapped_aptos_coin);

        let module_data = borrow_global_mut<ModuleData>(@resource_account);
        let resource_signer = account::create_signer_with_capability(&module_data.resource_signer_cap);

        (coin, coin::withdraw<AptosCoin>(&resource_signer, 1))
    }
}
