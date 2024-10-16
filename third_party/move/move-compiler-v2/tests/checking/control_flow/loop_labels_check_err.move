module 0x815::test {
    fun undefined_label() {
        break 'outer;
        'outer: loop {
            break 'inner
        }
    }

    fun duplicate_label() {
        'l1: loop {};
        'l1: loop 'l1: loop {};
        'l1: loop {}
    }
}
