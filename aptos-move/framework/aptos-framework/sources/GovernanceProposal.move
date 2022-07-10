/// Define the GovernanceProposal that will be used as part of on-chain governance by AptosGovernance.
///
/// This is separate from the AptosGovernance module to avoid circular dependency between AptosGovernance and Stake.
module AptosFramework::GovernanceProposal {
    friend AptosFramework::AptosGovernance;

    struct GovernanceProposal has store, drop {}

    /// Create and return a GovernanceProposal resource. Can only be called by AptosGovernance
    public(friend) fun create_proposal(): GovernanceProposal {
        GovernanceProposal {}
    }

    #[test_only]
    public fun create_test_proposal(): GovernanceProposal {
        create_proposal()
    }
}
