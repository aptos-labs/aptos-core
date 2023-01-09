spec aptos_framework::aptos_governance {
    spec reconfigure {
        use aptos_framework::chain_status;
        use aptos_framework::coin::CoinInfo;
        use aptos_framework::aptos_coin::AptosCoin;

        requires chain_status::is_operating();
        requires timestamp::spec_now_microseconds() >= reconfiguration::last_reconfiguration_time();
        requires exists<stake::ValidatorFees>(@aptos_framework);
        requires exists<CoinInfo<AptosCoin>>(@aptos_framework);
    }
}
