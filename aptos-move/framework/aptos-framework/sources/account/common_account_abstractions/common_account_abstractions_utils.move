module aptos_framework::common_account_abstractions_utils {
    use std::chain_id;
    use std::string_utils;

    friend aptos_framework::ethereum_derivable_account;
    friend aptos_framework::solana_derivable_account;

    public(friend) fun network_name(): vector<u8> {
        let chain_id = chain_id::get();
        if (chain_id == 1) {
            b"mainnet"
        } else if (chain_id == 2) {
            b"testnet"
        } else if (chain_id == 4) {
            b"local"
        } else {
            let network_name = &mut vector[];
            network_name.append(b"custom network: ");
            network_name.append(*string_utils::to_string(&chain_id).bytes());
            *network_name
        }
    }
}
