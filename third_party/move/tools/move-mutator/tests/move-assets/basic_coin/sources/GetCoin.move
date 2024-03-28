script {
    use CafeAccount::BasicCoin;

    fun publish_balance(s: signer) {
        BasicCoin::publish_balance(&s);
    }
}

script {
    use CafeAccount::BasicCoin;

    fun mint_some(module_owner: signer, rx_addr: address, amount: u64) {
        BasicCoin::mint(&module_owner, rx_addr, amount);
    }
}

script {
    use CafeAccount::BasicCoin;

    fun test(module_owner: signer, rx_addr: address, amount: u64) {
        let amount2 = amount + 10;
    }
}