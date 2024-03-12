module aptos_framework::signing_data {
    enum SigningData has copy, drop {
        V1 { digest: vector<u8>, authenticator: vector<u8> },
    }

    #[test_only]
    public fun create_signing_data(digest: vector<u8>): SigningData {
        SigningData::V1 { digest }
    }

    public fun digest(signing_data: &SigningData): &vector<u8> {
        &signing_data.digest
    }

    public fun authenticator(signing_data: &SigningData): &vector<u8> {
        &signing_data.authenticator
    }
}
