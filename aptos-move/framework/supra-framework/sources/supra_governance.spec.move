spec supra_framework::supra_governance {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The create proposal function calls create proposal v2.
    /// Criticality: Low
    /// Implementation: The create_proposal function internally calls create_proposal_v2.
    /// Enforcement: This is manually audited to ensure create_proposal_v2 is called in create_proposal.
    ///
    /// No.: 2
    /// Requirement: The Approved execution hashes resources that exist when the vote function is called.
    /// Criticality: Low
    /// Implementation: The Vote function acquires the Approved execution hashes resources.
    /// Enforcement: Formally verified in [high-level-req-2](VoteAbortIf).
    ///
    /// No.: 3
    /// Requirement: The execution script hash of a successful governance proposal is added to the approved list if the
    /// proposal can be resolved.
    /// Criticality: Medium
    /// Implementation: The add_approved_script_hash function asserts that proposal_state == PROPOSAL_STATE_SUCCEEDED.
    /// Enforcement: Formally verified in [high-level-req-3](AddApprovedScriptHash).
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        // pragma aborts_if_is_strict;
        pragma aborts_if_is_partial;
    }

    spec store_signer_cap(
        supra_framework: &signer,
        signer_address: address,
        signer_cap: SignerCapability,
    ) {
        aborts_if !system_addresses::is_supra_framework_address(signer::address_of(supra_framework));
        aborts_if !system_addresses::is_framework_reserved_address(signer_address);

        let signer_caps = global<GovernanceResponsbility>(@supra_framework).signer_caps;
        aborts_if exists<GovernanceResponsbility>(@supra_framework) &&
            simple_map::spec_contains_key(signer_caps, signer_address);
        ensures exists<GovernanceResponsbility>(@supra_framework);
        let post post_signer_caps = global<GovernanceResponsbility>(@supra_framework).signer_caps;
        ensures simple_map::spec_contains_key(post_signer_caps, signer_address);
    }

    /// Signer address must be @supra_framework.
    /// The signer does not allow these resources (GovernanceProposal, GovernanceConfig, GovernanceEvents, VotingRecords, ApprovedExecutionHashes) to exist.
    /// The signer must have an Account.
    /// Limit addition overflow.
    spec initialize(
        supra_framework: &signer,
        voting_duration_secs: u64,
        min_voting_threshold: u64,
        voters: vector<address>,
    ) {
        use aptos_std::type_info::Self;
        let addr = signer::address_of(supra_framework);
        let register_account = global<account::Account>(addr);
        aborts_if exists<multisig_voting::VotingForum<GovernanceProposal>>(addr);
        aborts_if min_voting_threshold <= 1;
        aborts_if !(vector::length(voters) >= min_voting_threshold && min_voting_threshold > vector::length(voters) / 2);
        aborts_if !exists<account::Account>(addr);
        aborts_if register_account.guid_creation_num + 7 > MAX_U64;
        aborts_if register_account.guid_creation_num + 7 >= account::MAX_GUID_CREATION_NUM;
        aborts_if !type_info::spec_is_struct<GovernanceProposal>();

        include InitializeAbortIf;

        ensures exists<multisig_voting::VotingForum<governance_proposal::GovernanceProposal>>(addr);
        ensures exists<SupraGovernanceConfig>(addr);
        ensures exists<SupraGovernanceEvents>(addr);
        ensures exists<ApprovedExecutionHashes>(addr);
    }

    spec schema InitializeAbortIf {
        supra_framework: &signer;
        min_voting_threshold: u128;
        voters: vector<address>;
        voting_duration_secs: u64;

        let addr = signer::address_of(supra_framework);
        let account = global<account::Account>(addr);
        aborts_if exists<multisig_voting::VotingForum<governance_proposal::GovernanceProposal>>(addr);
        aborts_if exists<SupraGovernanceConfig>(addr);
        aborts_if exists<SupraGovernanceEvents>(addr);
        aborts_if exists<ApprovedExecutionHashes>(addr);
        aborts_if !exists<account::Account>(addr);
    }

    /// Signer address must be @supra_framework.
    /// Address @supra_framework must exist GovernanceConfig and GovernanceEvents.
    spec update_supra_governance_config(
        supra_framework: &signer,
        voting_duration_secs: u64,
        min_voting_threshold: u64,
        voters: vector<address>,
    ) {
        aborts_if min_voting_threshold <= 1;
        aborts_if !(vector::length(voters) >= min_voting_threshold && min_voting_threshold > vector::length(voters) / 2);
        let addr = signer::address_of(supra_framework);
        let governance_config = global<SupraGovernanceConfig>(@supra_framework);

        let post new_governance_config = global<SupraGovernanceConfig>(@supra_framework);
        aborts_if addr != @supra_framework;
        aborts_if (vector::length(voters) < min_voting_threshold || min_voting_threshold < vector::length(voters) / 2);
        aborts_if min_voting_threshold <= 1;
        aborts_if !exists<SupraGovernanceConfig>(@supra_framework);
        aborts_if !exists<SupraGovernanceEvents>(@supra_framework);
        modifies global<SupraGovernanceConfig>(addr);

        ensures new_governance_config.voting_duration_secs == voting_duration_secs;
        ensures new_governance_config.min_voting_threshold == min_voting_threshold;
    }

    /// Signer address must be @supra_framework.
    /// Address @supra_framework must exist GovernanceConfig and GovernanceEvents.
    spec toggle_features(
        supra_framework: &signer,
        enable: vector<u64>,
        disable: vector<u64>,
    ) {
        use supra_framework::chain_status;
        use supra_framework::coin::CoinInfo;
        use supra_framework::supra_coin::SupraCoin;
        use supra_framework::transaction_fee;
        pragma verify = false; // TODO: set because of timeout (property proved).
        let addr = signer::address_of(supra_framework);
        aborts_if addr != @supra_framework;
        include reconfiguration_with_dkg::FinishRequirement {
            framework: supra_framework
        };
        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
        requires chain_status::is_operating();
        requires exists<CoinInfo<SupraCoin>>(@supra_framework);
    }

    spec get_voting_duration_secs(): u64 {
        include AbortsIfNotGovernanceConfig;
    }

    spec get_min_voting_threshold(): u64 {
        include AbortsIfNotGovernanceConfig;
    }

    spec get_voters_list(): vector<address> {
        include AbortsIfNotGovernanceConfig;
    }

    spec schema AbortsIfNotGovernanceConfig {
        aborts_if !exists<SupraGovernanceConfig>(@supra_framework);
    }

    /// The same as spec of `create_proposal_v2()`.
    spec supra_create_proposal(
        proposer: &signer,
        execution_hash: vector<u8>,
        metadata_location: vector<u8>,
        metadata_hash: vector<u8>,
    ) {
        use supra_framework::chain_status;
        pragma verify_duration_estimate = 60;

        requires chain_status::is_operating();
        include CreateProposalAbortsIf;
    }

    spec supra_create_proposal_v2(
        proposer: &signer,
        execution_hash: vector<u8>,
        metadata_location: vector<u8>,
        metadata_hash: vector<u8>,
        is_multi_step_proposal: bool,
    ) {
        use supra_framework::chain_status;

        pragma verify_duration_estimate = 60;
        requires chain_status::is_operating();
        include CreateProposalAbortsIf;
    }

    spec supra_create_proposal_v2_impl (
        proposer: &signer,
        execution_hash: vector<u8>,
        metadata_location: vector<u8>,
        metadata_hash: vector<u8>,
        is_multi_step_proposal: bool,
    ): u64 {
        use supra_framework::chain_status;
        pragma verify_duration_estimate = 60;
        requires chain_status::is_operating();
        include CreateProposalAbortsIf;
    }

    /// `stake_pool` must exist StakePool.
    /// The delegated voter under the resource StakePool of the stake_pool must be the proposer address.
    /// Address @supra_framework must exist GovernanceEvents.
    spec schema CreateProposalAbortsIf {
        use aptos_std::table;
        proposer: &signer;
        execution_hash: vector<u8>;
        metadata_location: vector<u8>;
        metadata_hash: vector<u8>;

        include VotingGetDelegatedVoterAbortsIf { sign: proposer };
        include AbortsIfNotGovernanceConfig;

        let governance_config = global<SupraGovernanceConfig>(@supra_framework);

        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@supra_framework);
        let current_time = timestamp::spec_now_seconds();
        let proposal_expiration = current_time + governance_config.voting_duration_secs;

        // verify create_proposal_metadata
        include CreateProposalMetadataAbortsIf;

        // verify voting::create_proposal_v2
        aborts_if len(execution_hash) == 0;
        aborts_if !exists<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        let voting_forum = global<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        let proposal_id = voting_forum.next_proposal_id;
        aborts_if proposal_id + 1 > MAX_U64;
        let post post_voting_forum = global<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        let post post_next_proposal_id = post_voting_forum.next_proposal_id;
        ensures post_next_proposal_id == proposal_id + 1;
        aborts_if !string::spec_internal_check_utf8(multisig_voting::IS_MULTI_STEP_PROPOSAL_KEY);
        aborts_if !string::spec_internal_check_utf8(multisig_voting::IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
        aborts_if table::spec_contains(voting_forum.proposals,proposal_id);
        ensures table::spec_contains(post_voting_forum.proposals, proposal_id);
        aborts_if !exists<SupraGovernanceEvents>(@supra_framework);
    }

    spec schema VotingGetDelegatedVoterAbortsIf {
        sign: signer;

        let addr = signer::address_of(sign);
    }

    /// stake_pool must exist StakePool.
    /// The delegated voter under the resource StakePool of the stake_pool must be the voter address.
    /// Address @supra_framework must exist VotingRecords and GovernanceProposal.
    spec supra_vote (
        voter: &signer,
        proposal_id: u64,
        should_pass: bool,
    ) {
        use supra_framework::chain_status;
        pragma verify_duration_estimate = 60;

        requires chain_status::is_operating();
        include VoteAbortIf;
    }

    spec supra_vote_internal (
        voter: &signer,
        proposal_id: u64,
        should_pass: bool,
    ) {
        use supra_framework::chain_status;
        pragma verify_duration_estimate = 60;

        requires chain_status::is_operating();
        include SupraVoteAbortIf;
    }

    spec schema SupraVoteAbortIf {
        voter: &signer;
        proposal_id: u64;
        should_pass: bool;

        aborts_if spec_proposal_expiration <= timestamp::now_seconds() && !exists<timestamp::CurrentTimeMicroseconds>(@supra_framework);
        let spec_proposal_expiration = multisig_voting::spec_get_proposal_expiration_secs<GovernanceProposal>(@supra_framework, proposal_id);
    }

    spec schema VoteAbortIf {
        use aptos_std::table;

        voter: &signer;
        proposal_id: u64;
        should_pass: bool;

        include VotingGetDelegatedVoterAbortsIf { sign: voter };
        aborts_if !exists<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        let voting_forum = global<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        let proposal = table::spec_get(voting_forum.proposals, proposal_id);
        aborts_if !table::spec_contains(voting_forum.proposals, proposal_id);
        let proposal_expiration = proposal.expiration_secs;
        // verify voting::vote
        aborts_if timestamp::now_seconds() > proposal_expiration;
        aborts_if proposal.is_resolved;
        aborts_if !string::spec_internal_check_utf8(multisig_voting::IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
        let execution_key = utf8(multisig_voting::IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
        aborts_if simple_map::spec_contains_key(proposal.metadata, execution_key) &&
            simple_map::spec_get(proposal.metadata, execution_key) != std::bcs::to_bytes(false);
        let post post_voting_forum = global<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        let post post_proposal = table::spec_get(post_voting_forum.proposals, proposal_id);

        aborts_if !string::spec_internal_check_utf8(multisig_voting::RESOLVABLE_TIME_METADATA_KEY);
        let key = utf8(multisig_voting::RESOLVABLE_TIME_METADATA_KEY);
        ensures simple_map::spec_contains_key(post_proposal.metadata, key);
        ensures simple_map::spec_get(post_proposal.metadata, key) == std::bcs::to_bytes(timestamp::now_seconds());

        aborts_if !exists<SupraGovernanceEvents>(@supra_framework);

        // verify add_approved_script_hash(proposal_id)
        let execution_hash = proposal.execution_hash;
        let post post_approved_hashes = global<ApprovedExecutionHashes>(@supra_framework);

        // Due to the complexity of the success state, the validation of 'borrow_global_mut<ApprovedExecutionHashes>(@supra_framework);' is discussed in four cases.
        /// [high-level-req-3]
        // aborts_if !exists<ApprovedExecutionHashes>(@supra_framework);

        ensures simple_map::spec_contains_key(post_approved_hashes.hashes, proposal_id) &&
            simple_map::spec_get(post_approved_hashes.hashes, proposal_id) == execution_hash;
    }

    spec add_supra_approved_script_hash(proposal_id: u64) {

        use supra_framework::chain_status;
        pragma aborts_if_is_partial = true;

        requires chain_status::is_operating();
        include AddApprovedScriptHash;
    }

    spec add_supra_approved_script_hash_script(proposal_id: u64) {
        use supra_framework::chain_status;
        pragma aborts_if_is_partial = true;

        requires chain_status::is_operating();
        include AddApprovedScriptHash;
    }

    spec schema AddApprovedScriptHash {
        use aptos_std::table;
        proposal_id: u64;
        aborts_if !exists<ApprovedExecutionHashes>(@supra_framework);

        aborts_if !exists<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        let voting_forum = global<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        let proposal = table::spec_get(voting_forum.proposals, proposal_id);
        aborts_if !table::spec_contains(voting_forum.proposals, proposal_id);

        let post post_approved_hashes = global<ApprovedExecutionHashes>(@supra_framework);
        /// [high-level-req-4]
        ensures simple_map::spec_contains_key(post_approved_hashes.hashes, proposal_id) &&
            simple_map::spec_get(post_approved_hashes.hashes, proposal_id) == proposal.execution_hash;
    }

    /// Address @supra_framework must exist ApprovedExecutionHashes and GovernanceProposal and GovernanceResponsbility.
    spec supra_resolve(proposal_id: u64, signer_address: address): signer {
        use supra_framework::chain_status;
        use aptos_std::table;
        use std::option;
        //TODO: Remove pragma aborts_if_is_partial;
        pragma aborts_if_is_partial = true;
        requires chain_status::is_operating();
        // verify mutisig_voting::resolve
        include VotingIsProposalResolvableAbortsif;

        let voting_forum = global<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        let proposal = table::spec_get(voting_forum.proposals, proposal_id);

        let multi_step_key = utf8(multisig_voting::IS_MULTI_STEP_PROPOSAL_KEY);
        let has_multi_step_key = simple_map::spec_contains_key(proposal.metadata, multi_step_key);
        let is_multi_step_proposal = aptos_std::from_bcs::deserialize<bool>(simple_map::spec_get(proposal.metadata, multi_step_key));
        aborts_if has_multi_step_key && !aptos_std::from_bcs::deserializable<bool>(simple_map::spec_get(proposal.metadata, multi_step_key));
        aborts_if !string::spec_internal_check_utf8(multisig_voting::IS_MULTI_STEP_PROPOSAL_KEY);
        aborts_if has_multi_step_key && is_multi_step_proposal;

        let post post_voting_forum = global<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        let post post_proposal = table::spec_get(post_voting_forum.proposals, proposal_id);
        ensures post_proposal.is_resolved == true && post_proposal.resolution_time_secs == timestamp::now_seconds();
        aborts_if option::spec_is_none(proposal.execution_content);

        // verify remove_approved_hash
        aborts_if !exists<ApprovedExecutionHashes>(@supra_framework);
        let post post_approved_hashes = global<ApprovedExecutionHashes>(@supra_framework).hashes;
        ensures !simple_map::spec_contains_key(post_approved_hashes, proposal_id);

        // verify get_signer
        include GetSignerAbortsIf;
        let governance_responsibility = global<GovernanceResponsbility>(@supra_framework);
        let signer_cap = simple_map::spec_get(governance_responsibility.signer_caps, signer_address);
        let addr = signer_cap.account;
        ensures signer::address_of(result) == addr;
    }

    /// Address @supra_framework must exist ApprovedExecutionHashes and GovernanceProposal.
    spec remove_supra_approved_hash(proposal_id: u64) {
        use aptos_std::table;
        pragma aborts_if_is_partial = true;
        aborts_if !exists<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        aborts_if !exists<ApprovedExecutionHashes>(@supra_framework);
        let voting_forum = global<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        aborts_if !table::spec_contains(voting_forum.proposals, proposal_id);
        aborts_if !exists<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        let proposal = table::spec_get(voting_forum.proposals, proposal_id);
        aborts_if !proposal.is_resolved;
        let post approved_hashes = global<ApprovedExecutionHashes>(@supra_framework).hashes;
        ensures !simple_map::spec_contains_key(approved_hashes, proposal_id);
    }

    spec reconfigure(supra_framework: &signer) {
        use supra_framework::chain_status;
        use supra_framework::coin::CoinInfo;
        use supra_framework::supra_coin::SupraCoin;
        use supra_framework::transaction_fee;
        pragma verify = false; // TODO: set because of timeout (property proved).
        aborts_if !system_addresses::is_supra_framework_address(signer::address_of(supra_framework));
        include reconfiguration_with_dkg::FinishRequirement {
            framework: supra_framework
        };

        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
        requires chain_status::is_operating();
        requires exists<CoinInfo<SupraCoin>>(@supra_framework);
    }

    /// Signer address must be @core_resources.
    /// signer must exist in MintCapStore.
    /// Address @supra_framework must exist GovernanceResponsbility.
    spec get_signer_testnet_only(core_resources: &signer, signer_address: address): signer {
        aborts_if signer::address_of(core_resources) != @core_resources;
        aborts_if !exists<supra_coin::MintCapStore>(signer::address_of(core_resources));
        include GetSignerAbortsIf;
    }

    spec get_signer(signer_address: address): signer {
        include GetSignerAbortsIf;
    }

    spec schema GetSignerAbortsIf {
        signer_address: address;

        aborts_if !exists<GovernanceResponsbility>(@supra_framework);
        let cap_map = global<GovernanceResponsbility>(@supra_framework).signer_caps;
        aborts_if !simple_map::spec_contains_key(cap_map, signer_address);
    }

    spec create_proposal_metadata(metadata_location: vector<u8>, metadata_hash: vector<u8>): SimpleMap<String, vector<u8>> {
        include CreateProposalMetadataAbortsIf;
    }

    spec schema CreateProposalMetadataAbortsIf {
        metadata_location: vector<u8>;
        metadata_hash: vector<u8>;

        aborts_if string::length(utf8(metadata_location)) > 256;
        aborts_if string::length(utf8(metadata_hash)) > 256;
        aborts_if !string::spec_internal_check_utf8(metadata_location);
        aborts_if !string::spec_internal_check_utf8(metadata_hash);
        aborts_if !string::spec_internal_check_utf8(METADATA_LOCATION_KEY);
        aborts_if !string::spec_internal_check_utf8(METADATA_HASH_KEY);
    }

    /// verify_only
    spec initialize_for_verification(
        supra_framework: &signer,
        voting_duration_secs: u64,
        supra_min_voting_threshold: u64,
        voters: vector<address>,
    ) {
        pragma verify = false;
    }

    spec resolve_supra_multi_step_proposal(proposal_id: u64, signer_address: address, next_execution_hash: vector<u8>): signer {
        use supra_framework::chain_status;
        use aptos_std::table;

        requires chain_status::is_operating();

        // TODO: These function passed locally however failed in github CI
        pragma verify_duration_estimate = 120;
        // verify multisig_voting::resolve_proposal_v2
        include VotingIsProposalResolvableAbortsif;

        let voting_forum = global<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        let proposal = table::spec_get(voting_forum.proposals, proposal_id);
        let post post_voting_forum = global<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        let post post_proposal = table::spec_get(post_voting_forum.proposals, proposal_id);

        aborts_if !string::spec_internal_check_utf8(multisig_voting::IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
        let multi_step_in_execution_key = utf8(multisig_voting::IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
        let post is_multi_step_proposal_in_execution_value = simple_map::spec_get(post_proposal.metadata, multi_step_in_execution_key);

        aborts_if !string::spec_internal_check_utf8(multisig_voting::IS_MULTI_STEP_PROPOSAL_KEY);
        let multi_step_key = utf8(multisig_voting::IS_MULTI_STEP_PROPOSAL_KEY);
        aborts_if simple_map::spec_contains_key(proposal.metadata, multi_step_key) &&
            !aptos_std::from_bcs::deserializable<bool>(simple_map::spec_get(proposal.metadata, multi_step_key));
        let is_multi_step = simple_map::spec_contains_key(proposal.metadata, multi_step_key) &&
            aptos_std::from_bcs::deserialize<bool>(simple_map::spec_get(proposal.metadata, multi_step_key));
        let next_execution_hash_is_empty = len(next_execution_hash) == 0;
        aborts_if !is_multi_step && !next_execution_hash_is_empty;
        aborts_if next_execution_hash_is_empty && is_multi_step && !simple_map::spec_contains_key(proposal.metadata, multi_step_in_execution_key); // ?
        ensures next_execution_hash_is_empty ==> post_proposal.is_resolved == true && post_proposal.resolution_time_secs == timestamp::spec_now_seconds() &&
            if (is_multi_step) {
                is_multi_step_proposal_in_execution_value == std::bcs::serialize(false)
            } else {
                simple_map::spec_contains_key(proposal.metadata, multi_step_in_execution_key) ==>
                    is_multi_step_proposal_in_execution_value == std::bcs::serialize(true)
            };
        ensures !next_execution_hash_is_empty ==> post_proposal.execution_hash == next_execution_hash;

        // verify remove_approved_hash
        aborts_if !exists<ApprovedExecutionHashes>(@supra_framework);
        let post post_approved_hashes = global<ApprovedExecutionHashes>(@supra_framework).hashes;
        ensures next_execution_hash_is_empty ==> !simple_map::spec_contains_key(post_approved_hashes, proposal_id);
        ensures !next_execution_hash_is_empty ==>
            simple_map::spec_get(post_approved_hashes, proposal_id) == next_execution_hash;

        // verify get_signer
        include GetSignerAbortsIf;
        let governance_responsibility = global<GovernanceResponsbility>(@supra_framework);
        let signer_cap = simple_map::spec_get(governance_responsibility.signer_caps, signer_address);
        let addr = signer_cap.account;
        ensures signer::address_of(result) == addr;
    }

    spec schema VotingIsProposalResolvableAbortsif {
        use aptos_std::table;
        proposal_id: u64;

        aborts_if !exists<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        let voting_forum = global<multisig_voting::VotingForum<GovernanceProposal>>(@supra_framework);
        let proposal = table::spec_get(voting_forum.proposals, proposal_id);
        aborts_if !table::spec_contains(voting_forum.proposals, proposal_id);
        let voting_period_over = timestamp::now_seconds() > proposal.expiration_secs;

        aborts_if proposal.is_resolved;
        aborts_if !string::spec_internal_check_utf8(multisig_voting::RESOLVABLE_TIME_METADATA_KEY);
        aborts_if !simple_map::spec_contains_key(proposal.metadata, utf8(multisig_voting::RESOLVABLE_TIME_METADATA_KEY));
        let resolvable_time = aptos_std::from_bcs::deserialize<u64>(simple_map::spec_get(proposal.metadata, utf8(multisig_voting::RESOLVABLE_TIME_METADATA_KEY)));
        aborts_if !aptos_std::from_bcs::deserializable<u64>(simple_map::spec_get(proposal.metadata, utf8(multisig_voting::RESOLVABLE_TIME_METADATA_KEY)));
        aborts_if timestamp::now_seconds() <= resolvable_time;
        aborts_if supra_framework::transaction_context::spec_get_script_hash() != proposal.execution_hash;
    }

    spec force_end_epoch(supra_framework: &signer) {
        use supra_framework::reconfiguration_with_dkg;
        use std::signer;
        pragma verify = false; // TODO: set because of timeout (property proved).
        let address = signer::address_of(supra_framework);
        include reconfiguration_with_dkg::FinishRequirement {
            framework: supra_framework
        };
    }

    spec force_end_epoch_test_only {
        pragma verify = false;
    }
}
