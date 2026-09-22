// This file specifies the module `pool_u64`. It specifies the invariants of the struct Pool, and the pre/post-conditions
// of the functions.
spec aptos_std::pool_u64 {

    spec module {
    }
    // -----------------
    // Struct invariants
    // -----------------

    // The invariants of the struct Pool.
    spec Pool {
        // Every element of `shareholders` is a key in `shares`.
        // This is ∀∀ (no existential), Z3-friendly.
        invariant forall i in 0..len(shareholders):
            simple_map::spec_contains_key(shares, shareholders[i]);

        // `shares` and `shareholders` have the same cardinality.
        // Combined with the above + no-dup, this implies the full bijection.
        invariant simple_map::spec_len(shares) == len(shareholders);

        // `shareholders` is bounded by the limit.
        invariant len(shareholders) <= shareholders_limit;

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
        pragma opaque = true;
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
        pragma opaque = true;
        aborts_if false;
        ensures result == spec_shares(self, shareholder);
    }

    spec balance(self: &Pool, shareholder: address): u64 {
        pragma opaque = true;
        let shares = spec_shares(self, shareholder);
        let total_coins = self.total_coins;
        aborts_if self.total_coins > 0 && self.total_shares > 0 && (shares * total_coins) / self.total_shares > MAX_U64;
        ensures result == spec_shares_to_amount_with_total_coins(self, shares, total_coins);
    }

    spec buy_in(self: &mut Pool, shareholder: address, coins_amount: u64): u64 {
        pragma opaque = true;
        // new_shares is computed from the pre-state.
        // In aborts_if/include, self is already pre-state so we use it directly.
        // In ensures, we use old(self) explicitly.
        let new_shares = spec_amount_to_shares_with_total_coins(self, coins_amount, self.total_coins);
        aborts_if self.total_coins + coins_amount > MAX_U64;
        aborts_if self.total_shares + new_shares > MAX_U64;
        include coins_amount > 0 ==> AddSharesAbortsIf { new_shares };
        include coins_amount > 0 ==> AddSharesEnsures { new_shares };
        ensures self.total_coins == old(self).total_coins + coins_amount;
        ensures self.total_shares == old(self).total_shares
            + spec_amount_to_shares_with_total_coins(old(self), coins_amount, old(self).total_coins);
        ensures result == spec_amount_to_shares_with_total_coins(old(self), coins_amount, old(self).total_coins);
    }

    spec add_shares(self: &mut Pool, shareholder: address, new_shares: u64): u64 {
        pragma opaque = true;
        include AddSharesAbortsIf;
        include AddSharesEnsures;
        // Frame: only shares and shareholders are modified, not the other fields.
        ensures self.total_coins == old(self).total_coins;
        ensures self.total_shares == old(self).total_shares;
        ensures self.shareholders_limit == old(self).shareholders_limit;
        ensures self.scaling_factor == old(self).scaling_factor;
        // result value: existing shareholder gets updated total, new shareholder gets new_shares.
        ensures simple_map::spec_contains_key(old(self).shares, shareholder)
            ==> result == simple_map::spec_get(old(self).shares, shareholder) + new_shares;
        ensures !simple_map::spec_contains_key(old(self).shares, shareholder)
            ==> result == new_shares;
    }
    spec schema AddSharesAbortsIf {
        self: Pool;
        shareholder: address;
        new_shares: u64;

        // In aborts_if context, self is pre-state (no old() needed)
        let key_exists = simple_map::spec_contains_key(self.shares, shareholder);
        let current_shares = simple_map::spec_get(self.shares, shareholder);

        aborts_if key_exists && current_shares + new_shares > MAX_U64;
        aborts_if !key_exists && new_shares > 0 && len(self.shareholders) >= self.shareholders_limit;
    }
    spec schema AddSharesEnsures {
        self: Pool;
        shareholder: address;
        new_shares: u64;

        // All old(self) references inlined directly — no let bindings with old().
        ensures simple_map::spec_contains_key(old(self).shares, shareholder) ==>
            self.shares == simple_map::spec_set(old(self).shares, shareholder,
                simple_map::spec_get(old(self).shares, shareholder) + new_shares);
        ensures (!simple_map::spec_contains_key(old(self).shares, shareholder) && new_shares > 0) ==>
            self.shares == simple_map::spec_set(old(self).shares, shareholder, new_shares);
        // No change to shares/shareholders when new_shares == 0 and key doesn't exist.
        ensures (!simple_map::spec_contains_key(old(self).shares, shareholder) && new_shares == 0) ==>
            self.shares == old(self).shares;
        ensures (!simple_map::spec_contains_key(old(self).shares, shareholder) && new_shares == 0) ==>
            self.shareholders == old(self).shareholders;
        // Element-level ensures for the vector push (replaces eq_push_back).
        // eq_push_back uses array slices which Z3 cannot unfold into element-level facts,
        // preventing it from re-establishing the ∀i invariant on shareholders.
        ensures (!simple_map::spec_contains_key(old(self).shares, shareholder) && new_shares > 0) ==>
            len(self.shareholders) == len(old(self).shareholders) + 1;
        ensures (!simple_map::spec_contains_key(old(self).shares, shareholder) && new_shares > 0) ==>
            self.shareholders[len(old(self).shareholders)] == shareholder;
        ensures (!simple_map::spec_contains_key(old(self).shares, shareholder) && new_shares > 0) ==>
            (forall i in 0..len(old(self).shareholders):
                self.shareholders[i] == old(self).shareholders[i]);
        // When key exists, shareholders is unchanged.
        ensures simple_map::spec_contains_key(old(self).shares, shareholder) ==>
            self.shareholders == old(self).shareholders;
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
        pragma opaque = true;
        aborts_if self.total_coins > 0 && self.total_shares > 0
            && (coins_amount * self.total_shares) / total_coins > MAX_U64;
        aborts_if (self.total_coins == 0 || self.total_shares == 0)
            && coins_amount * self.scaling_factor > MAX_U64;
        aborts_if self.total_coins > 0 && self.total_shares > 0 && total_coins == 0;
        ensures result == spec_amount_to_shares_with_total_coins(self, coins_amount, total_coins);
    }

    spec shares_to_amount_with_total_coins(self: &Pool, shares: u64, total_coins: u64): u64 {
        pragma opaque = true;
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
        pragma opaque = true;
        aborts_if z == 0;
        aborts_if (x * y) / z > MAX_U64;
        ensures result == (x * y) / z;
    }

    spec redeem_shares(self: &mut Pool, shareholder: address, shares_to_redeem: u64): u64 {
        pragma opaque = true;
        // redeemed_coins is computed from the pre-state (aborts_if clauses use pre-state).
        let redeemed_coins = spec_shares_to_amount_with_total_coins(self, shares_to_redeem, self.total_coins);
        aborts_if !spec_contains(self, shareholder);
        aborts_if spec_shares(self, shareholder) < shares_to_redeem;
        aborts_if self.total_coins < redeemed_coins;
        aborts_if self.total_shares < shares_to_redeem;
        ensures self.total_coins == old(self).total_coins
            - spec_shares_to_amount_with_total_coins(old(self), shares_to_redeem, old(self).total_coins);
        ensures self.total_shares == old(self).total_shares - shares_to_redeem;
        include shares_to_redeem > 0 ==> DeductSharesEnsures {
            num_shares: shares_to_redeem
        };
        ensures result == spec_shares_to_amount_with_total_coins(old(self), shares_to_redeem, old(self).total_coins);
    }

    spec transfer_shares(
        self: &mut Pool,
        shareholder_1: address,
        shareholder_2: address,
        shares_to_transfer: u64
    ) {
        pragma aborts_if_is_partial;
        pragma opaque = true;
        aborts_if !spec_contains(self, shareholder_1);
        aborts_if spec_shares(self, shareholder_1) < shares_to_transfer;
        // TODO: difficult to specify due to the intermediate state problem.
    }

    spec deduct_shares(self: &mut Pool, shareholder: address, num_shares: u64): u64 {
        pragma opaque = true;
        // aborts_if: self is pre-state
        aborts_if !spec_contains(self, shareholder);
        aborts_if spec_shares(self, shareholder) < num_shares;

        include DeductSharesEnsures;
        // Frame: only shares and shareholders are modified.
        ensures self.total_coins == old(self).total_coins;
        ensures self.total_shares == old(self).total_shares;
        ensures self.shareholders_limit == old(self).shareholders_limit;
        ensures self.scaling_factor == old(self).scaling_factor;
        // remaining_shares uses pre-state (self is pre-state in aborts_if / let context)
        let remaining_shares = simple_map::spec_get(self.shares, shareholder) - num_shares;
        ensures remaining_shares > 0 ==> result == remaining_shares;
        ensures remaining_shares == 0 ==> result == 0;
    }
    spec schema DeductSharesEnsures {
        self: Pool;
        shareholder: address;
        num_shares: u64;
        // NOTE: no `let` bindings using old() here — inline instead.
        // pre_remaining = spec_get(old_shares, shareholder) - num_shares
        // We use old(self) only in ensures clauses (valid there).
        ensures simple_map::spec_get(old(self).shares, shareholder) - num_shares > 0
            ==> simple_map::spec_get(self.shares, shareholder)
                == simple_map::spec_get(old(self).shares, shareholder) - num_shares;
        ensures simple_map::spec_get(old(self).shares, shareholder) - num_shares == 0
            ==> !simple_map::spec_contains_key(self.shares, shareholder);
        ensures simple_map::spec_get(old(self).shares, shareholder) - num_shares == 0
            ==> !vector::spec_contains(self.shareholders, shareholder);
        // Explicit length postcondition: anchors the spec_len == len invariant after removal.
        ensures simple_map::spec_get(old(self).shares, shareholder) - num_shares == 0 ==>
            len(self.shareholders) == len(old(self).shareholders) - 1;
        ensures simple_map::spec_get(old(self).shares, shareholder) - num_shares == 0 ==>
            simple_map::spec_len(self.shares) == simple_map::spec_len(old(self).shares) - 1;
    }
    spec new(shareholders_limit: u64): Pool {
        pragma opaque = true;
        ensures result == Pool {
            shareholders_limit: shareholders_limit,
            total_coins: 0,
            total_shares: 0,
            shares: simple_map::spec_new<address, u64>(),
            shareholders: vector[],
            scaling_factor: 1
        };
        aborts_if false;
    }

    spec create(shareholders_limit: u64): Pool {
        pragma opaque = true;
        ensures result == Pool {
            shareholders_limit: shareholders_limit,
            total_coins: 0,
            total_shares: 0,
            shares: simple_map::spec_new<address, u64>(),
            shareholders: vector[],
            scaling_factor: 1
        };
        aborts_if false;
    }

    spec amount_to_shares(self: &Pool, coins_amount: u64): u64 {
        pragma opaque = true;
        aborts_if self.total_coins > 0 && self.total_shares > 0
            && (coins_amount * self.total_shares) / self.total_coins > MAX_U64;
        aborts_if (self.total_coins == 0 || self.total_shares == 0)
            && coins_amount * self.scaling_factor > MAX_U64;
        // self.total_coins > 0 && self.total_coins == 0 is always false — no abort needed here.
        ensures result == spec_amount_to_shares_with_total_coins(self, coins_amount, self.total_coins);
    }

    spec create_with_scaling_factor(shareholders_limit: u64, scaling_factor: u64): Pool {
        pragma opaque = true;
        ensures result == Pool {
            shareholders_limit: shareholders_limit,
            total_coins: 0,
            total_shares: 0,
            shares: simple_map::spec_new<address, u64>(),
            shareholders: vector[],
            scaling_factor: scaling_factor
        };
        aborts_if false;
    }

    spec shares_to_amount(self: &Pool, shares: u64): u64 {
        pragma opaque = true;
        aborts_if self.total_coins > 0 && self.total_shares > 0
            && (shares * self.total_coins) / self.total_shares > MAX_U64;
        ensures result == spec_shares_to_amount_with_total_coins(self, shares, self.total_coins);
    }

    spec update_total_coins(self: &mut Pool, new_total_coins: u64) {
        aborts_if false;
        ensures self.total_coins == new_total_coins;
        ensures self.total_shares == old(self).total_shares;
        ensures self.shareholders_limit == old(self).shareholders_limit;
        ensures self.scaling_factor == old(self).scaling_factor;
        ensures self.shares == old(self).shares;
        ensures self.shareholders == old(self).shareholders;
    }

    spec destroy_empty(self: Pool) {
        aborts_if self.total_coins != 0;
    }

}
