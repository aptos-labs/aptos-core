module 0x42::example {
	struct Coin(u64) has key; // semicolon needed

enum Option<T> {
    None,
    Some(T),
}

fun create_coin(): Coin {
    Coin(0)
}

fun some<T>(x: T): Option<T> {
    Option::Some(x)
}

fun unwrap_coin_0(x: Coin): u64 {
    x.0
}

fun inc_coin(x: &mut Coin) {
    x.0 = x.0 + 1;
}

fun unwrap_coin_1(x: Coin): u64 {
    let Coin(x) = x;
    x
}

fun inc_coin_2(x: &mut Coin) {
    let Coin(x) = x;
	*x = *x + 1;
}

fun unwrap_option<T>(self: Option<T>): T {
    match (self) {
        Option::None => abort 0,
        Option::Some(x) => x,
    }
}
}
