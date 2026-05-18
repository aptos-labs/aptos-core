// Test for bug fix: CSE hoisting a quantifier-bound variable out of forall scope.
//
// When inference injects a struct invariant into the WP, it produces a quantified
// condition of the form `forall i in range: is_valid(bag.items[i])`. Because
// is_valid has `pragma opaque`, ExpSimplifier cannot inline its spec fun (the
// spec fun has `is_move_fun = true` which makes try_unfold_spec_fun return None).
// So the call stays as a SpecFunction node with free variable i.
//
// The CSE pass previously identified is_valid(bag.items[i]) as a CSE candidate
// (SpecFunction call) and hoisted it as `let is_valid_0 = is_valid(bag.items[i])`
// before the forall, making the quantifier-bound variable i undeclared → compile error.
//
// Fix: is_cse_candidate now rejects expressions whose free_vars() is non-empty.
// is_valid(bag.items[i]) contains the free variable i, so it is no longer a CSE
// candidate and stays inside the forall where i is in scope.
module 0x42::cse_forall {
    fun is_valid(x: u64): bool { x <= 100 }
    spec is_valid {
        pragma opaque;
        ensures result == (x <= 100);
        aborts_if false;
        ensures [inferred] result == (x <= 100);
        aborts_if [inferred] false;
    }

    struct Bag has copy, drop {
        items: vector<u64>,
    }
    spec Bag {
        // The invariant produces forall i in 0..len(self.items): is_valid(self.items[i])
        // in the WP for any function touching a Bag. The is_valid call with bound
        // variable i is the CSE trigger point.
        invariant forall i in 0..len(self.items): is_valid(self.items[i]);
    }

    // check_bag has no user ensures — inference runs and injects the Bag invariant
    // into the WP, producing a forall with is_valid(bag.items[i]) inside.
    // Before the fix, CSE hoisted is_valid(bag.items[i]) outside the forall
    // making i undeclared. After the fix, it stays inside.
    //
    // The body just returns true without accessing the vector, so the inferred
    // spec avoids result_of<borrow> behavior predicates — the struct invariant
    // condition alone demonstrates the CSE fix.
    fun check_bag(_bag: &Bag): bool {
        true
    }
    spec check_bag(_bag: &Bag): bool {
        pragma opaque = true;
        ensures [inferred] (forall x in 0..len(_bag.items): is_valid(_bag.items[x])) ==> result == true;
        aborts_if [inferred] false;
    }

}
/*
Verification: Succeeded.
*/
