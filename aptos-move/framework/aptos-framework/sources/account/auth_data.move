module aptos_framework::auth_data {
    use std::error;

    const ENOT_REGULAR_AUTH_DATA: u64 = 1;
    const ENOT_DOMAIN_AUTH_DATA: u64 = 2;

    enum DomainAccount has copy, drop {
        V1 {
            account_identity: vector<u8>,
        }
    }

    enum AbstractionAuthData has copy, drop {
        V1 { digest: vector<u8>, authenticator: vector<u8> },
        DomainV1 { digest: vector<u8>, authenticator: vector<u8>, account: DomainAccount }
    }

    #[test_only]
    public fun create_auth_data(digest: vector<u8>, authenticator: vector<u8>): AbstractionAuthData {
        AbstractionAuthData::V1 { digest, authenticator }
    }

    public fun digest(self: &AbstractionAuthData): &vector<u8> {
        &self.digest
    }

    // separate authenticator and domain_authenticator - to not allow accidental mixing
    // in user authentication code

    public fun authenticator(self: &AbstractionAuthData): &vector<u8> {
        assert!(self is V1, error::invalid_argument(ENOT_REGULAR_AUTH_DATA));
        &self.authenticator
    }

    public fun is_domain(self: &AbstractionAuthData): bool {
        self is DomainV1
    }

    public fun domain_authenticator(self: &AbstractionAuthData): &vector<u8> {
        assert!(self is DomainV1, error::invalid_argument(ENOT_REGULAR_AUTH_DATA));
        &self.authenticator
    }

    public fun domain_account_identity(self: &AbstractionAuthData): &vector<u8> {
        assert!(self is DomainV1, error::invalid_argument(ENOT_DOMAIN_AUTH_DATA));
        &self.account.account_identity
    }
}
