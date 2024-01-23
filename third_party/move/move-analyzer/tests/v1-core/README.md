# AnimeSwap

**AnimeSwap** is AMM protocol for [Aptos](https://www.aptos.com/) blockchain. 

* [Contracts documents](https://docs.animeswap.org/docs/contracts/Aptos/contracts)
* [SDK](https://github.com/AnimeSwap/v1-sdk)

The current repository contains: 

* u256
* uq64x64
* TestCoin
* Faucet
* LPCoin
* LPResourceAccount
* Swap

## Add as dependency

Update your `Move.toml` with

```toml
[dependencies.AnimeSwap]
git = 'https://github.com/AnimeSwap/v1-core.git'
rev = 'v1.0.1'
subdir = 'Swap'
```

-----

Swap example:
```move
// swap exact coin to maximal coin
use SwapDeployer::AnimeSwapPoolV1;
...
// swap `amount_in` X to Y
let amount_in = 100000;
let coins_in = coin::withdraw(&account, amount_in);
let coins_out = AnimeSwapPoolV1::swap_coins_for_coins<X, Y>(coins_in);
```
or
```move
// swap minimal coin to exact coin (maybe some more dust)
use SwapDeployer::AnimeSwapPoolV1;
...
// swap X to `amount_out` Y
let amount_out = 100000;
let amount_in = AnimeSwapPoolV1::get_amounts_in_1_pair<X, Y>(amount_out);
// check if `amount_in` meets your demand
let coins_in = coin::withdraw(&account, amount_in);
let coins_out = AnimeSwapPoolV1::swap_coins_for_coins<X, Y>(coins_in);
// Because of discrete, coins_out value is actually `amount_out + dust`.
// Our protocol does not keep the dust, but return to user instead.
assert!(coin::value(&coins_out) >= amount_out, 1);
```

-----

Flash swap example:
```move
use SwapDeployer::AnimeSwapPoolV1Library;
use SwapDeployer::AnimeSwapPoolV1;
...
// loan `amount` Y and repay X
let amount = 100000;
let borrow_amount = AnimeSwapPoolV1::get_amounts_out_1_pair<X, Y>(amount);
let coins_out;
if (AnimeSwapPoolV1Library::compare<X, Y>()) {
    // flash loan Y
    let (coins_in_zero, coins_in, flash_swap) = AnimeSwapPoolV1::flash_swap<X, Y>(0, borrow_amount);
    coin::destroy_zero<X>(coins_in_zero);
    // do something with coins_in and get coins_out
    coins_out = f(coins_in);
    // repay X
    let repay_coins = coin::extract(&mut coins_out, amount);
    AnimeSwapPoolV1::pay_flash_swap<X, Y>(repay_coins, coin::zero<Y>(), flash_swap);
} else {
    // flash loan Y
    let (coins_in, coins_in_zero, flash_swap) = AnimeSwapPoolV1::flash_swap<Y, X>(borrow_amount, 0);
    coin::destroy_zero<X>(coins_in_zero);
    // do something with coins_in and get coins_out
    coins_out = f(coins_in);
    // repay X
    let repay_coins = coin::extract(&mut coins_out, amount);
    AnimeSwapPoolV1::pay_flash_swap<Y, X>(coin::zero<Y>(), repay_coins, flash_swap);
};
// keep the rest `coins_out`
```
or
```move
use SwapDeployer::AnimeSwapPoolV1Library;
use SwapDeployer::AnimeSwapPoolV1;
...
// loan `amount` X and repay Y
let amount = 100000;
let repay_amount = AnimeSwapPoolV1::get_amounts_in_1_pair<X, Y>(amount);
let coins_out;
if (AnimeSwapPoolV1Library::compare<X, Y>()) {
    // flash loan X
    let (coins_in, coins_in_zero, flash_swap) = AnimeSwapPoolV1::flash_swap<X, Y>(amount, 0);
    coin::destroy_zero<Y>(coins_in_zero);
    // do something with coins_in and get coins_out
    coins_out = f(coins_in);
    // repay Y
    let repay_coins = coin::extract(&mut coins_out, repay_amount);
    AnimeSwapPoolV1::pay_flash_swap<X, Y>(coin::zero<X>(), repay_coins, flash_swap);
} else {
    // flash loan X
    let (coins_in_zero, coins_in, flash_swap) = AnimeSwapPoolV1::flash_swap<Y, X>(0, amount);
    coin::destroy_zero<Y>(coins_in_zero);
    // do something with coins_in and get coins_out
    coins_out = f(coins_in);
    // repay Y
    let repay_coins = coin::extract(&mut coins_out, repay_amount);
    AnimeSwapPoolV1::pay_flash_swap<Y, X>(repay_coins, coin::zero<X>(), flash_swap);
};
// keep the rest `coins_out`
```