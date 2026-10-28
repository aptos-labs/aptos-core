module 0xbeef::limits {
    use aptos_framework::aptos_account;

    public entry fun spin(n: u64) {
        let i = 0;
        while (i < n) {
            i = i + 1;
        };
    }

    public entry fun spin_then_transfer(s: &signer, to: address, amount: u64, n: u64) {
        spin(n);
        aptos_account::transfer(s, to, amount);
    }
}
