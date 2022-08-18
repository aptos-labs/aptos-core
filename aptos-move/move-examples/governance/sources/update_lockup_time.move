script {
  use aptos_framework::aptos_governance;
  use aptos_framework::governance_proposal::GovernanceProposal;
  use aptos_framework::staking_config;
  use aptos_framework::voting;

  fun main(proposal_id: u64) {
    let proposal = voting::resolve<GovernanceProposal>(@aptos_framework, proposal_id);
    let framework_signer = aptos_governance::get_signer(proposal, @aptos_framework);
    // Change recurring lockup to 10 day.
    staking_config::update_recurring_lockup_duration_secs(&framework_signer, 864000);
  }
}
