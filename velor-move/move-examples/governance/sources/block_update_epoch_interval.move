script {
    use velor_framework::velor_governance;
    use velor_framework::block;

    fun main(proposal_id: u64) {
        let framework_signer = velor_governance::resolve(proposal_id, @velor_framework);
        // Update epoch interval to 2 hours.
        let epoch_interval_secs = 2 * 60 * 60;
        let epoch_interval_microsecs = epoch_interval_secs * 1000000;
        block::update_epoch_interval_microsecs(&framework_signer, epoch_interval_microsecs);
    }
}
