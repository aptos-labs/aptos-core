module 0xABCD::permissioned_transfer {
    use aptos_framework::aptos_account;

    public entry fun transfer(
        source: &signer, to: address, amount: u64
    ) {
        aptos_account::transfer(source, to, amount);
    }
}
