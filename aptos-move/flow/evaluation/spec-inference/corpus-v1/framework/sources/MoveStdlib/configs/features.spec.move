/// Maintains feature flags.
spec std::features {
    spec Features {
        pragma bv = b"0";
    }

    spec PendingFeatures {
        pragma bv = b"0";
    }

    spec set(features: &mut vector<u8>, feature: u64, include: bool) {
        pragma bv = b"0";
        aborts_if false;
        ensures feature / 8 < len(features);
        ensures include == spec_contains(features, feature);
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred = vacuous] forall x: vector<u8> :
            len(old(features)) > feature / 8 ==> features == x;
        aborts_if [inferred = sathard] exists x: vector<u8> :
            len(x) > feature / 8 && !in_range(x, feature / 8);
        aborts_if [inferred = sathard]((feature % 8) as u8) >= 8;
        aborts_if [inferred = sathard] feature % 8 > MAX_U8;
    }

    spec apply_diff(
        features: &mut vector<u8>, enable: vector<u64>, disable: vector<u64>
    ) {
        aborts_if [abstract] false; // TODO(#12011)
        ensures [abstract] forall i in disable: !spec_contains(features, i);
        ensures [abstract] forall i in enable:
            !vector::spec_contains(disable, i) ==>
                spec_contains(features, i);
        pragma opaque;
        pragma aborts_if_is_partial = true;
        ensures [inferred = sathard] forall x: u64:
            x < len(enable) ==>
                ensures_of<set>(old(features), enable[x], true);
        ensures [inferred = sathard] forall x: u64:
            x < len(disable) ==>
                ensures_of<set>(old(features), disable[x], false);
        ensures [inferred = vacuous] forall x: vector<u8> : features == x;
        aborts_if [inferred = sathard] len(enable) == 18446744073709551616;
        aborts_if [inferred = sathard] exists x: u64:
            x < len(enable) && !in_range(enable, x);
        aborts_if [inferred = sathard] len(disable) == 18446744073709551616;
        aborts_if [inferred = sathard] exists x: u64:
            x >= len(enable) && x < len(disable) && !in_range(disable, x);
    }

    spec contains(features: &vector<u8>, feature: u64): bool {
        pragma opaque = true;
        pragma bv = b"0";
        aborts_if false;
        ensures result == spec_contains(features, feature);
    }

    spec change_feature_flags_for_next_epoch(
        framework: &signer, enable: vector<u64>, disable: vector<u64>
    ) {
        use 0x1::signer;
        aborts_if signer::address_of(framework) != @std;
        // TODO(tengzhang): add functional spec
        // TODO(#12526): undo declaring opaque once fixed
        pragma opaque;
        modifies global<Features>(@std);
        modifies global<PendingFeatures>(@std);
        pragma aborts_if_is_partial = true;
        ensures [inferred] signer::address_of(framework) == @0x1
            && exists<PendingFeatures>(@0x1) ==>
            {
                let a = signer::address_of(framework);
                let b = PendingFeatures { features: PendingFeatures[@0x1].features };
                S2.. |~ publish<PendingFeatures>(a, b)
            };
        ensures [inferred] signer::address_of(framework) == @0x1
            && exists<PendingFeatures>(@0x1) ==>
            (
                ..S2 |~ ensures_of<apply_diff>(
                    PendingFeatures[@0x1].features, enable, disable
                )
            );
        ensures [inferred] signer::address_of(framework) == @0x1
            && exists<PendingFeatures>(@0x1) ==>
            remove<PendingFeatures>(@0x1);
        ensures [inferred] signer::address_of(framework) == @0x1
            && !exists<PendingFeatures>(@0x1) ==>
            {
                let a = signer::address_of(framework);
                S2.. |~ publish<PendingFeatures>(a, PendingFeatures { features: vec<u8>() })
            };
        ensures [inferred] signer::address_of(framework) == @0x1
            && !exists<PendingFeatures>(@0x1) ==>
            (..S2 |~ ensures_of<apply_diff>(vec<u8>(), enable, disable));
        aborts_if [inferred] signer::address_of(framework) != @0x1;
        aborts_if [inferred] signer::address_of(framework) == @0x1
            && (S2 |~ exists<PendingFeatures>(signer::address_of(framework)));
    }

    spec fun spec_contains(features: vector<u8>, feature: u64): bool {
        (
            (int2bv((((1 as u8) << ((feature % (8 as u64)) as u64)) as u8)) as u8)
                & features[feature / 8] as u8
        ) > (0 as u8) && (feature / 8) < len(features)
    }

    spec change_feature_flags_internal(
        framework: &signer, enable: vector<u64>, disable: vector<u64>
    ) {
        use 0x1::signer;
        pragma opaque;
        modifies global<Features>(@std);
        aborts_if signer::address_of(framework) != @std;
        pragma aborts_if_is_partial = true;
        ensures [inferred = sathard] signer::address_of(framework) == @0x1 ==>
            (
                forall x: u64,
                y: vector<u8> : x < len(enable) ==>
                    ensures_of<set>(y, enable[x], true)
            );
        ensures [inferred = sathard] signer::address_of(framework) == @0x1 ==>
            (
                forall x: vector<u8> :
                    update<Features>(
                        @0x1,
                        update_field(Features[@0x1], features, x)
                    )
            );
        ensures [inferred = sathard] signer::address_of(framework) == @0x1 ==>
            (
                forall x: u64,
                y: vector<u8> :
                    x < len(disable) ==>
                        ensures_of<set>(y, disable[x], false)
            );
        ensures [inferred = sathard] signer::address_of(framework) == @0x1
            && !exists<Features>(@0x1) ==>
            publish<Features>(
                signer::address_of(framework),
                Features { features: vec<u8>() }
            );
        aborts_if [inferred = sathard] signer::address_of(framework) != @0x1;
        aborts_if [inferred = sathard] signer::address_of(framework) == @0x1
            && len(enable) == 18446744073709551616;
        aborts_if [inferred = sathard] signer::address_of(framework) == @0x1
            && (exists x: u64: x < len(enable)
                && !in_range(enable, x));
        aborts_if [inferred = sathard] signer::address_of(framework) == @0x1
            && len(disable) == 18446744073709551616;
        aborts_if [inferred = sathard] signer::address_of(framework) == @0x1
            && (
                exists x: u64:
                    x >= len(enable)
                        && x < len(disable)
                        && !in_range(disable, x)
            );
        aborts_if [inferred = sathard] signer::address_of(framework) == @0x1
            && !exists<Features>(@0x1);
        aborts_if [inferred = sathard] signer::address_of(framework) == @0x1
            && (
                !exists<Features>(@0x1)
                    && exists<Features>(signer::address_of(framework))
            );
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
        ensures [inferred] result == spec_is_enabled(16);
        aborts_if [inferred] aborts_of<is_enabled>(16);
    }

    spec fun spec_partial_governance_voting_enabled(): bool {
        spec_is_enabled(PARTIAL_GOVERNANCE_VOTING)
    }

    spec partial_governance_voting_enabled {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_partial_governance_voting_enabled();
        ensures [inferred] result == spec_is_enabled(17);
        aborts_if [inferred] aborts_of<is_enabled>(17);
    }

    spec module_event_enabled {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_module_event_enabled();
        ensures [inferred] result == spec_is_enabled(26);
        aborts_if [inferred] aborts_of<is_enabled>(26);
    }

    spec fun spec_abort_if_multisig_payload_mismatch_enabled(): bool {
        spec_is_enabled(ABORT_IF_MULTISIG_PAYLOAD_MISMATCH)
    }

    spec fun spec_multisig_timelock_enabled(): bool {
        spec_is_enabled(MULTISIG_TIMELOCK)
    }

    spec is_multisig_timelock_enabled {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_multisig_timelock_enabled();
        ensures [inferred] result == spec_is_enabled(115);
        aborts_if [inferred] aborts_of<is_enabled>(115);
    }

    spec fun spec_new_accounts_default_to_fa_store_enabled(): bool {
        spec_is_enabled(NEW_ACCOUNTS_DEFAULT_TO_FA_STORE)
    }

    spec fun spec_simulation_enhancement_enabled(): bool {
        spec_is_enabled(TRANSACTION_SIMULATION_ENHANCEMENT)
    }

    spec abort_if_multisig_payload_mismatch_enabled {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_abort_if_multisig_payload_mismatch_enabled();
        ensures [inferred] result == spec_is_enabled(70);
        aborts_if [inferred] aborts_of<is_enabled>(70);
    }

    spec is_default_account_resource_enabled {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_is_enabled(DEFAULT_ACCOUNT_RESOURCE);
    }

    spec on_new_epoch(framework: &signer) {
        use 0x1::signer;
        requires @std == signer::address_of(framework);
        let features_pending = global<PendingFeatures>(@std).features;
        let post features_std = global<Features>(@std).features;
        ensures exists<PendingFeatures>(@std) ==>
            features_std == features_pending;
        aborts_if false;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies Features[@0x1];
        modifies Features[signer::address_of(framework)];
        ensures [inferred]({
            let a = S1 |~ exists<PendingFeatures>(@0x1);
            let b = S2 |~ exists<Features>(@0x1);
            let c = S1 |~ global<PendingFeatures>(@0x1);
            @0x1 == signer::address_of(framework)
                && (a && b) ==>
                Features[@0x1].features == c.features
        });
        ensures [inferred]@0x1 == signer::address_of(framework)
            && (
                (S1 |~ exists<PendingFeatures>(@0x1))
                    && (S2 |~ !exists<Features>(@0x1))
            ) ==>
            {
                let a = signer::address_of(framework);
                let b = Features {
                    features: (S1 |~ global<PendingFeatures>(@0x1)).features
                };
                S2.. |~ publish<Features>(a, b)
            };
        ensures [inferred]@0x1 == signer::address_of(framework)
            && (S1 |~ exists<PendingFeatures>(@0x1)) ==>
            (S1..S2 |~ remove<PendingFeatures>(@0x1));
        ensures [inferred]@0x1 == signer::address_of(framework) ==>
            (..S1 |~ ensures_of<ensure_framework_signer>(framework));
        aborts_if [inferred]({
            let a = S1 |~ exists<PendingFeatures>(@0x1);
            let b = S2 |~ exists<Features>(@0x1);
            let c = S2 |~ exists<Features>(signer::address_of(framework));
            @0x1 == signer::address_of(framework) && (a && (!b && c))
        });
    }

    spec fun spec_sha_512_and_ripemd_160_enabled(): bool {
        spec_is_enabled(SHA_512_AND_RIPEMD_160_NATIVES)
    }

    spec is_storage_slot_natives_enabled {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_is_enabled(STORAGE_SLOT_NATIVES);
        ensures [inferred] result == spec_is_enabled(113);
        aborts_if [inferred] aborts_of<is_enabled>(113);
    }

    spec aggregator_v2_api_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == true;
        aborts_if [inferred] false;
    }

    spec aggregator_v2_is_at_least_api_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == true;
        aborts_if [inferred] false;
    }

    spec allow_vm_binary_format_v6(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(5);
        aborts_if [inferred] aborts_of<is_enabled>(5);
    }

    spec aptos_stdlib_chain_id_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(4);
        aborts_if [inferred] aborts_of<is_enabled>(4);
    }

    spec auids_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == true;
        aborts_if [inferred] false;
    }

    spec blake2b_256_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(8);
        aborts_if [inferred] aborts_of<is_enabled>(8);
    }

    spec bls12_381_structures_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(13);
        aborts_if [inferred] aborts_of<is_enabled>(13);
    }

    spec bn254_structures_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(43);
        aborts_if [inferred] aborts_of<is_enabled>(43);
    }

    spec bulletproofs_batch_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(87);
        aborts_if [inferred] aborts_of<is_enabled>(87);
    }

    spec bulletproofs_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(24);
        aborts_if [inferred] aborts_of<is_enabled>(24);
    }

    spec code_dependency_check_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(1);
        aborts_if [inferred] aborts_of<is_enabled>(1);
    }

    spec coin_to_fungible_asset_migration_feature_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(60);
        aborts_if [inferred] aborts_of<is_enabled>(60);
    }

    spec collect_and_distribute_gas_fees(): bool {
        pragma opaque = true;
        ensures [inferred] result == false;
        aborts_if [inferred] false;
    }

    spec commission_change_delegation_pool_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(42);
        aborts_if [inferred] aborts_of<is_enabled>(42);
    }

    spec concurrent_fungible_assets_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(50);
        aborts_if [inferred] aborts_of<is_enabled>(50);
    }

    spec concurrent_fungible_balance_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(67);
        aborts_if [inferred] aborts_of<is_enabled>(67);
    }

    spec concurrent_token_v2_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == true;
        aborts_if [inferred] false;
    }

    spec cryptography_algebra_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(12);
        aborts_if [inferred] aborts_of<is_enabled>(12);
    }

    spec default_to_concurrent_fungible_balance_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(68);
        aborts_if [inferred] aborts_of<is_enabled>(68);
    }

    spec delegation_pool_allowlisting_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(56);
        aborts_if [inferred] aborts_of<is_enabled>(56);
    }

    spec delegation_pool_partial_governance_voting_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(21);
        aborts_if [inferred] aborts_of<is_enabled>(21);
    }

    spec delegation_pools_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(11);
        aborts_if [inferred] aborts_of<is_enabled>(11);
    }

    spec dispatchable_fungible_asset_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == true;
        aborts_if [inferred] false;
    }

    spec ensure_framework_signer(account: &signer) {
        use 0x1::signer;
        pragma opaque = true, aborts_if_is_partial = true;
        aborts_if [inferred] signer::address_of(account) != @0x1;
    }

    spec fee_payer_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(22);
        aborts_if [inferred] aborts_of<is_enabled>(22);
    }

    spec gas_refund_fa_mint_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(124);
        aborts_if [inferred] aborts_of<is_enabled>(124);
    }

    spec get_abort_if_multisig_payload_mismatch_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 70;
        aborts_if [inferred] false;
    }

    spec get_account_abstraction_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 85;
        aborts_if [inferred] false;
    }

    spec get_aptos_stdlib_chain_id_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 4;
        aborts_if [inferred] false;
    }

    spec get_auids(): u64 {
        use 0x1::error;
        pragma opaque = true;
        ensures [inferred] result == error::invalid_argument(3);
        aborts_if [inferred] aborts_of<error::invalid_argument>(3);
    }

    spec get_blake2b_256_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 8;
        aborts_if [inferred] false;
    }

    spec get_bls12_381_strutures_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 13;
        aborts_if [inferred] false;
    }

    spec get_bn254_strutures_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 43;
        aborts_if [inferred] false;
    }

    spec get_bulletproofs_batch_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 87;
        aborts_if [inferred] false;
    }

    spec get_bulletproofs_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 24;
        aborts_if [inferred] false;
    }

    spec get_calculate_transaction_fee_for_distribution_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 96;
        aborts_if [inferred] false;
    }

    spec get_coin_to_fungible_asset_migration_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 60;
        aborts_if [inferred] false;
    }

    spec get_collection_owner_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 79;
        aborts_if [inferred] false;
    }

    spec get_commission_change_delegation_pool_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 42;
        aborts_if [inferred] false;
    }

    spec get_concurrent_fungible_assets_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 50;
        aborts_if [inferred] false;
    }

    spec get_concurrent_fungible_balance_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 67;
        aborts_if [inferred] false;
    }

    spec get_concurrent_token_v2_feature(): u64 {
        use 0x1::error;
        pragma opaque = true;
        ensures [inferred] result == error::invalid_argument(3);
        aborts_if [inferred] aborts_of<error::invalid_argument>(3);
    }

    spec get_cryptography_algebra_natives_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 12;
        aborts_if [inferred] false;
    }

    spec get_default_account_resource_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 91;
        aborts_if [inferred] false;
    }

    spec get_default_to_concurrent_fungible_balance_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 68;
        aborts_if [inferred] false;
    }

    spec get_delegation_pool_allowlisting_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 56;
        aborts_if [inferred] false;
    }

    spec get_delegation_pool_partial_governance_voting(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 21;
        aborts_if [inferred] false;
    }

    spec get_delegation_pools_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 11;
        aborts_if [inferred] false;
    }

    spec get_distribute_transaction_fee_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 97;
        aborts_if [inferred] false;
    }

    spec get_encrypted_transactions_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 108;
        aborts_if [inferred] false;
    }

    spec get_function_reflection_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 105;
        aborts_if [inferred] false;
    }

    spec get_function_value_dispatch_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 125;
        aborts_if [inferred] false;
    }

    spec get_jwk_consensus_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 49;
        aborts_if [inferred] false;
    }

    spec get_jwk_consensus_per_key_mode_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 92;
        aborts_if [inferred] false;
    }

    spec get_keyless_accounts_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 46;
        aborts_if [inferred] false;
    }

    spec get_keyless_accounts_with_passkeys_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 54;
        aborts_if [inferred] false;
    }

    spec get_keyless_but_zkless_accounts_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 47;
        aborts_if [inferred] false;
    }

    spec get_max_object_nesting_check_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 53;
        aborts_if [inferred] false;
    }

    spec get_module_event_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 26;
        aborts_if [inferred] false;
    }

    spec get_module_event_migration_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 57;
        aborts_if [inferred] false;
    }

    spec get_multisig_accounts_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 10;
        aborts_if [inferred] false;
    }

    spec get_multisig_script_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 110;
        aborts_if [inferred] false;
    }

    spec get_multisig_timelock_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 115;
        aborts_if [inferred] false;
    }

    spec get_multisig_v2_enhancement_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 55;
        aborts_if [inferred] false;
    }

    spec get_native_collateral_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 121;
        aborts_if [inferred] false;
    }

    spec get_native_memory_operations_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 80;
        aborts_if [inferred] false;
    }

    spec get_native_orderbook_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 120;
        aborts_if [inferred] false;
    }

    spec get_native_position_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 119;
        aborts_if [inferred] false;
    }

    spec get_operator_beneficiary_change_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 39;
        aborts_if [inferred] false;
    }

    spec get_orderless_transactions_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 94;
        aborts_if [inferred] false;
    }

    spec get_partial_governance_voting(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 17;
        aborts_if [inferred] false;
    }

    spec get_periodical_reward_rate_decrease_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 16;
        aborts_if [inferred] false;
    }

    spec get_reconfigure_with_dkg_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 45;
        aborts_if [inferred] false;
    }

    spec get_resource_groups_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 9;
        aborts_if [inferred] false;
    }

    spec get_sha_512_and_ripemd_160_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 3;
        aborts_if [inferred] false;
    }

    spec get_signer_native_format_fix_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 25;
        aborts_if [inferred] false;
    }

    spec get_slh_dsa_sha2_128s_signature_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 107;
        aborts_if [inferred] false;
    }

    spec get_sponsored_automatic_account_creation(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 34;
        aborts_if [inferred] false;
    }

    spec get_trading_native_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 118;
        aborts_if [inferred] false;
    }

    spec get_transaction_context_extension_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 59;
        aborts_if [inferred] false;
    }

    spec get_transaction_limits_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 111;
        aborts_if [inferred] false;
    }

    spec get_transaction_simulation_enhancement_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 78;
        aborts_if [inferred] false;
    }

    spec get_vm_binary_format_v6(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 5;
        aborts_if [inferred] false;
    }

    spec is_account_abstraction_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(85);
        aborts_if [inferred] aborts_of<is_enabled>(85);
    }

    spec is_calculate_transaction_fee_for_distribution_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(96);
        aborts_if [inferred] aborts_of<is_enabled>(96);
    }

    spec is_collection_owner_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(79);
        aborts_if [inferred] aborts_of<is_enabled>(79);
    }

    spec is_derivable_account_abstraction_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(88);
        aborts_if [inferred] aborts_of<is_enabled>(88);
    }

    spec is_distribute_transaction_fee_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(97);
        aborts_if [inferred] aborts_of<is_enabled>(97);
    }

    spec is_domain_account_abstraction_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == false;
        aborts_if [inferred] false;
    }

    spec is_encrypted_transactions_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(108);
        aborts_if [inferred] aborts_of<is_enabled>(108);
    }

    spec is_function_reflection_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(105);
        aborts_if [inferred] aborts_of<is_enabled>(105);
    }

    spec is_function_value_dispatch_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] spec_is_enabled(125) ==>
            result == spec_is_enabled(105);
        ensures [inferred]!spec_is_enabled(125) ==> result == false;
        aborts_if [inferred] spec_is_enabled(125) && aborts_of<is_enabled>(105);
        aborts_if [inferred] aborts_of<is_enabled>(125);
    }

    spec is_jwk_consensus_per_key_mode_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(92);
        aborts_if [inferred] aborts_of<is_enabled>(92);
    }

    spec is_lazy_module_initialization_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(127);
        aborts_if [inferred] aborts_of<is_enabled>(127);
    }

    spec is_monotonically_increasing_counter_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == true;
        aborts_if [inferred] false;
    }

    spec is_multisig_script_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(110);
        aborts_if [inferred] aborts_of<is_enabled>(110);
    }

    spec is_native_collateral_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(121);
        aborts_if [inferred] aborts_of<is_enabled>(121);
    }

    spec is_native_memory_operations_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == true;
        aborts_if [inferred] false;
    }

    spec is_native_orderbook_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(120);
        aborts_if [inferred] aborts_of<is_enabled>(120);
    }

    spec is_native_position_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(119);
        aborts_if [inferred] aborts_of<is_enabled>(119);
    }

    spec is_object_code_deployment_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == true;
        aborts_if [inferred] false;
    }

    spec is_permissioned_signer_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == false;
        aborts_if [inferred] false;
    }

    spec is_trading_native_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(118);
        aborts_if [inferred] aborts_of<is_enabled>(118);
    }

    spec is_transaction_limits_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(111);
        aborts_if [inferred] aborts_of<is_enabled>(111);
    }

    spec jwk_consensus_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(49);
        aborts_if [inferred] aborts_of<is_enabled>(49);
    }

    spec keyless_accounts_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(46);
        aborts_if [inferred] aborts_of<is_enabled>(46);
    }

    spec keyless_accounts_with_passkeys_feature_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(54);
        aborts_if [inferred] aborts_of<is_enabled>(54);
    }

    spec keyless_but_zkless_accounts_feature_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(47);
        aborts_if [inferred] aborts_of<is_enabled>(47);
    }

    spec max_object_nesting_check_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(53);
        aborts_if [inferred] aborts_of<is_enabled>(53);
    }

    spec module_event_migration_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(57);
        aborts_if [inferred] aborts_of<is_enabled>(57);
    }

    spec multi_ed25519_pk_validate_v2_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(7);
        aborts_if [inferred] aborts_of<is_enabled>(7);
    }

    spec multi_ed25519_pk_validate_v2_feature(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 7;
        aborts_if [inferred] false;
    }

    spec multisig_accounts_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(10);
        aborts_if [inferred] aborts_of<is_enabled>(10);
    }

    spec multisig_v2_enhancement_feature_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(55);
        aborts_if [inferred] aborts_of<is_enabled>(55);
    }

    spec new_accounts_default_to_fa_apt_store_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == true;
        aborts_if [inferred] false;
    }

    spec new_accounts_default_to_fa_store_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == true;
        aborts_if [inferred] false;
    }

    spec object_native_derived_address_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == true;
        aborts_if [inferred] false;
    }

    spec operations_default_to_fa_apt_store_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == true;
        aborts_if [inferred] false;
    }

    spec operator_beneficiary_change_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(39);
        aborts_if [inferred] aborts_of<is_enabled>(39);
    }

    spec orderless_transactions_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(94);
        aborts_if [inferred] aborts_of<is_enabled>(94);
    }

    spec primary_apt_fungible_store_at_user_address_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == true;
        aborts_if [inferred] false;
    }

    spec reconfigure_with_dkg_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(45);
        aborts_if [inferred] aborts_of<is_enabled>(45);
    }

    spec resource_groups_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(9);
        aborts_if [inferred] aborts_of<is_enabled>(9);
    }

    spec sha_512_and_ripemd_160_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(3);
        aborts_if [inferred] aborts_of<is_enabled>(3);
    }

    spec signer_native_format_fix_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(25);
        aborts_if [inferred] aborts_of<is_enabled>(25);
    }

    spec slh_dsa_sha2_128s_signature_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(107);
        aborts_if [inferred] aborts_of<is_enabled>(107);
    }

    spec sponsored_automatic_account_creation_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(34);
        aborts_if [inferred] aborts_of<is_enabled>(34);
    }

    spec transaction_context_extension_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(59);
        aborts_if [inferred] aborts_of<is_enabled>(59);
    }

    spec transaction_simulation_enhancement_enabled(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(78);
        aborts_if [inferred] aborts_of<is_enabled>(78);
    }

    spec treat_friend_as_private(): bool {
        pragma opaque = true;
        ensures [inferred] result == spec_is_enabled(2);
        aborts_if [inferred] aborts_of<is_enabled>(2);
    }
}
