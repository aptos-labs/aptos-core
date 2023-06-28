
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
    public fun mint_sol(value: u64): Coin<Sol> {
        Coin { value }
    }

    public fun mint_bitcoin_via_generic(n: u64): Coin<Bitcoin> {
        mint_generic<Bitcoin>(n)
    }

    public fun get_value_generic<Currency>(c: Coin<Currency>): u64 {
        let Coin<Currency> { value } = c;
        value
    }
}

script {
    fun main() {
        let some_sol = 0x100::M2::mint_sol(860);
        let t1 = 0x100::M2::get_value_generic<0x100::M2::Sol>(some_sol);
        assert!(t1 == 860, 0xf00);

        let some_btc = 0x100::M2::mint_bitcoin_via_generic(13);
        let t2 = 0x100::M2::get_value_generic<0x100::M2::Bitcoin>(some_btc);
        assert!(t2 == 13, 0xf01);
    }
}
