/// Define the GovernanceProposal that will be used as part of on-chain governance by VelorGovernance.
///
/// This is separate from the VelorGovernance module to avoid circular dependency between VelorGovernance and Stake.
module velor_framework::governance_proposal {
    friend velor_framework::velor_governance;

    struct GovernanceProposal has store, drop {}

    /// Create and return a GovernanceProposal resource. Can only be called by VelorGovernance
    public(friend) fun create_proposal(): GovernanceProposal {
        GovernanceProposal {}
    }

    /// Useful for VelorGovernance to create an empty proposal as proof.
    public(friend) fun create_empty_proposal(): GovernanceProposal {
        create_proposal()
    }

    #[test_only]
    public fun create_test_proposal(): GovernanceProposal {
        create_empty_proposal()
    }
}
