/// Define the GovernanceProposal that will be used as part of on-chain governance by AptosGovernance.
///
/// This is separate from the AptosGovernance module to avoid circular dependency between AptosGovernance and Stake.
module AptosFramework::GovernanceProposal {
    friend AptosFramework::AptosGovernance;
    use Std::ASCII::{String, string};

    struct GovernanceProposal has store, drop {
        /// The location (e.g. url) where the proposal resolution script's code can be accessed.
        /// Maximum length allowed is 256 chars.
        code_location: String,
        /// Description of the proposal.
        /// Maximum length allowed is 256 chars.
        title: String,
        /// Description of the proposal.
        /// Maximum length allowed is 256 chars.
        description: String,
    }

    /// Create and return a GovernanceProposal resource. Can only be called by AptosGovernance
    public(friend) fun create_proposal(
        code_location: String,
        title: String,
        description: String,
    ): GovernanceProposal {
        GovernanceProposal {
            code_location,
            title,
            description
        }
    }

    /// Useful for AptosGovernance to create an empty proposal as proof.
    public(friend) fun create_empty_proposal(): GovernanceProposal {
        create_proposal(string(b""), string(b""), string(b""))
    }

    #[test_only]
    public fun create_test_proposal(): GovernanceProposal {
        create_proposal(
            string(b""),
            string(b""),
            string(b""),
        )
    }
}
