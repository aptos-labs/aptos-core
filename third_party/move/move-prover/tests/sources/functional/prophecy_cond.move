// also_include_for: prophecy
module 0x42::prophecy_cond {
    struct S has drop { f: u64, g: u64 }

    // A reference selected conditionally, then mutated. Under the static model this
    // needs runtime IsParent write-back choices; under the prophecy model each branch's
    // borrow establishes its own creation-time equation and the solver's path
    // sensitivity disambiguates, with no runtime branch.
    fun cond_mut(c: bool): (u64, u64) {
        let s = S { f: 1, g: 2 };
        let r = if (c) &mut s.f else &mut s.g;
        *r = 9;
        (s.f, s.g)
    }
    spec cond_mut {
        ensures c ==> (result_1 == 9 && result_2 == 2);
        ensures !c ==> (result_1 == 1 && result_2 == 9);
    }
}
