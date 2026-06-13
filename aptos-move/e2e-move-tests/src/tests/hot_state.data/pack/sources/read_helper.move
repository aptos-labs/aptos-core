module 0xcafe::read_helper {
    use aptos_framework::account;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;

    /// Reads state without writing it, so the touched slots stay eligible for hot
    /// state promotion: the account resource of `target`, plus `CoinInfo<AptosCoin>`
    /// and the table-backed coin-to-fungible-asset conversion map behind
    /// `coin::supply`.
    public entry fun read_only(target: address) {
        let _ = account::get_sequence_number(target);
        let _ = coin::supply<AptosCoin>();
    }
}
