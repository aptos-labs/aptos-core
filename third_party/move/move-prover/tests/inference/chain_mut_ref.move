// Test that chaining: (1) a behavior-predicate call that reads &Pool,
// followed by (2) a direct field mutation to self,
// followed by (3) a behavior-predicate call that takes &mut Pool,
// produces valid spec syntax.
//
// buy_in in pool_u64 follows exactly this pattern, and produces expressions like:
//   S1.. |~ result_of<update>(updated_self, result_of<compute>(old(self), x))
// where old(self) appears inside a S1.. state-label expression.
// The resulting block expressions { let (_t0,_t1) = S1.. |~ ...; _t1 }.field
// must be valid for verification to succeed.
module 0x42::chain_mut_ref {
    struct Pool has copy, drop {
        total: u64,
        items: vector<u64>,
    }

    spec Pool {
        // Simple struct invariant so WP must express post-Pool invariants
        // as { let (_t0,_t1) = S1.. |~ result_of<update>(...); _t1 }.items[i] >= 0.
        invariant forall i in 0..len(items): items[i] >= 0;
    }

    // A function with a while loop — impure, so it becomes a behavior predicate.
    fun helper(n: u64): u64 {
        let i = 0;
        while (i < n) {
            i = i + 1;
        } spec {
            invariant i <= n;
        };
        i
    }
    spec helper(n: u64): u64 {
        ensures result == n;
        aborts_if false;
    }

    // Reads &Pool; impure because it calls helper (while loop → Assign).
    // try_as_pure_spec_call fourth check rejects it → behavior predicate.
    fun compute(self: &Pool, x: u64): u64 {
        helper(x) + self.total
    }
    spec compute {
        pragma opaque = true;
        pragma inference = none;
        aborts_if x + self.total > MAX_U64;
        ensures result == x + self.total;
    }

    // Takes &mut Pool → first check in try_as_pure_spec_call fails immediately.
    // Always a behavior predicate; result_of<update> has extended type (u64, Pool).
    fun update(self: &mut Pool, new_item: u64): u64 {
        self.items.push_back(new_item);
        new_item
    }
    spec update {
        pragma opaque = true;
        pragma inference = none;
        aborts_if false;
        ensures result == new_item;
        ensures len(self.items) == len(old(self.items)) + 1;
        ensures forall i in 0..len(old(self.items)): self.items[i] == old(self.items[i]);
        ensures self.items[len(old(self.items))] == new_item;
        ensures self.total == old(self.total);
    }

    // Pattern from buy_in:
    // 1. Call compute (behavior predicate, reads &self) → c
    // 2. Mutate self.total += c  (direct field write)
    // 3. Call update (behavior predicate, &mut self) → uses modified self
    //
    // WP creates intermediate state S1 for the state before update.
    // The argument old(self) from compute appears inside
    //   S1.. |~ result_of<update>(modified_self, result_of<compute>(old(self), x))
    // and the struct invariant injects
    //   { let (_t0,_t1) = S1.. |~ result_of<update>(...); _t1 }.items[i] >= 0.
    fun chain(self: &mut Pool, x: u64): u64 {
        let c = compute(self, x);
        self.total = self.total + c;
        update(self, c)
    }
}
