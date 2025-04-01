// This file specifies the module `pool_u64`. It specifies the invariants of the struct Pool, and the pre/post-conditions
// of the functions.
spec aptos_std::pool_u64 {

    spec module {
        // TODO: Disabled due to the issue with the data invariant verification
        pragma verify = false;
    }
    // -----------------
    // Struct invariants
    // -----------------

    // The invariants of the struct Pool.
    spec Pool {
        // `shares` contains the key `addr` if and only if `shareholders` contains `addr.
        invariant forall addr: address:
            (simple_map::spec_contains_key(shares, addr) == vector::spec_contains(shareholders, addr));

        // `shareholders` does not contain duplicates.
        invariant forall i in 0..len(shareholders), j in 0..len(shareholders):
            shareholders[i] == shareholders[j] ==> i == j;
    }


    // -----------------------
    // Function specifications
    // -----------------------

    spec fun spec_contains(pool: Pool, shareholder: address): bool {
        simple_map::spec_contains_key(pool.shares, shareholder)
    }

    spec contains(self: &Pool, shareholder: address): bool {
        aborts_if false;
        ensures result == spec_contains(self, shareholder);
    }

    spec fun spec_shares(pool: Pool, shareholder: address): u64 {
        if (simple_map::spec_contains_key(pool.shares, shareholder)) {
            simple_map::spec_get(pool.shares, shareholder)
        }
        else {
            0
        }
    }

    spec shares(self: &Pool, shareholder: address): u64 {
        aborts_if false;
        ensures result == spec_shares(self, shareholder);
    }

    spec balance(self: &Pool, shareholder: address): u64 {
        let shares = spec_shares(self, shareholder);
        let total_coins = self.total_coins;
        aborts_if self.total_coins > 0 && self.total_shares > 0 && (shares * total_coins) / self.total_shares > MAX_U64;
        ensures result == spec_shares_to_amount_with_total_coins(self, shares, total_coins);
    }

    spec buy_in(self: &mut Pool, shareholder: address, coins_amount: u64): u64 {
        let new_shares = spec_amount_to_shares_with_total_coins(self, coins_amount, self.total_coins);
        aborts_if self.total_coins + coins_amount > MAX_U64;
        aborts_if self.total_shares + new_shares > MAX_U64;
        include coins_amount > 0 ==> AddSharesAbortsIf { new_shares };
        include coins_amount > 0 ==> AddSharesEnsures { new_shares };
        ensures self.total_coins == old(self.total_coins) + coins_amount;
        ensures self.total_shares == old(self.total_shares) + new_shares;
        ensures result == new_shares;
    }

    spec add_shares(self: &mut Pool, shareholder: address, new_shares: u64): u64 {
        include AddSharesAbortsIf;
        include AddSharesEnsures;

        let key_exists = simple_map::spec_contains_key(self.shares, shareholder);
        ensures result == if (key_exists) { simple_map::spec_get(self.shares, shareholder) }
        else { new_shares };
    }
    spec schema AddSharesAbortsIf {
        self: Pool;
        shareholder: address;
        new_shares: u64;

        let key_exists = simple_map::spec_contains_key(self.shares, shareholder);
        let current_shares = simple_map::spec_get(self.shares, shareholder);

        aborts_if key_exists && current_shares + new_shares > MAX_U64;
        aborts_if !key_exists && new_shares > 0 && len(self.shareholders) >= self.shareholders_limit;
    }
    spec schema AddSharesEnsures {
        self: Pool;
        shareholder: address;
        new_shares: u64;

        let key_exists = simple_map::spec_contains_key(self.shares, shareholder);
        let current_shares = simple_map::spec_get(self.shares, shareholder);

        ensures key_exists ==>
            self.shares == simple_map::spec_set(old(self.shares), shareholder, current_shares + new_shares);
        ensures (!key_exists && new_shares > 0) ==>
            self.shares == simple_map::spec_set(old(self.shares), shareholder, new_shares);
        ensures (!key_exists && new_shares > 0) ==>
            vector::eq_push_back(self.shareholders, old(self.shareholders), shareholder);
    }

    spec fun spec_amount_to_shares_with_total_coins(pool: Pool, coins_amount: u64, total_coins: u64): u64 {
        if (pool.total_coins == 0 || pool.total_shares == 0) {
            coins_amount * pool.scaling_factor
        }
        else {
            (coins_amount * pool.total_shares) / total_coins
        }
    }

    spec amount_to_shares_with_total_coins(self: &Pool, coins_amount: u64, total_coins: u64): u64 {
        aborts_if self.total_coins > 0 && self.total_shares > 0
            && (coins_amount * self.total_shares) / total_coins > MAX_U64;
        aborts_if (self.total_coins == 0 || self.total_shares == 0)
            && coins_amount * self.scaling_factor > MAX_U64;
        aborts_if self.total_coins > 0 && self.total_shares > 0 && total_coins == 0;
        ensures result == spec_amount_to_shares_with_total_coins(self, coins_amount, total_coins);
    }

    spec shares_to_amount_with_total_coins(self: &Pool, shares: u64, total_coins: u64): u64 {
        aborts_if self.total_coins > 0 && self.total_shares > 0
            && (shares * total_coins) / self.total_shares > MAX_U64;
        ensures result == spec_shares_to_amount_with_total_coins(self, shares, total_coins);
    }

    spec fun spec_shares_to_amount_with_total_coins(pool: Pool, shares: u64, total_coins: u64): u64 {
        if (pool.total_coins == 0 || pool.total_shares == 0) {
            0
        }
        else {
            (shares * total_coins) / pool.total_shares
        }
    }

    spec multiply_then_divide(self: &Pool, x: u64, y: u64, z: u64): u64 {
        aborts_if z == 0;
        aborts_if (x * y) / z > MAX_U64;
        ensures result == (x * y) / z;
    }

    spec redeem_shares(self: &mut Pool, shareholder: address, shares_to_redeem: u64): u64 {
        let redeemed_coins = spec_shares_to_amount_with_total_coins(self, shares_to_redeem, self.total_coins);
        aborts_if !spec_contains(self, shareholder);
        aborts_if spec_shares(self, shareholder) < shares_to_redeem;
        aborts_if self.total_coins < redeemed_coins;
        aborts_if self.total_shares < shares_to_redeem;
        ensures self.total_coins == old(self.total_coins) - redeemed_coins;
        ensures self.total_shares == old(self.total_shares) - shares_to_redeem;
        include shares_to_redeem > 0 ==> DeductSharesEnsures {
            num_shares: shares_to_redeem
        };
        ensures result == redeemed_coins;
    }

    spec transfer_shares(
        self: &mut Pool,
        shareholder_1: address,
        shareholder_2: address,
        shares_to_transfer: u64
    ) {
        pragma aborts_if_is_partial;
        aborts_if !spec_contains(self, shareholder_1);
        aborts_if spec_shares(self, shareholder_1) < shares_to_transfer;
        // TODO: difficult to specify due to the intermediate state problem.
    }

    spec deduct_shares(self: &mut Pool, shareholder: address, num_shares: u64): u64 {
        aborts_if !spec_contains(self, shareholder);
        aborts_if spec_shares(self, shareholder) < num_shares;

        include DeductSharesEnsures;
        let remaining_shares = simple_map::spec_get(self.shares, shareholder) - num_shares;
        ensures remaining_shares > 0 ==> result == simple_map::spec_get(self.shares, shareholder);
        ensures remaining_shares == 0 ==> result == 0;
    }
    spec schema DeductSharesEnsures {
        self: Pool;
        shareholder: address;
        num_shares: u64;
        let remaining_shares = simple_map::spec_get(self.shares, shareholder) - num_shares;
        ensures remaining_shares > 0 ==> simple_map::spec_get(self.shares, shareholder) == remaining_shares;
        ensures remaining_shares == 0 ==> !simple_map::spec_contains_key(self.shares, shareholder);
        ensures remaining_shares == 0 ==> !vector::spec_contains(self.shareholders, shareholder);
    }
}
