script {
    use aptos_framework::aptos_governance;
    use aptos_framework::delegation_pool;
    use std::vector;

    const MIN_RESOLVABLE_VOTING_THRESHOLD_OCTAS: u128 = 20000000000000000;

    fun main(
        core_resources: &signer,
        pool_address_0: address,
        pool_address_1: address,
        pool_address_2: address,
        pool_address_3: address,
    ) {
        let framework_signer = aptos_governance::get_signer_testnet_only(
            core_resources,
            @aptos_framework,
        );

        aptos_governance::toggle_features(&framework_signer, vector[17, 21], vector::empty<u64>());
        if (!aptos_governance::partial_voting_initialized()) {
            aptos_governance::initialize_partial_voting(&framework_signer);
        };
        aptos_governance::update_governance_config(
            &framework_signer,
            MIN_RESOLVABLE_VOTING_THRESHOLD_OCTAS,
            aptos_governance::get_required_proposer_stake(),
            aptos_governance::get_voting_duration_secs(),
        );

        let pool_addresses = vector[
            pool_address_0,
            pool_address_1,
            pool_address_2,
            pool_address_3,
        ];
        vector::for_each(pool_addresses, |pool_address| {
            if (!delegation_pool::governance_records_initialized(pool_address)) {
                delegation_pool::enable_partial_governance_voting(pool_address);
            };
        });

        aptos_governance::force_end_epoch(&framework_signer);
    }
}
