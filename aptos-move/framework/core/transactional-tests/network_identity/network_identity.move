//# init --parent-vasps Bob

// Test to check that we can write identity into the NetworkIdentity and update it.

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
    use DiemFramework::NetworkIdentity;

    fun main(_dr: signer, tc: signer) {
        // Initialize event handle
        NetworkIdentity::initialize_network_identity_event_handle(&tc);
    }
}

//# run --admin-script --signers DiemRoot TreasuryCompliance --show-events
script {
    use DiemFramework::NetworkIdentity;
    use Std::Signer;
    use Std::Vector;

    fun main(_dr: signer, tc: signer) {
        let tc = &tc;
        let input = b"id_1";

        // Add identities
        NetworkIdentity::add_identities(tc, Vector::singleton(copy input));

        let identities: vector<vector<u8>> = NetworkIdentity::get(Signer::address_of(tc));
        assert!(Vector::contains(&identities, &input), 0);
    }
}

//# run --admin-script --signers DiemRoot TreasuryCompliance --show-events
script{
    use DiemFramework::NetworkIdentity;
    use Std::Signer;
    use Std::Vector;

    fun main(_dr: signer, tc: signer) {
        let tc = &tc;
        let addr = Signer::address_of(tc);
        let original = b"id_1";
        let input = b"id_2";

        // Ensure that original value still exists before changing
        let identities: vector<vector<u8>> = NetworkIdentity::get(copy addr);
        assert!(Vector::contains(&identities, &original), 0);

        NetworkIdentity::add_identities(tc, Vector::singleton(copy input));
        let identities: vector<vector<u8>> = NetworkIdentity::get(addr);
        assert!(Vector::contains(&identities, &original), 0);
        assert!(Vector::contains(&identities, &input), 0);
    }
}

//# run --admin-script --signers DiemRoot TreasuryCompliance --show-events
script{
    use DiemFramework::NetworkIdentity;
    use Std::Signer;
    use Std::Vector;

    fun main(_dr: signer, tc: signer) {
        let tc = &tc;
        let addr = Signer::address_of(tc);
        let original = b"id_1";
        let input = b"id_2";

        NetworkIdentity::remove_identities(tc, Vector::singleton(copy input));
        let identities: vector<vector<u8>> = NetworkIdentity::get(addr);
        assert!(Vector::contains(&identities, &original), 0);
        assert!(!Vector::contains(&identities, &input), 0);
    }
}
