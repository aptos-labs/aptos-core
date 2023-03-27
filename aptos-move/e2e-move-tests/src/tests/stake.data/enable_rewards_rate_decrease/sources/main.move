script {
    use aptos_framework::aptos_governance;
    use aptos_framework::staking_config;
    use aptos_framework::timestamp;
    use std::features;

    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        staking_config::initialize_rewards(
            &framework_signer,
            100,
            30,
            10000,
            365 * 24 * 60 * 60 * 1000000,
            timestamp::now_microseconds(),
            5000,
        );
        let feature = features::get_reward_rate_decrease_feature();
        features::change_feature_flags(&framework_signer, vector[feature], vector[]);
    }
}
