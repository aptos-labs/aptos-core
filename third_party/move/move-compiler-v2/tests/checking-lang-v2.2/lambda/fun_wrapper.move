module 0x66::test {
    struct Work(|u64|u64) has drop;

    fun take_work(_work: Work) {}

    fun t1():bool {
        let work = Work(|x| x + 1);
        work == (|x| x + 2)
    }

    fun t2() {
        take_work(|x| x + 1)
    }
}
