/// Maintains feature flags.
spec std::features {
    spec Features {
        pragma bv=b"0";
    }

    spec set(features: &mut vector<u8>, feature: u64, include: bool) {
        pragma bv=b"0";
        aborts_if false;
        ensures feature / 8 < len(features);
        ensures include == spec_contains(features, feature);
    }

    spec contains(features: &vector<u8>, feature: u64): bool {
        pragma bv=b"0";
        aborts_if false;
        ensures result == spec_contains(features, feature);
    }


    spec fun spec_contains(features: vector<u8>, feature: u64): bool {
        ((int2bv((((1 as u8) << ((feature % (8 as u64)) as u64)) as u8)) as u8) & features[feature/8] as u8) > (0 as u8)
            && (feature / 8) < len(features)
    }

    spec change_feature_flags(framework: &signer, enable: vector<u64>, disable: vector<u64>) {
        pragma opaque;
        modifies global<Features>(@std);
        aborts_if signer::address_of(framework) != @std;
    }

    spec is_enabled(feature: u64): bool {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_is_enabled(feature);
    }

    spec fun spec_is_enabled(feature: u64): bool;

    spec fun spec_periodical_reward_rate_decrease_enabled(): bool {
        spec_is_enabled(PERIODICAL_REWARD_RATE_DECREASE)
    }

    spec fun spec_fee_payer_enabled(): bool {
        spec_is_enabled(FEE_PAYER_ENABLED)
    }

    spec fun spec_collect_and_distribute_gas_fees_enabled(): bool {
        spec_is_enabled(COLLECT_AND_DISTRIBUTE_GAS_FEES)
    }

    spec fun spec_module_event_enabled(): bool {
        spec_is_enabled(MODULE_EVENT)
    }

    spec periodical_reward_rate_decrease_enabled {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_periodical_reward_rate_decrease_enabled();
    }

    spec fun spec_partial_governance_voting_enabled(): bool {
        spec_is_enabled(PARTIAL_GOVERNANCE_VOTING)
    }

    spec partial_governance_voting_enabled {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_partial_governance_voting_enabled();
    }

    spec module_event_enabled {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_module_event_enabled();
    }
}
