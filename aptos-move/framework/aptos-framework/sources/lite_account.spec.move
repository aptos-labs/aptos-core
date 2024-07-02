spec aptos_framework::lite_account {
    spec module {
        pragma verify = false;
    }

    spec fun spec_native_authenticator(addr: address): Option<vector<u8>>;
}
