/// Define the GovernanceProposal that will be used as part of on-chain governance by AptosGovernance.
///
/// This is separate from the AptosGovernance module to avoid circular dependency between AptosGovernance and Stake.
module aptos_framework::governance_proposal {
    friend aptos_framework::aptos_governance;

    use std::string::{String, length, utf8};
    use std::error;

    const ETOO_LONG: u64 = 1;

    struct GovernanceProposal has store, drop {
        // Location where metadata such as the proposal's execution script content, description, etc. are hosted.
        metadata_location: String,
        // The hash of the metadata to allow easy verification when a user votes that the metadata hosted at a url is
        // correct.
        metadata_hash: String,
    }

    /// Create and return a GovernanceProposal resource. Can only be called by AptosGovernance
    public(friend) fun create_proposal(
        metadata_location: String,
        metadata_hash: String,
    ): GovernanceProposal {
        assert!(length(&metadata_location) <= 256, error::invalid_argument(ETOO_LONG));
        assert!(length(&metadata_hash) <= 256, error::invalid_argument(ETOO_LONG));

        GovernanceProposal {
            metadata_location,
            metadata_hash,
        }
    }

    /// Useful for AptosGovernance to create an empty proposal as proof.
    public(friend) fun create_empty_proposal(): GovernanceProposal {
        create_proposal(utf8(b""), utf8(b""))
    }

    #[test_only]
    public fun create_test_proposal(): GovernanceProposal {
        create_empty_proposal()
    }

    #[test]
    #[expected_failure(abort_code = 65537)]
    public fun test_metadata_url_too_long(): GovernanceProposal {
        create_proposal(
            utf8(b"123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789"),
            utf8(b""),
        )
    }

    #[test]
    #[expected_failure(abort_code = 65537)]
    public fun test_metadata_hash_too_long(): GovernanceProposal {
        create_proposal(
            utf8(b""),
            utf8(b"123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789123456789"),
        )
    }
}
