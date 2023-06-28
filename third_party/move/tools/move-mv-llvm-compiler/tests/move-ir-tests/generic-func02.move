
module 0x100::Coins {
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

    public fun get_value_generic<Currency>(c: Coin<Currency>): u64 {
        let Coin<Currency> { value } = c;
        value
    }

    public fun mint_2coins_generic<C1, C2>(n1: u64, n2: u64): (Coin<C1>, Coin<C2>) {
        (Coin<C1> { value: n1 }, Coin<C2> { value: n2 })
   }
}

// Instantiate generic function from a different module.
//
// Also exercises generic structs as arguments and return values
// of generic functions (including returning multiple generic values).
module 0x200::M11 {
    use 0x100::Coins::Coin;

    // Currency Specifiers.
    struct Eth {}
    struct USDC {}

    fun mint_usdc(n: u64): Coin<USDC> {
        0x100::Coins::mint_generic<USDC>(n)
    }

    fun mint_2coins_usdc_and_eth(nu: u64, ne: u64): (Coin<USDC>, Coin<Eth>) {
        0x100::Coins::mint_2coins_generic<USDC, Eth>(nu, ne)
    }

    fun get_value_usdc(c: Coin<USDC>): u64 {
        0x100::Coins::get_value_generic<USDC>(c)
    }
}
