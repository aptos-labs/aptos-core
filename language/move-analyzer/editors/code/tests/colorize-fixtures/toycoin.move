// Source code inspired by the Move book section on "creating coins."

address 0x2 {
    module Coin {
        struct Coin {
            value: u64,
        }

        public fun mint(value: u64): Coin {
            Coin { value }
        }

        public fun value(coin: &Coin): u64 {
            coin.value
        }

        public fun burn(coin: Coin): u64 {
            let Coin { value } = coin;
            value
        }
    }
}

script {
    use Std::Debug;
    use 0x2::Coin;

    fun main() {
        let coin = Coin::mint(100);
        Debug::print(&Coin::value(&coin));
        Coin::burn(coin);
    }
}
