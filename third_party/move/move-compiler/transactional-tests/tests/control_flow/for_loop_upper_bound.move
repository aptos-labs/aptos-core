//# publish
module 0x42::m {

    fun foo(x: &mut u64): u64 {
        *x = *x + 1;
        10
    }

    fun main(): u64 {
        let x = 0;
        for (i in 0..foo(&mut x)) {};
        x
    }
}

//# run 0x42::m::main
