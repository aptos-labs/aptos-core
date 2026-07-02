// also_include_for: prophecy
module 0x42::prophecy_opaque {
    fun inc(x: &mut u64) {
        *x = *x + 1;
    }
    spec inc {
        pragma opaque;
        ensures x == old(x) + 1;
    }

    fun client(): u64 {
        let a = 5;
        inc(&mut a);
        a
    }
    spec client {
        ensures result == 6;
    }
}
