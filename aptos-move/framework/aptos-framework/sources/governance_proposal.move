/// Define the GovernanceProposal that will be used as part of on-chain governance by AptosGovernance.
///
/// This is separate from the AptosGovernance module to avoid circular dependency between AptosGovernance and Stake.
module aptos_framework::governance_proposal {
    friend aptos_framework::aptos_governance;

    use std::string::{String, length, utf8};
    use std::error;

    const ECODE_LOCATION_TOO_LONG: u64 = 1;
    const ETITLE_TOO_LONG: u64 = 2;
    const EDESCRIPTION_TOO_LONG: u64 = 3;

    struct GovernanceProposal has store, drop {
        /// The location (e.g. url) where the proposal resolution script's code can be accessed.
        /// Maximum length allowed is 256 chars.
        code_location: String,
        /// Title of the proposal.
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
        assert!(length(&code_location) <= 256, error::invalid_argument(ECODE_LOCATION_TOO_LONG));
        assert!(length(&title) <= 256, error::invalid_argument(ETITLE_TOO_LONG));
        assert!(length(&description) <= 256, error::invalid_argument(EDESCRIPTION_TOO_LONG));

        GovernanceProposal {
            code_location,
            title,
            description
        }
    }

    /// Useful for AptosGovernance to create an empty proposal as proof.
    public(friend) fun create_empty_proposal(): GovernanceProposal {
        create_proposal(utf8(b""), utf8(b""), utf8(b""))
    }

    #[test_only]
    public fun create_test_proposal(): GovernanceProposal {
        create_empty_proposal()
    }

    #[test]
    #[expected_failure(abort_code = 65537)]
    public fun test_code_location_too_long(): GovernanceProposal {
        create_proposal(
            utf8(b"123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789"),
            utf8(b""),
            utf8(b""),
        )
    }

    #[test]
    #[expected_failure(abort_code = 65538)]
    public fun test_title_too_long(): GovernanceProposal {
        create_proposal(
            utf8(b""),
            utf8(b"123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789"),
            utf8(b""),
        )
    }

    #[test]
    #[expected_failure(abort_code = 65539)]
    public fun test_description_too_long(): GovernanceProposal {
        create_proposal(
            utf8(b""),
            utf8(b""),
            utf8(b"123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789"),
        )
    }
}
