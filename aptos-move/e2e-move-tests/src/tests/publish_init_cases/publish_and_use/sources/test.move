module 0xbeef::test {
    use aptos_framework::code::publish_package_txn;

    public entry fun run(
        s: &signer,
        metadata: vector<u8>,
        code: vector<vector<u8>>,
    ) {
        publish_package_txn(s, metadata, code);
    }
}
