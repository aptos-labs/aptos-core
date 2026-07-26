// Ghost-bearing types in behavioral-predicate (stored function) invariants:
// whole-value equality lifted into the axiom must ignore ghosts, like every
// other equality path. Raw datatype `==` would fix `r`'s ghost inside the
// axiom, letting distinct-input calls to a ghost-carrying stored function
// contradict each other and certify false. The pack-time invariant check
// happens in `bp_exploit` itself; its `ensures result` on a false return
// must FAIL.
module 0x42::ghost_field_bp {
    struct BpS has copy, drop { x: u64 }
    spec BpS { ghost g: u64; }

    spec fun bp_make_one(): BpS { BpS { x: 1 } }

    public fun bp_mk(v: u64): BpS {
        let s = BpS { x: 1 };
        spec { update s.g = v; };
        s
    }
    spec bp_mk {
        aborts_if false;
        ensures result.x == 1;
        ensures result.g == v;
    }

    struct BpHolder has drop {
        f: |u64|BpS has copy+drop,
    }
    spec BpHolder {
        invariant forall v: u64, r: BpS: ensures_of<f>(v, r) ==> r == bp_make_one();
    }

    public fun bp_exploit(): bool {
        let h = BpHolder { f: |v| bp_mk(v) };
        let r1 = (h.f)(1);
        let r2 = (h.f)(2);
        let _ = r1;
        let _ = r2;
        false
    }
    spec bp_exploit { ensures result; }    // FAILS
}
