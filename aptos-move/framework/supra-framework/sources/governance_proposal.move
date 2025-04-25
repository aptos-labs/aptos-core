/// Define the GovernanceProposal that will be used as part of on-chain governance by SupraGovernance.
///
/// This is separate from the SupraGovernance module to avoid circular dependency between SupraGovernance and Stake.
module supra_framework::governance_proposal {
    friend supra_framework::supra_governance;

    struct GovernanceProposal has store, drop {}

    /// Create and return a GovernanceProposal resource. Can only be called by SupraGovernance
    public(friend) fun create_proposal(): GovernanceProposal {
        GovernanceProposal {}
    }

    /// Useful for SupraGovernance to create an empty proposal as proof.
    public(friend) fun create_empty_proposal(): GovernanceProposal {
        create_proposal()
    }

    #[test_only]
    public fun create_test_proposal(): GovernanceProposal {
        create_empty_proposal()
    }
}
