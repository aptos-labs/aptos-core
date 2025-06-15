
module 0x01::test {
    struct Work(|u64|u64) has copy, drop;

    fun eq1():Work {
        let work1 = Work(|x| x + 1);
        let work2 = Work(|x| x + 2);
        if (work1 == work2)
            work1
        else
            work2
    }

    fun eq2():Work {
        let work1 = Work(|x| x + 1);
        let work2 = Work(|x| x + 2);
        if (&work1 == &work2)
            work1
        else
            work2
    }

    fun eq3():Work {
        let work1 = Work(|x| x + 1);
        let work2 = Work(|x| x + 2);
        if (work1(2) == work2(1))
            work1
        else
            work2
    }

    fun eq4():Work {
        let work1 = Work(|x| x + 1);
        let work2 = Work(|x| x + 2);
        if (&work1(2) == &work2(1))
            work1
        else
            work2
    }
}
