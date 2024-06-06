// This module implements a simple reserve-backed currency system.
module defi::reserve {
    use std::fixed_point32::{Self, FixedPoint32};
    const ADMIN: address = @defi;

    struct Coin1Info has key {
        total_value: u64,
        reserve_coin2: ReserveComponent,
    }
    spec Coin1Info {
        // Safety property: the backing value always satisfies the backing ratio.
        invariant fixed_point32::spec_multiply_u64(total_value, reserve_coin2.backing_ratio)
            <= reserve_coin2.backing_value;
    }

    struct ReserveComponent has store {
        backing_value: u64,
        backing_ratio: FixedPoint32,
    }

    // Mint Coin1 by providing Coin2 as backing.
    // For simplicity, `backing_coin2` should be the exact amount of Coin2 to reserve for the minted Coin1.
    public fun mint_coin1(amount_to_mint: u64, backing_coin2: u64): u64 // returns the minted Coin1.
    acquires Coin1Info {
        assert!(amount_to_mint > 0, 1);
        let coin1info = borrow_global_mut<Coin1Info>(ADMIN);
        let coin2_amount_to_reserve = fixed_point32::multiply_u64(amount_to_mint, *& coin1info.reserve_coin2.backing_ratio) + 1;
        assert!(backing_coin2 == coin2_amount_to_reserve, 2);
        coin1info.reserve_coin2.backing_value = coin1info.reserve_coin2.backing_value + coin2_amount_to_reserve;
        coin1info.total_value = coin1info.total_value + amount_to_mint;
        amount_to_mint
    }
    spec mint_coin1 {
        let coin1info = global<Coin1Info>(ADMIN);
        let coin2_amount_to_reserve = fixed_point32::spec_multiply_u64(amount_to_mint, coin1info.reserve_coin2.backing_ratio) + 1;
        aborts_if amount_to_mint == 0;
        aborts_if backing_coin2 != coin2_amount_to_reserve;
        aborts_if global<Coin1Info>(ADMIN).total_value + amount_to_mint > MAX_U64;
        aborts_if coin1info.reserve_coin2.backing_value + backing_coin2 > MAX_U64;
        aborts_if !exists<Coin1Info>(ADMIN);

        ensures global<Coin1Info>(ADMIN).total_value == old(global<Coin1Info>(ADMIN).total_value) + amount_to_mint;
        ensures global<Coin1Info>(ADMIN).reserve_coin2.backing_value == old(global<Coin1Info>(ADMIN).reserve_coin2.backing_value) + backing_coin2;
        ensures backing_coin2 == coin2_amount_to_reserve;
    }

    // Mint Coin1 by providing Coin2 as backing.
    // This function is incorrect because it does not reserve enough Coin2 due to miscalculation.
    public fun mint_coin1_incorrect(amount_to_mint: u64, backing_coin2: u64): u64 // returns the minted Coin1.
    acquires Coin1Info {
        assert!(amount_to_mint > 0, 1);
        let coin1info = borrow_global_mut<Coin1Info>(ADMIN);
        let coin2_amount_to_reserve = fixed_point32::multiply_u64(amount_to_mint, *& coin1info.reserve_coin2.backing_ratio);
        assert!(backing_coin2 == coin2_amount_to_reserve, 2);
        coin1info.reserve_coin2.backing_value = coin1info.reserve_coin2.backing_value + coin2_amount_to_reserve;
        coin1info.total_value = coin1info.total_value + amount_to_mint;
        amount_to_mint
    }

    // Burn Coin1 and get back Coin2.
    public fun burn_coin1(amount_to_burn: u64): u64 // returns the Coin2 that was reserved.
    acquires Coin1Info {
        let coin1info = borrow_global_mut<Coin1Info>(ADMIN);
        let coin2_amount_to_return = fixed_point32::multiply_u64(amount_to_burn, *& coin1info.reserve_coin2.backing_ratio);
        assert!(coin1info.reserve_coin2.backing_value >= coin2_amount_to_return, 1);
        coin1info.reserve_coin2.backing_value = coin1info.reserve_coin2.backing_value - coin2_amount_to_return;
        coin1info.total_value = coin1info.total_value - amount_to_burn;
        coin2_amount_to_return
    }

    // Burn Coin1 and get back Coin2.
    // This function is incorrect because it does not reserve enough Coin2 due to miscalculation.
    public fun burn_coin1_incorrect(amount_to_burn: u64): u64 // returns the Coin2 that was reserved.
    acquires Coin1Info {
        let coin1info = borrow_global_mut<Coin1Info>(ADMIN);
        let coin2_amount_to_return = fixed_point32::multiply_u64(amount_to_burn, *& coin1info.reserve_coin2.backing_ratio) + 1;
        assert!(coin1info.reserve_coin2.backing_value >= coin2_amount_to_return, 1);
        coin1info.reserve_coin2.backing_value = coin1info.reserve_coin2.backing_value - coin2_amount_to_return;
        coin1info.total_value = coin1info.total_value - amount_to_burn;
        coin2_amount_to_return
    }

    #[verify_only]
    // This theorems shows that one cannot take out more backing coins from the reserve than the backing value.
    fun mint_and_burn(amount_to_mint: u64, backing_coin2: u64): u64 acquires Coin1Info{
        let coin1 = mint_coin1(amount_to_mint, backing_coin2);
        let coin2 = burn_coin1(coin1);
        coin2
    }
    spec mint_and_burn {
        ensures result <= backing_coin2;
    }

    spec module {
        // We assume that the backing ratio is fixed at 1:3, and never changes.
        invariant exists<Coin1Info>(ADMIN) ==>
            global<Coin1Info>(ADMIN).reserve_coin2.backing_ratio == fixed_point32::spec_create_from_rational(1, 5);
    }
}
