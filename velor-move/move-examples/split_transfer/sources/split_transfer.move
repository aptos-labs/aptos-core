script {
    use velor_framework::coin;

    // There are two ways to approach this problem
    // 1. Withdraw the total then distribute the pieces by breaking it up or
    // 2. Transfer for each amount individually
    fun main<CoinType>(sender: &signer, receiver_a: address, receiver_b: address, amount: u64) {
        let coins = coin::withdraw<CoinType>(sender, amount);

        let coins_a = coin::extract(&mut coins, amount / 2);
        coin::deposit(receiver_a, coins_a);
        coin::deposit(receiver_b, coins);
    }
}
