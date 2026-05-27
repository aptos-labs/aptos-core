module 0x42::loop_invalid {

    fun apply_to_position_and_return<T>(f: |&mut u64, u64| T): T {
        let x = 42;
        let y = 42;
        let position = &mut x;
        f(position, y)
    }

    fun test(
        f: |&mut u64, u64|
    ) {
        apply_to_position_and_return(|position, val| {
            f(position, val);
            true
        });
    }
}
