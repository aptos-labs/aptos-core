spec aptos_framework::reconfiguration_with_dkg {
    spec module {
        pragma verify = true;
    }

    spec try_start() {
        use aptos_framework::chain_status;
        use aptos_framework::staking_config;
        use aptos_framework::reconfiguration;
        pragma verify_duration_estimate = 120;
        requires exists<reconfiguration::Configuration>(@aptos_framework);
        requires chain_status::is_operating();
        include stake::ResourceRequirement;
        include stake::GetReconfigStartTimeRequirement;
        include features::spec_periodical_reward_rate_decrease_enabled(
        ) ==> staking_config::StakingRewardsConfigEnabledRequirement;
        aborts_if false;
        pragma verify_duration_estimate = 600; // TODO: set because of timeout (property proved).
    }

    spec finish(framework: &signer) {
        pragma verify_duration_estimate = 1500;
        include FinishRequirement;
        aborts_if false;
    }

    spec schema FinishRequirement {
        use aptos_framework::chain_status;
        use std::signer;
        use std::features;
        use aptos_framework::stake;
        use aptos_framework::coin::CoinInfo;
        use aptos_framework::aptos_coin::AptosCoin;
        use aptos_framework::transaction_fee;
        use aptos_framework::staking_config;
        use aptos_framework::config_buffer;
        use aptos_framework::version;
        use aptos_framework::consensus_config;
        use aptos_framework::execution_config;
        use aptos_framework::gas_schedule;
        use aptos_framework::jwks;
        use aptos_framework::randomness_config;
        use aptos_framework::jwk_consensus_config;
        framework: signer;
        requires signer::address_of(framework) == @aptos_framework;
        requires chain_status::is_operating();
        requires exists<CoinInfo<AptosCoin>>(@aptos_framework);
        include staking_config::StakingRewardsConfigRequirement;
        requires exists<stake::ValidatorFees>(@aptos_framework);
        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
        requires exists<features::Features>(@std);
        include config_buffer::OnNewEpochRequirement<version::Version>;
        include config_buffer::OnNewEpochRequirement<gas_schedule::GasScheduleV2>;
        include config_buffer::OnNewEpochRequirement<execution_config::ExecutionConfig>;
        include config_buffer::OnNewEpochRequirement<consensus_config::ConsensusConfig>;
        include config_buffer::OnNewEpochRequirement<jwks::SupportedOIDCProviders>;
        include config_buffer::OnNewEpochRequirement<randomness_config::RandomnessConfig>;
        include config_buffer::OnNewEpochRequirement<randomness_config_seqnum::RandomnessConfigSeqNum>;
        include config_buffer::OnNewEpochRequirement<randomness_api_v0_config::AllowCustomMaxGasFlag>;
        include config_buffer::OnNewEpochRequirement<randomness_api_v0_config::RequiredGasDeposit>;
        include config_buffer::OnNewEpochRequirement<jwk_consensus_config::JWKConsensusConfig>;
        include config_buffer::OnNewEpochRequirement<keyless_account::Configuration>;
        include config_buffer::OnNewEpochRequirement<keyless_account::Groth16VerificationKey>;
    }

    spec finish_with_dkg_result(account: &signer, dkg_result: vector<u8>) {
        use aptos_framework::dkg;
        pragma verify_duration_estimate = 1500;
        include FinishRequirement {
            framework: account
        };
        requires dkg::has_incomplete_session();
        aborts_if false;
    }
}
