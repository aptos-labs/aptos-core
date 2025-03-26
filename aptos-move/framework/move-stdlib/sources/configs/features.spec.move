/// Maintains feature flags.
spec std::features {
    spec Features {
        pragma bv=b"0";
    }

    spec PendingFeatures {
        pragma bv=b"0";
    }

    spec set(features: &mut vector<u8>, feature: u64, include: bool) {
        pragma bv=b"0";
        aborts_if false;
        ensures feature / 8 < len(features);
        ensures include == spec_contains(features, feature);
    }


    spec apply_diff(features: &mut vector<u8>, enable: vector<u64>, disable: vector<u64>) {
        aborts_if [abstract] false; // TODO(#12011)
        ensures [abstract] forall i in disable: !spec_contains(features, i);
        ensures [abstract] forall i in enable: !vector::spec_contains(disable, i)
            ==> spec_contains(features, i);
        pragma opaque;
    }

    spec contains(features: &vector<u8>, feature: u64): bool {
        pragma bv=b"0";
        aborts_if false;
        ensures result == spec_contains(features, feature);
    }

    spec change_feature_flags_for_next_epoch(framework: &signer, enable: vector<u64>, disable: vector<u64>) {
        aborts_if signer::address_of(framework) != @std;
        // TODO(tengzhang): add functional spec
        // TODO(#12526): undo declaring opaque once fixed
        pragma opaque;
        modifies global<Features>(@std);
        modifies global<PendingFeatures>(@std);
    }

    spec fun spec_contains(features: vector<u8>, feature: u64): bool {
        ((int2bv((((1 as u8) << ((feature % (8 as u64)) as u64)) as u8)) as u8) & features[feature/8] as u8) > (0 as u8)
            && (feature / 8) < len(features)
    }

    spec change_feature_flags_internal(framework: &signer, enable: vector<u64>, disable: vector<u64>) {
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

    spec fun spec_abort_if_multisig_payload_mismatch_enabled(): bool {
        spec_is_enabled(ABORT_IF_MULTISIG_PAYLOAD_MISMATCH)
    }

    spec fun spec_new_accounts_default_to_fa_apt_store_enabled(): bool {
        spec_is_enabled(NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE)
    }

    spec fun spec_simulation_enhancement_enabled(): bool {
        spec_is_enabled(TRANSACTION_SIMULATION_ENHANCEMENT)
    }

    spec abort_if_multisig_payload_mismatch_enabled {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_abort_if_multisig_payload_mismatch_enabled();
    }

    spec on_new_epoch(framework: &signer) {
        requires @std == signer::address_of(framework);
        let features_pending = global<PendingFeatures>(@std).features;
        let post features_std = global<Features>(@std).features;
        ensures exists<PendingFeatures>(@std) ==> features_std == features_pending;
        aborts_if false;
    }

    spec fun spec_sha_512_and_ripemd_160_enabled(): bool {
        spec_is_enabled(SHA_512_AND_RIPEMD_160_NATIVES)
    }
}
