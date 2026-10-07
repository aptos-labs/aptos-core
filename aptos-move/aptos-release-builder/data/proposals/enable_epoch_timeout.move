// Enable the epoch timeout watchdog with a 1-hour grace period.
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::epoch_timeout_config;

    const GRACE_PERIOD_SECS: u64 = 3600;

    fun main(proposal_id: u64) {
        let framework = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );

        // The watchdog may bypass both ordinary DKG and Chunky DKG once
        // reconfiguration has been in progress for GRACE_PERIOD_SECS.
        // This buffered setting only becomes active at the next epoch.
        epoch_timeout_config::set_for_next_epoch(
            &framework,
            epoch_timeout_config::new_with_grace_period(GRACE_PERIOD_SECS),
        );
        aptos_governance::reconfigure(&framework);
    }
}
