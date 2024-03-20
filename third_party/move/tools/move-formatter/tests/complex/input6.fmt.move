module econia::incentives {

    // Uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::{Self, Coin};
    use aptos_std::type_info::{Self, TypeInfo};
    use econia::resource_account;
    use econia::tablist::{Self, Tablist};
    use std::signer::address_of;
    use std::vector;

    #[test(econia = @econia, integrator = @user)]
    /// Verify registration of assorted coin stores, fee assessment, and
    /// withdrawal scenarios.
    fun test_register_assess_withdraw(econia: &signer, integrator: &signer)
        acquires EconiaFeeStore, IncentiveParameters, IntegratorFeeStores, UtilityCoinStore {
        init_test(); // Init incentives.
    }
}
