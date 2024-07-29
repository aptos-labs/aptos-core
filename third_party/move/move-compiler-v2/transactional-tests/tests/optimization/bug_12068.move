//# publish
module 0x32::m {
    public fun main() {
        let x = 0;
        while (true) {
            x = x + 1;
            break
        };
        assert!(x == 1, 42);
    }
}

//# run 0x32::m::main
