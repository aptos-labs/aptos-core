/// Reconfiguration with DKG helper functions.
module aptos_framework::reconfiguration_with_dkg {
    use std::features;
    use aptos_framework::chunky_dkg;
    use aptos_framework::chunky_dkg_config;
    use aptos_framework::consensus_config;
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
        decryption::on_new_epoch(framework);
        reconfiguration::reconfigure();
    }

    /// Call finish(account) only when (1) reconfiguration is in progress, and
    /// (2) both DKG and Chunky DKG have no in-progress session.
    /// Guard (1) ensures we never run reconfiguration twice (after the first
    /// finish(account), reconfig is no longer in progress).
    fun maybe_finish_reconfig_with_chunky_dkg(account: &signer) {
        if (!reconfiguration_state::is_in_progress()) { return };
        let dkg_incomplete = dkg::incomplete_session();
        let chunky_incomplete = chunky_dkg::incomplete_session();
        if (dkg_incomplete.is_none() && chunky_incomplete.is_none()) {
            finish(account);
        }
    }

    /// Complete the current DKG session with the given result.
    /// Aborts if no DKG session is in progress.
    /// If Chunky DKG is enabled, finish(account) is invoked only once both DKG and Chunky DKG
    /// have no in-progress session; otherwise finish(account) is invoked immediately.
    fun finish_with_dkg_result(account: &signer, dkg_result: vector<u8>) {
        dkg::finish(dkg_result);
        if (chunky_dkg_config::enabled()) {
            maybe_finish_reconfig_with_chunky_dkg(account);
        } else {
            finish(account);
        }
    }

    /// Complete the current Chunky DKG session with the given result.
    /// No-op if Chunky DKG is not enabled.
    /// Buffers the derived encryption key for the next epoch.
    /// finish(account) is invoked only when both DKG and Chunky DKG have no in-progress session
    /// (via maybe_finish_reconfig_with_chunky_dkg).
    fun finish_with_chunky_dkg_result(
        account: &signer, chunky_dkg_result: vector<u8>, encryption_key: vector<u8>
    ) {
        if (!chunky_dkg_config::enabled()) {
            return;
        };

        chunky_dkg::finish(chunky_dkg_result);
        let next_epoch = reconfiguration::current_epoch() + 1;
        decryption::set_for_next_epoch(next_epoch, encryption_key);
        maybe_finish_reconfig_with_chunky_dkg(account);
    }
}
