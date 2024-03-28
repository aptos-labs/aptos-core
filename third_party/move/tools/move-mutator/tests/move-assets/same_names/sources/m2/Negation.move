module TestAccount::Negation_m2 {
    fun neg_log(x: bool): bool {
        !x
    }

    spec neg_log {
        ensures result == !x;
    }
}
