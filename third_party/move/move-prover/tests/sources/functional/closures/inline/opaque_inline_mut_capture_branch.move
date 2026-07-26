// Test for mut-capturing closures packed under a branch: two packs of the
// same closure shape from different roots merge into one temp at the join,
// exercising the conditional write-back with `IsParent` on `Capture` edges.
module 0x42::opaque_inline_mut_capture_branch {
    inline fun modify(f: |u64|) {
        f(1)
    }
    spec modify {
        pragma opaque;
        requires !aborts_of<f>(1);
        aborts_if false;
        ensures ensures_of<f>(1);
    }

    /// Test: conditional capture of one of two locals.
    fun test_branch_capture(c: bool): (u64, u64) {
        let x = 10;
        let z = 20;
        if (c) {
            let rx = &mut x;
            modify(|y| *rx = *rx + y spec {
                aborts_if rx + y > MAX_U64;
                ensures rx == old(rx) + y;
            });
        } else {
            let rz = &mut z;
            modify(|y| *rz = *rz + y spec {
                aborts_if rz + y > MAX_U64;
                ensures rz == old(rz) + y;
            });
        };
        (x, z)
    }
    spec test_branch_capture {
        ensures c ==> result_1 == 11 && result_2 == 20;
        ensures !c ==> result_1 == 10 && result_2 == 21;
    }
}
