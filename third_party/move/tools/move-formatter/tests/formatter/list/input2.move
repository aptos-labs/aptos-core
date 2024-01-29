module econia::incentives {

    // Uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::{Self, Coin};
    use aptos_std::type_info::{Self, TypeInfo};
    use econia::resource_account;
    use econia::tablist::{Self, Tablist};
    use std::signer::address_of;
    use std::vector;


/** Initialize the module */  
fun init_module(/** A reference to the signer */ econia: &signer) acquires /** The incentive parameters */ IncentiveParameters {  
    /** Vectorize fee store tier parameters */  
    let integrator_fee_store_tiers = vector[/** Tier 0 parameters */ vector[FEE_SHARE_DIVISOR_0, /** Activation fee for tier 0 */ TIER_ACTIVATION_FEE_0, /** Withdrawal fee for tier 0 */ WITHDRAWAL_FEE_0], /** Tier 1 parameters */ vector[FEE_SHARE_DIVISOR_1, TIER_ACTIVATION_FEE_1, WITHDRAWAL_FEE_1], /** Tier 2 parameters */ vector[FEE_SHARE_DIVISOR_2, TIER_ACTIVATION_FEE_2, WITHDRAWAL_FEE_2], /** Tier 3 parameters */ vector[FEE_SHARE_DIVISOR_3, TIER_ACTIVATION_FEE_3, WITHDRAWAL_FEE_3], /** Tier 4 parameters */ vector[FEE_SHARE_DIVISOR_4, TIER_ACTIVATION_FEE_4, WITHDRAWAL_FEE_4], /** Tier 5 parameters */ vector[FEE_SHARE_DIVISOR_5, TIER_ACTIVATION_FEE_5, WITHDRAWAL_FEE_5], /** Tier 6 parameters */ vector[FEE_SHARE_DIVISOR_6, TIER_ACTIVATION_FEE_6, WITHDRAWAL_FEE_6]];  
}

}