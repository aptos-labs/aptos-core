/// Reconfiguration with DKG helper functions.
module aptos_framework::reconfiguration_with_dkg {
    use std::features;
    use aptos_framework::chunky_dkg;
    use aptos_framework::chunky_dkg_config;
    use aptos_framework::chunky_dkg_config_seqnum;
    use aptos_framework::consensus_config;
    use aptos_framework::create_signer;
    use aptos_framework::decryption;
    use aptos_framework::dkg;
    use aptos_framework::execution_config;
    use aptos_framework::gas_schedule;
    use aptos_framework::jwk_consensus_config;
    use aptos_framework::jwks;
    use aptos_framework::keyless_account;
    use aptos_framework::randomness_api_v0_config;
    use aptos_framework::randomness_config;
    use aptos_framework::randomness_config_seqnum;
    use aptos_framework::reconfiguration;
    use aptos_framework::reconfiguration_state;
    use aptos_framework::stake;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    friend aptos_framework::block;
    friend aptos_framework::aptos_governance;

    /// Trigger a reconfiguration with DKG.
    /// Do nothing if one is already in progress.
    public(friend) fun try_start() {
        let incomplete_dkg_session = dkg::incomplete_session();
        if (incomplete_dkg_session.is_some()) {
            let session = incomplete_dkg_session.borrow();
            if (dkg::session_dealer_epoch(session) == reconfiguration::current_epoch()) {
                return
            }
        };
        // V1 prologue dispatch means chunky DKG is not running this attempt;
        // drop any stale chunky session so finish_with_dkg_result can proceed
        // (e.g., recovery from a stall via local chunky_dkg_override_seq_num).
        if (chunky_dkg::incomplete_session().is_some()) {
            let framework = create_signer::create_signer(@aptos_framework);
            chunky_dkg::try_clear_incomplete_session(&framework);
        };

        reconfiguration_state::on_reconfig_start();
        let cur_epoch = reconfiguration::current_epoch();
        dkg::start(
            cur_epoch,
            randomness_config::current(),
            stake::cur_validator_consensus_infos(),
            stake::next_validator_consensus_infos()
        );
    }

    /// Trigger a reconfiguration with DKG and Chunky DKG.
    /// Do nothing if reconfiguration is already in progress; otherwise start both DKG and Chunky DKG.
    public(friend) fun try_start_with_chunky_dkg() {
        if (reconfiguration_state::is_in_progress()) { return };

        reconfiguration_state::on_reconfig_start();

        let cur_epoch = reconfiguration::current_epoch();
        dkg::start(
            cur_epoch,
            randomness_config::current(),
            stake::cur_validator_consensus_infos(),
            stake::next_validator_consensus_infos()
        );
        chunky_dkg::start(
            cur_epoch,
            chunky_dkg_config::current(),
            stake::cur_validator_consensus_infos(),
            stake::next_validator_consensus_infos()
        );
    }

    /// Clear incomplete DKG session, if it exists.
    /// Apply buffered on-chain configs (except for ValidatorSet, which is done inside `reconfiguration::reconfigure()`).
    /// Re-enable validator set changes.
    /// Run the default reconfiguration to enter the new epoch.
    public(friend) fun finish(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        dkg::try_clear_incomplete_session(framework);
        chunky_dkg::try_clear_incomplete_session(framework);
        consensus_config::on_new_epoch(framework);
        execution_config::on_new_epoch(framework);
        gas_schedule::on_new_epoch(framework);
        std::version::on_new_epoch(framework);
        features::on_new_epoch(framework);
        jwk_consensus_config::on_new_epoch(framework);
        jwks::on_new_epoch(framework);
        keyless_account::on_new_epoch(framework);
        randomness_config_seqnum::on_new_epoch(framework);
        randomness_config::on_new_epoch(framework);
        randomness_api_v0_config::on_new_epoch(framework);
        chunky_dkg_config_seqnum::on_new_epoch(framework);
        chunky_dkg_config::on_new_epoch(framework);
        decryption::on_new_epoch(framework);
        reconfiguration::reconfigure();
    }

    /// Single decision point for completing the in-progress reconfig.
    /// Calls finish(account) iff:
    /// - reconfiguration is in progress, AND
    /// - DKG has no in-progress session, AND
    /// - Chunky DKG has no in-progress session, OR the configured grace period
    ///   (shadow mode) has elapsed since the chunky session started.
    /// No-op otherwise. Callers (finish_with_dkg_result,
    /// finish_with_chunky_dkg_result, try_complete_after_grace_period) just
    /// signal "something may have changed" and let this function decide.
    fun try_finalize_reconfig(account: &signer) {
        if (!reconfiguration_state::is_in_progress()) { return };

        // DKG must be done.
        if (dkg::incomplete_session().is_some()) { return };

        // Chunky DKG must be done OR its grace period (shadow mode) must have elapsed.
        let chunky_session = chunky_dkg::incomplete_session();
        if (chunky_session.is_some()) {
            let grace_period = chunky_dkg_config::grace_period_secs();
            if (grace_period.is_none()) { return };
            let start_time_us = chunky_dkg::session_start_time(chunky_session.borrow());
            let grace_period_us = (*grace_period.borrow()) * 1_000_000;
            if (timestamp::now_microseconds() - start_time_us < grace_period_us) {
                return
            };
        };

        finish(account);
    }

    /// Complete the current DKG session with the given result.
    /// Aborts if no DKG session is in progress.
    fun finish_with_dkg_result(account: &signer, dkg_result: vector<u8>) {
        dkg::finish(dkg_result);
        try_finalize_reconfig(account);
    }

    /// Complete the current Chunky DKG session with the given result.
    /// No-op if Chunky DKG is not enabled.
    /// Buffers the derived encryption key for the next epoch.
    fun finish_with_chunky_dkg_result(
        account: &signer, chunky_dkg_result: vector<u8>, encryption_key: vector<u8>
    ) {
        if (!chunky_dkg_config::enabled()) {
            return;
        };

        chunky_dkg::finish(chunky_dkg_result);
        let next_epoch = reconfiguration::current_epoch() + 1;
        decryption::set_for_next_epoch(next_epoch, encryption_key);
        try_finalize_reconfig(account);
    }

    /// Periodic finalization tick: try to advance the in-progress reconfig.
    /// Called from block_prologue_ext / block_prologue_ext_v2 every block
    /// after the epoch interval has elapsed. In V2 mode, also gives the
    /// grace-period (shadow-mode) escape a chance to fire. In V1 mode after
    /// a chunky-only override recovery, lets the reconfig finalize without
    /// re-dealing DKG (dkg::start is idempotent per epoch).
    public(friend) fun try_advance_reconfig() {
        let framework = create_signer::create_signer(@aptos_framework);
        try_finalize_reconfig(&framework);
    }
}
