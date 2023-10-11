module aptos_framework::dkg {
    friend aptos_framework::reconfiguration;

    public(friend) fun update(_params: vector<u8>): bool {
        true //TODO: real logic
    }
}
