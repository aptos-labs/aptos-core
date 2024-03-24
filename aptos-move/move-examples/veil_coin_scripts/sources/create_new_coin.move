script {
    use aptos_framework::managed_coin;
    use 0x9ac6e327c67207e641be69940ab4abfe7a2920c969185ae2b8515e4fa45dce01::gem_coin;

    fun main(account: &signer) {
        // managed_coin::register<gem_coin::GemCoin>(account);
        //managed_coin::mint<gem_coin::GemCoin>(account, @0x9ac6e327c67207e641be69940ab4abfe7a2920c969185ae2b8515e4fa45dce01, 10000000000);
        //managed_coin::mint<gem_coin::GemCoin>(account, @0xe7194662d2006cf307eafbb00387137dc95dfdd228ae0c8158cf3dfabb9972a0, 10000000000);
        managed_coin::mint<gem_coin::GemCoin>(account, @0xe7194662d2006cf307eafbb00387137dc95dfdd228ae0c8158cf3dfabb9972a0, 10000000000);

    }
}
