// When more than 10 borrow edges connect two references, they collapse to one weak edge with
// an empty path. From the frame root, this edge conservatively borrows every local and global,
// even if the original edges borrowed only locals.
module 0x42::m {
    struct R has key, copy, drop {
        x: u64,
    }

    fun mutate_global(ref_r: &R) acquires R {
        let global_ref = borrow_global_mut<R>(@0x42);
        global_ref.x = ref_r.x + 1;
    }

    fun touch_global() acquires R {
        borrow_global_mut<R>(@0x42).x = 0;
    }

    fun consume(_r: &R) {}

    fun consume_mut(_r: &mut R) {}

    // The analysis treats each result as potentially borrowing from any argument.
    fun pick(
        r0: &R, _r1: &R, _r2: &R, _r3: &R, _r4: &R, _r5: &R, _r6: &R, _r7: &R, _r8: &R, _r9: &R,
        _r10: &R
    ): &R {
        r0
    }

    fun pick_mut(
        r0: &mut R, _r1: &mut R, _r2: &mut R, _r3: &mut R, _r4: &mut R, _r5: &mut R, _r6: &mut R,
        _r7: &mut R, _r8: &mut R, _r9: &mut R, _r10: &mut R
    ): &mut R {
        r0
    }

    // 9 local borrows and 1 global borrow: no overflow.
    fun acquires_without_overflow(cond: u8) acquires R {
        let local0 = R { x: 0 };
        let local1 = R { x: 0 };
        let local2 = R { x: 0 };
        let local3 = R { x: 0 };
        let local4 = R { x: 0 };
        let local5 = R { x: 0 };
        let local6 = R { x: 0 };
        let local7 = R { x: 0 };
        let local8 = R { x: 0 };
        let borrowed: &R;
        if (cond == 0) {
            borrowed = &local0;
        } else if (cond == 1) {
            borrowed = &local1;
        } else if (cond == 2) {
            borrowed = &local2;
        } else if (cond == 3) {
            borrowed = &local3;
        } else if (cond == 4) {
            borrowed = &local4;
        } else if (cond == 5) {
            borrowed = &local5;
        } else if (cond == 6) {
            borrowed = &local6;
        } else if (cond == 7) {
            borrowed = &local7;
        } else if (cond == 8) {
            borrowed = &local8;
        } else {
            borrowed = borrow_global<R>(@0x42);
        };
        mutate_global(borrowed);
    }

    // 10 local borrows and 1 global borrow: overflow.
    fun acquires_with_overflow(cond: u8) acquires R {
        let local0 = R { x: 0 };
        let local1 = R { x: 0 };
        let local2 = R { x: 0 };
        let local3 = R { x: 0 };
        let local4 = R { x: 0 };
        let local5 = R { x: 0 };
        let local6 = R { x: 0 };
        let local7 = R { x: 0 };
        let local8 = R { x: 0 };
        let local9 = R { x: 0 };
        let borrowed: &R;
        if (cond == 0) {
            borrowed = &local0;
        } else if (cond == 1) {
            borrowed = &local1;
        } else if (cond == 2) {
            borrowed = &local2;
        } else if (cond == 3) {
            borrowed = &local3;
        } else if (cond == 4) {
            borrowed = &local4;
        } else if (cond == 5) {
            borrowed = &local5;
        } else if (cond == 6) {
            borrowed = &local6;
        } else if (cond == 7) {
            borrowed = &local7;
        } else if (cond == 8) {
            borrowed = &local8;
        } else if (cond == 9) {
            borrowed = &local9;
        } else {
            borrowed = borrow_global<R>(@0x42);
        };
        mutate_global(borrowed);
    }

    // With 11 local borrows, overflow conservatively marks global `R` as borrowed.
    fun acquires_with_local_overflow(
        l0: R, l1: R, l2: R, l3: R, l4: R, l5: R, l6: R, l7: R, l8: R, l9: R, l10: R
    ) acquires R {
        mutate_global(pick(&l0, &l1, &l2, &l3, &l4, &l5, &l6, &l7, &l8, &l9, &l10));
    }

    // With 11 mutable local borrows, overflow marks global `R` as mutably borrowed.
    fun acquires_with_mut_overflow(
        l0: R, l1: R, l2: R, l3: R, l4: R, l5: R, l6: R, l7: R, l8: R, l9: R, l10: R
    ) acquires R {
        let borrowed = pick_mut(
            &mut l0, &mut l1, &mut l2, &mut l3, &mut l4, &mut l5, &mut l6, &mut l7, &mut l8, &mut l9,
            &mut l10
        );
        touch_global();
        consume_mut(borrowed);
    }

    fun return_with_overflow(
        _p: &R, l0: R, l1: R, l2: R, l3: R, l4: R, l5: R, l6: R, l7: R, l8: R, l9: R, l10: R
    ): &R {
        pick(&l0, &l1, &l2, &l3, &l4, &l5, &l6, &l7, &l8, &l9, &l10)
    }

    fun assign_with_overflow(
        other: u64, l0: R, l1: R, l2: R, l3: R, l4: R, l5: R, l6: R, l7: R, l8: R, l9: R, l10: R
    ): u64 {
        let borrowed = pick(&l0, &l1, &l2, &l3, &l4, &l5, &l6, &l7, &l8, &l9, &l10);
        let copied = other;
        borrowed.x + copied
    }

    // Existing borrows of `other`, including `field`, become borrows of the new mutable reference.
    // The `&mut other` diagnostic therefore reports only the full frame borrow caused by overflow.
    fun mut_borrow_with_overflow(
        other: R, l0: R, l1: R, l2: R, l3: R, l4: R, l5: R, l6: R, l7: R, l8: R, l9: R, l10: R
    ) {
        let field = &other.x;
        let borrowed = pick(&l0, &l1, &l2, &l3, &l4, &l5, &l6, &l7, &l8, &l9, &l10);
        consume_mut(&mut other);
        consume(borrowed);
        *field;
    }
}
