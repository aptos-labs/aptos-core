// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A precondition `skip || requires_of<f>(x)` of a closure taking `&mut` is not
// established at the call, and `call_disjunct` is a parked gap. `call_plain` is
// the control: the precondition `requires_of<f>(x)` alone holds at its call.
module 0x42::requires_of_disjunct {
    fun modify(skip: bool, x: &mut u64, f: |&mut u64| u64): u64 {
        if (skip) return 0;
        f(x)
    }
    spec modify {
        pragma opaque;
        pragma verify = false;
        requires skip || requires_of<f>(x);
        aborts_if !skip && aborts_of<f>(x);
    }

    fun modify_plain(x: &mut u64, f: |&mut u64| u64): u64 {
        f(x)
    }
    spec modify_plain {
        pragma opaque;
        pragma verify = false;
        requires requires_of<f>(x);
        aborts_if aborts_of<f>(x);
    }

    fun call_disjunct(skip: bool): u64 {
        let x = 1;
        modify(skip, &mut x, |v| { let o = *v; *v = 50; o })
    }

    fun call_plain(): u64 {
        let x = 1;
        modify_plain(&mut x, |v| { let o = *v; *v = 50; o })
    }
}
