// This file specifies the module `pool_u64`. It specifies the invariants of the struct Pool, and the pre/post-conditions
// of the functions.
spec aptos_std::pool_u64 {

    spec module {}
    // -----------------
    // Struct invariants
    // -----------------

    // The invariants of the struct Pool.
    spec Pool {
        // Every element of `shareholders` is a key in `shares`.
        // This is ∀∀ (no existential), Z3-friendly.
        invariant forall i in 0..len(shareholders):
            simple_map::spec_contains_key(shares, shareholders[i]);

        // The converse is redundant with cardinality plus uniqueness, but
        // making it explicit gives `index_of` a concrete witness when a map
        // entry is removed. Solvers do not reliably derive this finite-set
        // pigeonhole argument from the other invariants.
        invariant forall shareholder: address:
            simple_map::spec_contains_key(shares, shareholder) ==>
                vector::spec_contains(shareholders, shareholder);

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
        } else { 0 }
    }

    /// Exact local-state effect of `add_shares` on a valid pool.
    spec fun spec_add_shares_state(
        pool: Pool, shareholder: address, new_shares: u64
    ): Pool {
        if (simple_map::spec_contains_key(pool.shares, shareholder)) {
            update_field(
                pool,
                shares,
                simple_map::spec_set(
                    pool.shares,
                    shareholder,
                    simple_map::spec_get(pool.shares, shareholder) + new_shares
                )
            )
        } else if (new_shares > 0) {
            update_field(
                update_field(
                    pool,
                    shares,
                    simple_map::spec_set(pool.shares, shareholder, new_shares)
                ),
                shareholders,
                concat(pool.shareholders, vector[shareholder])
            )
        } else { pool }
    }

    /// Exact local-state effect of a successful `buy_in`.
    spec fun spec_buy_in_state(
        pool: Pool, shareholder: address, coins_amount: u64
    ): Pool {
        if (coins_amount == 0) { pool }
        else {
            let new_shares =
                spec_amount_to_shares_with_total_coins(
                    pool, coins_amount, pool.total_coins
                );
            spec_add_shares_state(
                update_field(
                    update_field(pool, total_coins, pool.total_coins + coins_amount),
                    total_shares,
                    pool.total_shares + new_shares
                ),
                shareholder,
                new_shares
            )
        }
    }

    /// Exact abort domain of `buy_in`, including the conversion performed
    /// before the pool's scalar overflow checks.
    spec fun spec_buy_in_aborts(
        pool: Pool, shareholder: address, coins_amount: u64
    ): bool {
        if (coins_amount == 0) { false }
        else {
            let new_shares =
                spec_amount_to_shares_with_total_coins(
                    pool, coins_amount, pool.total_coins
                );
            let conversion_aborts =
                if (pool.total_coins == 0 || pool.total_shares == 0) {
                    coins_amount * pool.scaling_factor > MAX_U64
                } else {
                    (coins_amount * pool.total_shares) / pool.total_coins > MAX_U64
                };
            conversion_aborts
                || pool.total_coins + coins_amount > MAX_U64
                || pool.total_shares + new_shares > MAX_U64
                || (
                    simple_map::spec_contains_key(pool.shares, shareholder)
                        && MAX_U64 - simple_map::spec_get(pool.shares, shareholder)
                            < new_shares
                )
                || (
                    !simple_map::spec_contains_key(pool.shares, shareholder)
                        && new_shares > 0
                        && len(pool.shareholders) >= pool.shareholders_limit
                )
        }
    }

    /// The map component of a transfer.  Keeping this as a pure function lets
    /// callers state a loop fold without exposing the implementation's
    /// remove-then-add intermediate state.
    spec fun spec_transfer_shares_map(
        pool: Pool,
        shareholder_1: address,
        shareholder_2: address,
        shares_to_transfer: u64
    ): simple_map::SimpleMap<address, u64> {
        if (shares_to_transfer == 0) {
            pool.shares
        } else {
            let after_deduct =
                if (spec_shares(pool, shareholder_1) > shares_to_transfer) {
                    simple_map::spec_set(
                        pool.shares,
                        shareholder_1,
                        spec_shares(pool, shareholder_1) - shares_to_transfer
                    )
                } else {
                    simple_map::spec_remove(pool.shares, shareholder_1)
                };
            if (simple_map::spec_contains_key(after_deduct, shareholder_2)) {
                simple_map::spec_set(
                    after_deduct,
                    shareholder_2,
                    simple_map::spec_get(after_deduct, shareholder_2)
                        + shares_to_transfer
                )
            } else {
                simple_map::spec_set(after_deduct, shareholder_2, shares_to_transfer)
            }
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
        aborts_if self.total_coins > 0
            && self.total_shares > 0
            && (shares * total_coins) / self.total_shares > MAX_U64;
        ensures result
            == spec_shares_to_amount_with_total_coins(self, shares, total_coins);
    }

    spec buy_in(self: &mut Pool, shareholder: address, coins_amount: u64): u64 {
        pragma opaque = true;
        aborts_if spec_buy_in_aborts(self, shareholder, coins_amount);
        ensures self == spec_buy_in_state(old(self), shareholder, coins_amount);
        ensures result
            == spec_amount_to_shares_with_total_coins(
                old(self), coins_amount, old(self).total_coins
            );
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
        ensures simple_map::spec_contains_key(old(self).shares, shareholder) ==>
            result == simple_map::spec_get(old(self).shares, shareholder) + new_shares;
        ensures !simple_map::spec_contains_key(old(self).shares, shareholder) ==>
            result == new_shares;
    }

    spec schema AddSharesAbortsIf {
        self: Pool;
        shareholder: address;
        new_shares: u64;

        // In aborts_if context, self is pre-state (no old() needed)
        let key_exists = simple_map::spec_contains_key(self.shares, shareholder);
        let current_shares = simple_map::spec_get(self.shares, shareholder);

        aborts_if key_exists && MAX_U64 - current_shares < new_shares;
        aborts_if !key_exists
            && new_shares > 0
            && len(self.shareholders) >= self.shareholders_limit;
    }

    spec schema AddSharesEnsures {
        self: Pool;
        shareholder: address;
        new_shares: u64;

        // All old(self) references inlined directly — no let bindings with old().
        ensures simple_map::spec_contains_key(old(self).shares, shareholder) ==>
            self.shares
                == simple_map::spec_set(
                    old(self).shares,
                    shareholder,
                    simple_map::spec_get(old(self).shares, shareholder) + new_shares
                );
        ensures (
            !simple_map::spec_contains_key(old(self).shares, shareholder)
                && new_shares > 0
        ) ==>
            self.shares
                == simple_map::spec_set(old(self).shares, shareholder, new_shares);
        // No change to shares/shareholders when new_shares == 0 and key doesn't exist.
        ensures (
            !simple_map::spec_contains_key(old(self).shares, shareholder)
                && new_shares == 0
        ) ==>
            self.shares == old(self).shares;
        ensures (
            !simple_map::spec_contains_key(old(self).shares, shareholder)
                && new_shares == 0
        ) ==>
            self.shareholders == old(self).shareholders;
        // Element-level ensures for the vector push (replaces eq_push_back).
        // eq_push_back uses array slices which Z3 cannot unfold into element-level facts,
        // preventing it from re-establishing the ∀i invariant on shareholders.
        ensures (
            !simple_map::spec_contains_key(old(self).shares, shareholder)
                && new_shares > 0
        ) ==>
            len(self.shareholders) == len(old(self).shareholders) + 1;
        ensures (
            !simple_map::spec_contains_key(old(self).shares, shareholder)
                && new_shares > 0
        ) ==>
            self.shareholders[len(old(self).shareholders)] == shareholder;
        ensures (
            !simple_map::spec_contains_key(old(self).shares, shareholder)
                && new_shares > 0
        ) ==>
            (
                forall i in 0..len(old(self).shareholders):
                    self.shareholders[i] == old(self).shareholders[i]
            );
        ensures (
            !simple_map::spec_contains_key(old(self).shares, shareholder)
                && new_shares > 0
        ) ==>
            vector::eq_push_back(self.shareholders, old(self).shareholders, shareholder);
        // When key exists, shareholders is unchanged.
        ensures simple_map::spec_contains_key(old(self).shares, shareholder) ==>
            self.shareholders == old(self).shareholders;
    }

    spec fun spec_amount_to_shares_with_total_coins(
        pool: Pool, coins_amount: u64, total_coins: u64
    ): u64 {
        if (pool.total_coins == 0 || pool.total_shares == 0) {
            coins_amount * pool.scaling_factor
        } else {
            (coins_amount * pool.total_shares) / total_coins
        }
    }

    spec amount_to_shares_with_total_coins(
        self: &Pool, coins_amount: u64, total_coins: u64
    ): u64 {
        pragma opaque = true;
        aborts_if self.total_coins > 0
            && self.total_shares > 0
            && (coins_amount * self.total_shares) / total_coins > MAX_U64;
        aborts_if (self.total_coins == 0 || self.total_shares == 0)
            && coins_amount * self.scaling_factor > MAX_U64;
        aborts_if self.total_coins > 0
            && self.total_shares > 0
            && total_coins == 0;
        ensures result
            == spec_amount_to_shares_with_total_coins(self, coins_amount, total_coins);
    }

    spec shares_to_amount_with_total_coins(
        self: &Pool, shares: u64, total_coins: u64
    ): u64 {
        pragma opaque = true;
        aborts_if self.total_coins > 0
            && self.total_shares > 0
            && (shares * total_coins) / self.total_shares > MAX_U64;
        ensures result
            == spec_shares_to_amount_with_total_coins(self, shares, total_coins);
    }

    spec fun spec_shares_to_amount_with_total_coins(
        pool: Pool, shares: u64, total_coins: u64
    ): u64 {
        if (pool.total_coins == 0 || pool.total_shares == 0) { 0 }
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

    spec redeem_shares(
        self: &mut Pool, shareholder: address, shares_to_redeem: u64
    ): u64 {
        pragma opaque = true;
        // redeemed_coins is computed from the pre-state (aborts_if clauses use pre-state).
        let redeemed_coins = spec_shares_to_amount_with_total_coins(
            self, shares_to_redeem, self.total_coins
        );
        aborts_if !spec_contains(self, shareholder);
        aborts_if spec_shares(self, shareholder) < shares_to_redeem;
        aborts_if self.total_coins < redeemed_coins;
        aborts_if self.total_shares < shares_to_redeem;
        ensures self.total_coins
            == old(self).total_coins
                - spec_shares_to_amount_with_total_coins(
                    old(self), shares_to_redeem, old(self).total_coins
                );
        ensures self.total_shares == old(self).total_shares - shares_to_redeem;
        include shares_to_redeem > 0 ==>
            DeductSharesEnsures { num_shares: shares_to_redeem };
        ensures result
            == spec_shares_to_amount_with_total_coins(
                old(self), shares_to_redeem, old(self).total_coins
            );
    }

    spec transfer_shares(
        self: &mut Pool,
        shareholder_1: address,
        shareholder_2: address,
        shares_to_transfer: u64
    ) {
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        aborts_if !spec_contains(self, shareholder_1);
        aborts_if spec_shares(self, shareholder_1) < shares_to_transfer;
        // `deduct_shares` runs before `add_shares`.  The latter can only fail
        // when the receiver remains present and its shares overflow, or when
        // the sender remains present while a new receiver would exceed the
        // shareholder limit.  A fully deducted sender first frees its slot.
        aborts_if shares_to_transfer > 0
            && shareholder_1 != shareholder_2
            && spec_contains(self, shareholder_2)
            && spec_shares(self, shareholder_2) + shares_to_transfer > MAX_U64;
        aborts_if shares_to_transfer > 0
            && shareholder_1 != shareholder_2
            && !spec_contains(self, shareholder_2)
            && spec_shares(self, shareholder_1) > shares_to_transfer
            && simple_map::spec_len(self.shares) >= self.shareholders_limit;

        ensures self.total_coins == old(self).total_coins;
        ensures self.total_shares == old(self).total_shares;
        ensures self.shareholders_limit == old(self).shareholders_limit;
        ensures self.scaling_factor == old(self).scaling_factor;
        ensures shares_to_transfer == 0 ==>
            self.shares == old(self).shares;
        // A full transfer to the same shareholder removes and then re-adds
        // that entry, so its list position can change even though its map
        // value is restored.
        ensures shares_to_transfer > 0 && shareholder_1 == shareholder_2 ==>
            self.shares == old(self).shares;
        ensures self.shares
            == spec_transfer_shares_map(
                old(self),
                shareholder_1,
                shareholder_2,
                shares_to_transfer
            );
        ensures shares_to_transfer > 0 && shareholder_1 != shareholder_2 ==>
            spec_shares(self, shareholder_1)
                == spec_shares(old(self), shareholder_1) - shares_to_transfer;
        ensures shares_to_transfer > 0 && shareholder_1 != shareholder_2 ==>
            spec_shares(self, shareholder_2)
                == spec_shares(old(self), shareholder_2) + shares_to_transfer;
        ensures shares_to_transfer > 0
            && shareholder_1 != shareholder_2 ==>
            (
                forall shareholder: address:
                    shareholder != shareholder_1
                        && shareholder != shareholder_2 ==>
                        spec_shares(self, shareholder)
                            == spec_shares(old(self), shareholder)
            );
        // Exact shareholder ordering. A partial deduction leaves the sender
        // in place; a new receiver is appended. A full deduction removes the
        // sender at its unique old index before the receiver is either reused
        // or appended.
        ensures shares_to_transfer == 0 ==>
            self.shareholders == old(self).shareholders;
        ensures shares_to_transfer > 0
            && spec_shares(old(self), shareholder_1) > shares_to_transfer
            && spec_contains(old(self), shareholder_2) ==>
            self.shareholders == old(self).shareholders;
        ensures shares_to_transfer > 0
            && spec_shares(old(self), shareholder_1) > shares_to_transfer
            && !spec_contains(old(self), shareholder_2) ==>
            vector::eq_push_back(
                self.shareholders, old(self).shareholders, shareholder_2
            );
        ensures shares_to_transfer > 0
            && spec_shares(old(self), shareholder_1) == shares_to_transfer ==>
            (
                exists i: u64,
                after_deduct: vector<address> :
                    i < len(old(self).shareholders)
                        && old(self).shareholders[i] == shareholder_1
                        && vector::eq_remove_elem_at_index(
                            i, after_deduct, old(self).shareholders
                        )
                        && (
                            if (shareholder_1 != shareholder_2
                                && spec_contains(old(self), shareholder_2)) {
                                self.shareholders == after_deduct
                            } else {
                                vector::eq_push_back(
                                    self.shareholders, after_deduct, shareholder_2
                                )
                            }
                        )
            );
    }

    spec deduct_shares(
        self: &mut Pool, shareholder: address, num_shares: u64
    ): u64 {
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
        let remaining_shares = simple_map::spec_get(self.shares, shareholder)
            - num_shares;
        ensures remaining_shares > 0 ==>
            result == remaining_shares;
        ensures remaining_shares == 0 ==> result == 0;
    }

    spec schema DeductSharesEnsures {
        self: Pool;
        shareholder: address;
        num_shares: u64;
        // NOTE: no `let` bindings using old() here — inline instead.
        // pre_remaining = spec_get(old_shares, shareholder) - num_shares
        // We use old(self) only in ensures clauses (valid there).
        ensures simple_map::spec_get(old(self).shares, shareholder) - num_shares > 0 ==>
            self.shares
                == simple_map::spec_set(
                    old(self).shares,
                    shareholder,
                    simple_map::spec_get(old(self).shares, shareholder) - num_shares
                );
        ensures simple_map::spec_get(old(self).shares, shareholder) - num_shares == 0 ==>
            self.shares == simple_map::spec_remove(old(self).shares, shareholder);
        ensures simple_map::spec_get(old(self).shares, shareholder) - num_shares == 0 ==>
            !vector::spec_contains(self.shareholders, shareholder);
        // Explicit length postcondition: anchors the spec_len == len invariant after removal.
        ensures simple_map::spec_get(old(self).shares, shareholder) - num_shares == 0 ==>
            len(self.shareholders) == len(old(self).shareholders) - 1;
        ensures simple_map::spec_get(old(self).shares, shareholder) - num_shares == 0 ==>
            simple_map::spec_len(self.shares)
                == simple_map::spec_len(old(self).shares) - 1;
        ensures simple_map::spec_get(old(self).shares, shareholder) - num_shares > 0 ==>
            self.shareholders == old(self).shareholders;
        ensures simple_map::spec_get(old(self).shares, shareholder) - num_shares == 0 ==>
            (
                exists i: u64:
                    i < len(old(self).shareholders)
                        && old(self).shareholders[i] == shareholder
                        && vector::eq_remove_elem_at_index(
                            i, self.shareholders, old(self).shareholders
                        )
            );
    }

    spec new(shareholders_limit: u64): Pool {
        use 0x1::simple_map;
        pragma opaque = true;
        ensures result
            == Pool {
                shareholders_limit: shareholders_limit,
                total_coins: 0,
                total_shares: 0,
                shares: simple_map::spec_new<address, u64>(),
                shareholders: vector[],
                scaling_factor: 1
            };
        aborts_if false;
        pragma aborts_if_is_partial = true;
        ensures [inferred] create_with_scaling_factor(shareholders_limit, 1)
            == Pool {
                shareholders_limit: shareholders_limit,
                total_coins: 0,
                total_shares: 0,
                shares: simple_map::spec_new<address, u64>(),
                shareholders: vector<address>[],
                scaling_factor: 1
            } ==>
            result == create_with_scaling_factor(shareholders_limit, 1);
    }

    spec create(shareholders_limit: u64): Pool {
        use 0x1::simple_map;
        pragma opaque = true;
        ensures result
            == Pool {
                shareholders_limit: shareholders_limit,
                total_coins: 0,
                total_shares: 0,
                shares: simple_map::spec_new<address, u64>(),
                shareholders: vector[],
                scaling_factor: 1
            };
        aborts_if false;
        pragma aborts_if_is_partial = true;
        ensures [inferred] new(shareholders_limit)
            == Pool {
                shareholders_limit: shareholders_limit,
                total_coins: 0,
                total_shares: 0,
                shares: simple_map::spec_new<address, u64>(),
                shareholders: vector<address>[],
                scaling_factor: 1
            } ==>
            result == new(shareholders_limit);
    }

    spec amount_to_shares(self: &Pool, coins_amount: u64): u64 {
        pragma opaque = true;
        aborts_if self.total_coins > 0
            && self.total_shares > 0
            && (coins_amount * self.total_shares) / self.total_coins > MAX_U64;
        aborts_if (self.total_coins == 0 || self.total_shares == 0)
            && coins_amount * self.scaling_factor > MAX_U64;
        // self.total_coins > 0 && self.total_coins == 0 is always false — no abort needed here.
        ensures result
            == spec_amount_to_shares_with_total_coins(
                self, coins_amount, self.total_coins
            );
    }

    spec create_with_scaling_factor(
        shareholders_limit: u64, scaling_factor: u64
    ): Pool {
        use 0x1::simple_map;
        pragma opaque = true;
        ensures result
            == Pool {
                shareholders_limit: shareholders_limit,
                total_coins: 0,
                total_shares: 0,
                shares: simple_map::spec_new<address, u64>(),
                shareholders: vector[],
                scaling_factor: scaling_factor
            };
        aborts_if false;
        pragma aborts_if_is_partial = true;
        ensures [inferred] Pool {
            shareholders_limit: shareholders_limit,
            total_coins: 0,
            total_shares: 0,
            shares: simple_map::spec_new<address, u64>(),
            shareholders: vec<address>(),
            scaling_factor: scaling_factor
        } == Pool {
            shareholders_limit: shareholders_limit,
            total_coins: 0,
            total_shares: 0,
            shares: simple_map::spec_new<address, u64>(),
            shareholders: vector<address>[],
            scaling_factor: scaling_factor
        } ==>
            result
                == Pool {
                    shareholders_limit: shareholders_limit,
                    total_coins: 0,
                    total_shares: 0,
                    shares: simple_map::spec_new<address, u64>(),
                    shareholders: vec<address>(),
                    scaling_factor: scaling_factor
                };
    }

    spec shares_to_amount(self: &Pool, shares: u64): u64 {
        pragma opaque = true;
        aborts_if self.total_coins > 0
            && self.total_shares > 0
            && (shares * self.total_coins) / self.total_shares > MAX_U64;
        ensures result
            == spec_shares_to_amount_with_total_coins(self, shares, self.total_coins);
    }

    spec update_total_coins(self: &mut Pool, new_total_coins: u64) {
        pragma opaque = true;
        aborts_if false;
        ensures self.total_coins == new_total_coins;
        ensures self.total_shares == old(self).total_shares;
        ensures self.shareholders_limit == old(self).shareholders_limit;
        ensures self.scaling_factor == old(self).scaling_factor;
        ensures self.shares == old(self).shares;
        ensures self.shareholders == old(self).shareholders;
    }

    spec destroy_empty(self: Pool) {
        use 0x1::error;
        use 0x1::simple_map;
        aborts_if self.total_coins != 0;
        pragma opaque = true;
        aborts_if [inferred](
            forall x in 0..len(self.shareholders):
                simple_map::spec_contains_key<address, u64>(
                    self.shares, self.shareholders[x]
                )
        )
            && simple_map::spec_len<address, u64>(self.shares)
                == len(self.shareholders)
            && len(self.shareholders) <= self.shareholders_limit
            && (
                forall x in 0..len(self.shareholders),
                y in 0..len(self.shareholders):
                    self.shareholders[x] == self.shareholders[y] ==> x == y
            )
            && self.total_coins != 0;
        aborts_if [inferred](
            forall x in 0..len(self.shareholders):
                simple_map::spec_contains_key<address, u64>(
                    self.shares, self.shareholders[x]
                )
        )
            && simple_map::spec_len<address, u64>(self.shares)
                == len(self.shareholders)
            && len(self.shareholders) <= self.shareholders_limit
            && (
                forall x in 0..len(self.shareholders),
                y in 0..len(self.shareholders):
                    self.shareholders[x] == self.shareholders[y] ==> x == y
            )
            && (self.total_coins != 0
                && aborts_of<error::invalid_state>(3));
    }

    spec total_coins(self: &0x1::pool_u64::Pool): u64 {
        use 0x1::simple_map;
        pragma opaque = true;
        ensures [inferred](
            forall x in 0..len(self.shareholders):
                simple_map::spec_contains_key<address, u64>(
                    self.shares, self.shareholders[x]
                )
        )
            && simple_map::spec_len<address, u64>(self.shares)
                == len(self.shareholders)
            && len(self.shareholders) <= self.shareholders_limit
            && (
                forall x in 0..len(self.shareholders),
                y in 0..len(self.shareholders):
                    self.shareholders[x] == self.shareholders[y] ==> x == y
            ) ==>
            result == self.total_coins;
        aborts_if [inferred] false;
    }

    spec shareholders_count(self: &0x1::pool_u64::Pool): u64 {
        use 0x1::simple_map;
        pragma opaque = true;
        ensures [inferred](
            forall x in 0..len(self.shareholders):
                simple_map::spec_contains_key<address, u64>(
                    self.shares, self.shareholders[x]
                )
        )
            && simple_map::spec_len<address, u64>(self.shares)
                == len(self.shareholders)
            && len(self.shareholders) <= self.shareholders_limit
            && (
                forall x in 0..len(self.shareholders),
                y in 0..len(self.shareholders):
                    self.shareholders[x] == self.shareholders[y] ==> x == y
            ) ==>
            result == len(self.shareholders);
        aborts_if [inferred] false;
    }

    spec total_shares(self: &0x1::pool_u64::Pool): u64 {
        use 0x1::simple_map;
        pragma opaque = true;
        ensures [inferred](
            forall x in 0..len(self.shareholders):
                simple_map::spec_contains_key<address, u64>(
                    self.shares, self.shareholders[x]
                )
        )
            && simple_map::spec_len<address, u64>(self.shares)
                == len(self.shareholders)
            && len(self.shareholders) <= self.shareholders_limit
            && (
                forall x in 0..len(self.shareholders),
                y in 0..len(self.shareholders):
                    self.shareholders[x] == self.shareholders[y] ==> x == y
            ) ==>
            result == self.total_shares;
        aborts_if [inferred] false;
    }

    spec shareholders(self: &0x1::pool_u64::Pool): vector<address> {
        pragma opaque = true;
        aborts_if false;
        ensures result == self.shareholders;
    }
}
