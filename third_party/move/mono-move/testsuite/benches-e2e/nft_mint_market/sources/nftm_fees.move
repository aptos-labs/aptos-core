/// Three-way split of a sale price: marketplace commission, creator royalty,
/// and whatever is left over for the seller.
///
/// The royalty side reads the framework's `aptos_token_objects::royalty`
/// record through the token, which is where a real marketplace gets it and
/// which costs one more resource-group read per sale.
module bench::nftm_fees {
    use std::option;
    use std::signer;
    use aptos_framework::fungible_asset::{Self, FungibleAsset};
    use aptos_framework::object;
    use aptos_framework::primary_fungible_store;
    use aptos_token_objects::royalty;
    use aptos_token_objects::token::{Self, Token};

    /// Only the package address can hold the schedule.
    const E_NOT_BENCH: u64 = 1;
    /// Commission and royalty together would take more than the whole price.
    const E_FEES_TOO_HIGH: u64 = 2;

    /// Denominator both the commission and the royalty are quoted against.
    const BPS_DENOMINATOR: u64 = 10000;

    struct Schedule has key {
        commission_bps: u64,
        /// Royalty applied to a token whose collection carries none.
        royalty_bps: u64,
        fee_recipient: address,
    }

    public entry fun init_schedule(
        admin: &signer, commission_bps: u64, royalty_bps: u64
    ) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        assert!(
            commission_bps + royalty_bps <= BPS_DENOMINATOR, E_FEES_TOO_HIGH);
        move_to(
            admin,
            Schedule { commission_bps, royalty_bps, fee_recipient: @bench },
        );
    }

    /// Commission, default royalty, and the commission payee. Falls back to a
    /// zero schedule so that a sale can never abort on a missing one.
    public fun schedule(): (u64, u64, address) acquires Schedule {
        if (!exists<Schedule>(@bench)) return (0, 0, @bench);
        let schedule = borrow_global<Schedule>(@bench);
        (schedule.commission_bps, schedule.royalty_bps, schedule.fee_recipient)
    }

    /// `value * num / den`, computed wide and capped at `value`. A price near
    /// the top of `u64` would otherwise overflow the multiplication, and no
    /// single fee may take more than the whole price anyway.
    fun mul_div(value: u64, num: u64, den: u64): u64 {
        if (den == 0) return 0;
        let scaled = (value as u128) * (num as u128) / (den as u128);
        if (scaled > (value as u128)) value else (scaled as u64)
    }

    /// Commission, royalty and seller proceeds for `price`. Rounding is down
    /// on both fees, so the seller absorbs the remainder and the three parts
    /// always add back up to `price`.
    public fun split(
        price: u64, commission_bps: u64, royalty_num: u64, royalty_den: u64
    ): (u64, u64, u64) {
        let commission = mul_div(price, commission_bps, BPS_DENOMINATOR);
        let royalty = mul_div(price, royalty_num, royalty_den);
        // A commission and a royalty that together exceed the price would
        // otherwise underflow the proceeds.
        if (commission > price) {
            commission = price;
        };
        if (royalty > price - commission) {
            royalty = price - commission;
        };
        (commission, royalty, price - commission - royalty)
    }

    /// Royalty numerator, denominator and payee of `token`, falling back to
    /// `default_bps` paid to the package when the token carries none.
    public fun token_royalty(
        token_addr: address, default_bps: u64
    ): (u64, u64, address) {
        let maybe =
            token::royalty(object::address_to_object<Token>(token_addr));
        if (option::is_none(&maybe)) {
            return (default_bps, BPS_DENOMINATOR, @bench)
        };
        let royalty = option::destroy_some(maybe);
        (
            royalty::numerator(&royalty),
            royalty::denominator(&royalty),
            royalty::payee_address(&royalty),
        )
    }

    /// Pay `fa`, which must hold exactly `price`, out to the commission payee,
    /// the royalty payee and `seller`. Returns the three amounts.
    public fun payout(
        fa: FungibleAsset, price: u64, token_addr: address, seller: address
    ): (u64, u64, u64) acquires Schedule {
        let (commission_bps, default_royalty_bps, fee_recipient) = schedule();
        let (royalty_num, royalty_den, payee) =
            token_royalty(token_addr, default_royalty_bps);
        let (commission, royalty, proceeds) =
            split(price, commission_bps, royalty_num, royalty_den);
        if (commission > 0) {
            primary_fungible_store::deposit(
                fee_recipient, fungible_asset::extract(&mut fa, commission));
        };
        if (royalty > 0) {
            primary_fungible_store::deposit(
                payee, fungible_asset::extract(&mut fa, royalty));
        };
        primary_fungible_store::deposit(seller, fa);
        (commission, royalty, proceeds)
    }

    public fun bps_denominator(): u64 {
        BPS_DENOMINATOR
    }

    #[view]
    public fun commission_bps(): u64 acquires Schedule {
        let (commission_bps, _, _) = schedule();
        commission_bps
    }

    #[view]
    public fun default_royalty_bps(): u64 acquires Schedule {
        let (_, royalty_bps, _) = schedule();
        royalty_bps
    }
}
