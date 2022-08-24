spec aptos_framework::staking_config {
    spec module {
        use aptos_framework::timestamp;
        invariant timestamp::is_operating() ==> exists<StakingConfig>(@aptos_framework);
    }
}
