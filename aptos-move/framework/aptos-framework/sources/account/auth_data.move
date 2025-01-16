module aptos_framework::auth_data {
    enum AbstractionAuthData has copy, drop {
        V1 { digest: vector<u8>, authenticator: vector<u8> },
    }

    #[test_only]
    public fun create_auth_data(digest: vector<u8>, authenticator: vector<u8>): AbstractionAuthData {
        AbstractionAuthData::V1 { digest, authenticator }
    }

    public fun digest(signing_data: &AbstractionAuthData): &vector<u8> {
        &signing_data.digest
    }

    public fun authenticator(signing_data: &AbstractionAuthData): &vector<u8> {
        &signing_data.authenticator
    }
}
