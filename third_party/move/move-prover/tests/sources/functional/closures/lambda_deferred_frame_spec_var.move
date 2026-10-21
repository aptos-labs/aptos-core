// A closure whose target updates a spec variable cannot be deferred to a callee that leaves
// its frame to the call site: a call-site havoc never covers a spec variable, so the update
// would be dropped and the claims below, which say the variable is unchanged, would verify.
// The closure has to be rejected, directly and inside an opaque helper.
module 0x42::lambda_deferred_frame_spec_var {
    spec module {
        global ctr: num;
    }

    fun call_g(g: |u64| u64 has copy + drop, y: u64): u64 {
        g(y)
    }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    fun bump(y: u64): u64 {
        spec {
            update ctr = ctr + 1;
        };
        y
    }

    fun deferred(): u64 {
        call_g(|z| bump(z), 1)
    }
    spec deferred {
        ensures ctr == old(ctr);
    }

    fun helper(): u64 {
        call_g(|z| bump(z), 1)
    }
    spec helper {
        pragma opaque;
    }

    fun through_helper(): u64 {
        helper()
    }
    spec through_helper {
        ensures ctr == old(ctr);
    }
}
