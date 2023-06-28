
module 0x100::M2 {
    // Currency Specifiers
    struct Sol {}
    struct Bitcoin {}

    // A generic coin type that can be instantiated using a currency
    // specifier type.
    //   e.g. Coin<Currency1>, Coin<Currency2> etc.
    struct Coin<phantom Currency> has store {
        value: u64
    }

    // Write code generically about all currencies
    public fun mint_generic<Currency>(value: u64): Coin<Currency> {
        Coin { value }
    }

    // Write code concretely about one currency
    public fun mint_concrete(value: u64): Coin<Sol> {
        Coin { value }
    }

    fun call_mint_generic(): Coin<Bitcoin> {
        mint_generic<Bitcoin>(4)
    }
}
