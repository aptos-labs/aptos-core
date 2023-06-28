
module 0x100::M2 {
    // Currency Specifiers
    struct Currency1 {}
    struct Currency2 {}

    // A generic coin type that can be instantiated using a currency
    // specifier type.
    //   e.g. Coin<Currency1>, Coin<Currency2> etc.
    struct Coin<Currency> has store {
        value: u64
    }

    // TODO: Enable this once generic functions are implemented.
    //
    // Write code generically about all currencies
    //public fun mint_generic<Currency>(value: u64): Coin<Currency> {
    //    Coin { value }
    //}

    // Write code concretely about one currency
    public fun mint_concrete(value: u64): Coin<Currency1> {
        Coin { value }
    }
}
