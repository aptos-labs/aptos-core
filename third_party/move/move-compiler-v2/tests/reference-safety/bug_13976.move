// TODO(#13976): after fix, rename file to reflect issue (`bug_nnn.move` to `bug_nnn_<issue>.move`)
module 0x815::m {
    fun lifted_lambda(ap: &mut u64, z: u64): u64 {
        *ap = *ap + 1;
        z * *ap
    }

    entry fun foo2(): u64 {
        let a = 2;
        let z = 3;
        lifted_lambda(&mut a,
            lifted_lambda(&mut a, z))
    }
}
