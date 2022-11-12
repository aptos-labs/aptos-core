script {
    fun register(account: &signer) {
        aptos_framework::managed_coin::register<MoonCoin::moon_coin::MoonCoin>(account)
    }
}
