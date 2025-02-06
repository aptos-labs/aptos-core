module aptos_framework::auth_data {
    use std::string::String;

    enum DomainAccount has copy, drop {
        V1 {
            domain_name: String,
            account_authentication_key: vector<u8>,
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

    public fun authenticator(self: &AbstractionAuthData): &vector<u8> {
        &self.authenticator
    }

    public fun is_domain(self: &AbstractionAuthData): bool {
        self is DomainV1
    }

    public fun domain_name(self: &AbstractionAuthData): &String {
        &self.account.domain_name
    }

    public fun account_authentication_key(self: &AbstractionAuthData): &vector<u8> {
        &self.account.account_authentication_key
    }
}
