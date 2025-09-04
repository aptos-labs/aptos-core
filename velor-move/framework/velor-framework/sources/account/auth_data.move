module velor_framework::auth_data {
    use std::error;

    const ENOT_REGULAR_AUTH_DATA: u64 = 1;
    const ENOT_DERIVABLE_AUTH_DATA: u64 = 2;

    enum AbstractionAuthData has copy, drop {
        V1 {
            digest: vector<u8>,
            authenticator: vector<u8>
        },
        DerivableV1 {
            digest: vector<u8>,
            abstract_signature: vector<u8>,
            abstract_public_key: vector<u8>,
        },
    }

    #[test_only]
    public fun create_auth_data(digest: vector<u8>, authenticator: vector<u8>): AbstractionAuthData {
        AbstractionAuthData::V1 { digest, authenticator }
    }

    public fun digest(self: &AbstractionAuthData): &vector<u8> {
        &self.digest
    }

    // separate authenticator and derivable_authenticator - to not allow accidental mixing
    // in user authentication code

    #[test_only]
    public fun create_derivable_auth_data(
        digest: vector<u8>,
        abstract_signature: vector<u8>,
        abstract_public_key: vector<u8>
    ): AbstractionAuthData {
        AbstractionAuthData::DerivableV1 { digest, abstract_signature, abstract_public_key }
    }

    public fun authenticator(self: &AbstractionAuthData): &vector<u8> {
        assert!(self is V1, error::invalid_argument(ENOT_REGULAR_AUTH_DATA));
        &self.authenticator
    }

    public fun is_derivable(self: &AbstractionAuthData): bool {
        self is DerivableV1
    }

    public fun derivable_abstract_signature(self: &AbstractionAuthData): &vector<u8> {
        assert!(self is DerivableV1, error::invalid_argument(ENOT_REGULAR_AUTH_DATA));
        &self.abstract_signature
    }

    public fun derivable_abstract_public_key(self: &AbstractionAuthData): &vector<u8> {
        assert!(self is DerivableV1, error::invalid_argument(ENOT_DERIVABLE_AUTH_DATA));
        &self.abstract_public_key
    }
}
