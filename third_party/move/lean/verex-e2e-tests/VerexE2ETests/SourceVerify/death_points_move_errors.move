// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Where a loan dies: every holder is dead from the anchor on, on every path
// reaching it. Each function has a false twin and an `ensures false` twin.
module 0x42::death_points_move_errors {
    fun bump(r: &mut u64, v: u64) {
        *r = v;
    }
    spec bump {
        ensures r == v;
        aborts_if false;
    }

    fun whole_repoint(c: bool, a: u64, b: u64, v: u64): (u64, u64) {
        let x = a;
        let y = b;
        let r = &mut x;
        let s = &mut y;
        if (c) {
            s = r;
        };
        *s = v;
        (x, y)
    }
    spec whole_repoint {
        ensures c ==> result_1 == v && result_2 == b;
        ensures !c ==> result_1 == a && result_2 == v;
    }

    fun whole_repoint_false(c: bool, a: u64, b: u64, v: u64): (u64, u64) {
        let x = a;
        let y = b;
        let r = &mut x;
        let s = &mut y;
        if (c) {
            s = r;
        };
        *s = v;
        (x, y)
    }
    spec whole_repoint_false {
        ensures result_2 == v; // error: not on the other path
    }

    fun whole_repoint_vacuous(c: bool, a: u64, b: u64, v: u64): (u64, u64) {
        let x = a;
        let y = b;
        let r = &mut x;
        let s = &mut y;
        if (c) {
            s = r;
        };
        *s = v;
        (x, y)
    }
    spec whole_repoint_vacuous {
        ensures false; // error: the paths are not vacuous
    }

    fun whole_repoint_trailing(c: bool, a: u64, b: u64, v: u64): (u64, u64) {
        let x = a;
        let y = b;
        let r = &mut x;
        let s = &mut y;
        let i = 0;
        if (c) {
            s = r;
            i = i + 1;
        };
        *s = v;
        (x, y)
    }
    spec whole_repoint_trailing {
        ensures c ==> result_1 == v && result_2 == b;
        ensures !c ==> result_1 == a && result_2 == v;
    }

    fun whole_repoint_trailing_false(c: bool, a: u64, b: u64, v: u64): (u64, u64) {
        let x = a;
        let y = b;
        let r = &mut x;
        let s = &mut y;
        let i = 0;
        if (c) {
            s = r;
            i = i + 1;
        };
        *s = v;
        (x, y)
    }
    spec whole_repoint_trailing_false {
        ensures result_2 == v; // error: not on the other path
    }

    fun whole_repoint_trailing_vacuous(c: bool, a: u64, b: u64, v: u64): (u64, u64) {
        let x = a;
        let y = b;
        let r = &mut x;
        let s = &mut y;
        let i = 0;
        if (c) {
            s = r;
            i = i + 1;
        };
        *s = v;
        (x, y)
    }
    spec whole_repoint_trailing_vacuous {
        ensures false; // error: the paths are not vacuous
    }

    fun reborrow_chain(a: u64, v: u64): u64 {
        let x = a;
        let s = &mut x;
        let t = &mut *s;
        *t = v;
        x
    }
    spec reborrow_chain {
        ensures result == v;
    }

    fun reborrow_chain_false(a: u64, v: u64): u64 {
        let x = a;
        let s = &mut x;
        let t = &mut *s;
        *t = v;
        x
    }
    spec reborrow_chain_false {
        ensures result == a; // error: the write reaches it
    }

    fun reborrow_chain_vacuous(a: u64, v: u64): u64 {
        let x = a;
        let s = &mut x;
        let t = &mut *s;
        *t = v;
        x
    }
    spec reborrow_chain_vacuous {
        ensures false; // error: the paths are not vacuous
    }

    fun cond_consume(c: bool, a: u64, v: u64): u64 {
        let x = a;
        let r = &mut x;
        if (c) {
            bump(r, v);
        } else {
            *r = 1;
        };
        x
    }
    spec cond_consume {
        ensures c ==> result == v;
        ensures !c ==> result == 1;
    }

    fun cond_consume_false(c: bool, a: u64, v: u64): u64 {
        let x = a;
        let r = &mut x;
        if (c) {
            bump(r, v);
        } else {
            *r = 1;
        };
        x
    }
    spec cond_consume_false {
        ensures result == v; // error: not on the other path
    }

    fun cond_consume_vacuous(c: bool, a: u64, v: u64): u64 {
        let x = a;
        let r = &mut x;
        if (c) {
            bump(r, v);
        } else {
            *r = 1;
        };
        x
    }
    spec cond_consume_vacuous {
        ensures false; // error: the paths are not vacuous
    }

    fun then_reads_else_repoints(c: bool, a: u64, b: u64, v: u64): (u64, u64) {
        let x = a;
        let y = b;
        let s = &mut x;
        let w = &mut y;
        bump(w, b);
        if (c) {
            *s = v;
        } else {
            s = w;
        };
        *s = v;
        (x, y)
    }
    spec then_reads_else_repoints {
        ensures c ==> result_1 == v && result_2 == b;
        ensures !c ==> result_1 == a && result_2 == v;
    }

    fun then_reads_else_repoints_false(c: bool, a: u64, b: u64, v: u64): (u64, u64) {
        let x = a;
        let y = b;
        let s = &mut x;
        let w = &mut y;
        bump(w, b);
        if (c) {
            *s = v;
        } else {
            s = w;
        };
        *s = v;
        (x, y)
    }
    spec then_reads_else_repoints_false {
        ensures result_2 == v; // error: not on the other path
    }

    fun then_reads_else_repoints_vacuous(c: bool, a: u64, b: u64, v: u64): (u64, u64) {
        let x = a;
        let y = b;
        let s = &mut x;
        let w = &mut y;
        bump(w, b);
        if (c) {
            *s = v;
        } else {
            s = w;
        };
        *s = v;
        (x, y)
    }
    spec then_reads_else_repoints_vacuous {
        ensures false; // error: the paths are not vacuous
    }

    fun loop_local_write(a: u64, n: u64): u64 {
        let x = a;
        let r = &mut x;
        let i = 0;
        while (i < n) {
            *r = i;
            i = i + 1;
        };
        x
    }
    spec loop_local_write {
        pragma unroll = 3;
        requires n <= 3;
        ensures n == 0 ==> result == a;
        ensures n >= 1 ==> result == n - 1;
    }

    fun loop_local_write_false(a: u64, n: u64): u64 {
        let x = a;
        let r = &mut x;
        let i = 0;
        while (i < n) {
            *r = i;
            i = i + 1;
        };
        x
    }
    spec loop_local_write_false {
        pragma unroll = 3;
        requires n <= 3;
        ensures n >= 1 ==> result == n; // error: the last write is n - 1
    }

    fun loop_local_write_vacuous(a: u64, n: u64): u64 {
        let x = a;
        let r = &mut x;
        let i = 0;
        while (i < n) {
            *r = i;
            i = i + 1;
        };
        x
    }
    spec loop_local_write_vacuous {
        pragma unroll = 3;
        requires n <= 3;
        ensures false; // error: the paths are not vacuous
    }

    fun break_skips_repoint(c: bool, n: u64, a: u64, b: u64, v: u64): (u64, u64) {
        let x = a;
        let y = b;
        let s = &mut x;
        let i = 0;
        while (i < n) {
            if (c) {
                break
            };
            s = &mut y;
            i = i + 1;
        };
        *s = v;
        (x, y)
    }
    spec break_skips_repoint {
        pragma unroll = 3;
        requires n <= 3;
        ensures c || n == 0 ==> result_1 == v && result_2 == b;
        ensures !c && n > 0 ==> result_1 == a && result_2 == v;
    }

    fun break_skips_repoint_false(c: bool, n: u64, a: u64, b: u64, v: u64): (u64, u64) {
        let x = a;
        let y = b;
        let s = &mut x;
        let i = 0;
        while (i < n) {
            if (c) {
                break
            };
            s = &mut y;
            i = i + 1;
        };
        *s = v;
        (x, y)
    }
    spec break_skips_repoint_false {
        pragma unroll = 3;
        requires n <= 3;
        ensures result_2 == v; // error: not on the other path
    }

    fun break_skips_repoint_vacuous(c: bool, n: u64, a: u64, b: u64, v: u64): (u64, u64) {
        let x = a;
        let y = b;
        let s = &mut x;
        let i = 0;
        while (i < n) {
            if (c) {
                break
            };
            s = &mut y;
            i = i + 1;
        };
        *s = v;
        (x, y)
    }
    spec break_skips_repoint_vacuous {
        pragma unroll = 3;
        requires n <= 3;
        ensures false; // error: the paths are not vacuous
    }
}
