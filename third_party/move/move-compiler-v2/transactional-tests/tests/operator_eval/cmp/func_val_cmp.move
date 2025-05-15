
//# publish
module 0xc0ffee::m {

    struct Work(|u64|u64) has drop;

    public fun test1() {
        let work1 = Work(|x| x + 1);
        let work2 = Work(|x| x + 2);
         assert!(work1 != work2, 0);
    }

     public fun test2() {
        let work1 = Work(|x| x + 1);
        let work2 = Work(|x| x + 2);
         assert!(&work1 != &work2, 0);
    }

    public fun test3() {
        let work1 = Work(|x| x + 1);
        let work2 = Work(|x| x + 1);
         assert!(work1 != work2, 0);
    }

    public fun test4() {
        let work1 = Work(|x| x + 1);
        let work2 = Work(|x| x + 1);
         assert!(&work1 != &work2, 0);
    }

     public fun test5() {
        let work1 = Work(|x| x + 1);
        let work2 = Work(|x| x + 1);
         assert!(work1(1) == work2(1), 0);
    }

     public fun test6() {
        let work1 = Work(|x| x + 1);
        let work2 = Work(|x| x + 1);
         assert!(work1(1) != work2(2), 0);
    }
}

//# run 0xc0ffee::m::test1

//# run 0xc0ffee::m::test2

//# run 0xc0ffee::m::test3

//# run 0xc0ffee::m::test4

//# run 0xc0ffee::m::test5

//# run 0xc0ffee::m::test6
