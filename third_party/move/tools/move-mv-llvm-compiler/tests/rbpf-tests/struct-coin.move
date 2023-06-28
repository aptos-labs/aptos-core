
// A slightly modified example from below, along with a driver script to test it.
// https://move-language.github.io/move/structs-and-resources.html#example-1-coin

module 0x102::m {
    struct Coin has store {
        value: u64,
    }

    public fun mint(value: u64): Coin {
        // You would want to gate this function with some form of access control to prevent
        // anyone using this module from minting an infinite amount of coins.
        Coin { value }
    }

    public fun withdraw(coin: &mut Coin, amount: u64): Coin {
        assert!(coin.value >= amount, 1000);
        coin.value = coin.value - amount;
        Coin { value: amount }
    }

    public fun deposit(coin: &mut Coin, other: Coin) {
        let Coin { value } = other;
        coin.value = coin.value + value;
    }

    public fun split(coin: Coin, amount: u64): (Coin, Coin) {
        let other = withdraw(&mut coin, amount);
        (coin, other)
    }

    public fun merge(coin1: Coin, coin2: Coin): Coin {
        deposit(&mut coin1, coin2);
        coin1
    }

    public fun destroy_zero(coin: Coin) {
        let Coin { value } = coin;
        assert!(value == 0, 1001);
    }

    public fun get_value(coin: &Coin): u64 {
      coin.value
    }

    public fun burn(coin: &mut Coin) {
      coin.value = 0;
    }
}

script {
    use 0x102::m;

    fun main() {
        let c1 = m::mint(600);
        assert!(m::get_value(&c1) == 600, 0xf00);

        let c2 = m::withdraw(&mut c1, 200);
        assert!(m::get_value(&c1) == 400, 0xf01);
        assert!(m::get_value(&c2) == 200, 0xf02);

        m::deposit(&mut c1, c2);
        assert!(m::get_value(&c1) == 600, 0xf03);

        let (sc1, sc2) = m::split(c1, 75);
        assert!(m::get_value(&sc1) == 525, 0xf04);
        assert!(m::get_value(&sc2) == 75, 0xf05);

        let mc1 = m::merge(sc1, sc2);
        assert!(m::get_value(&mc1) == 600, 0xf06);

        m::burn(&mut mc1);
        m::destroy_zero(mc1);
    }
}
